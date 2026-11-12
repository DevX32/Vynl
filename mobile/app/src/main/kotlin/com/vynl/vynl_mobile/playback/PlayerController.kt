package com.vynl.vynl_mobile.playback

import android.content.ComponentName
import android.content.Context
import androidx.core.content.ContextCompat
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.vynl.vynl_mobile.data.CatalogueRepository
import com.vynl.vynl_mobile.data.CatalogueTrack
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

enum class RepeatMode { Off, All, One }

data class PlaybackState(
    val connected: Boolean = false,
    val track: CatalogueTrack? = null,
    val playing: Boolean = false,
    val positionMs: Long = 0,
    val durationMs: Long = 0,
    val bufferedMs: Long = 0,
    val buffering: Boolean = false,
    val queueIndex: Int = -1,
    val queueSize: Int = 0,
    val shuffle: Boolean = false,
    val repeatMode: RepeatMode = RepeatMode.Off,
    val error: String? = null,
)

/**
 * The playback side of the app. The queue lives here, the audio addresses are
 * resolved by the rust catalogue, and the player itself lives in the service
 * so it keeps playing in the background.
 */
class PlayerController(
    private val context: Context,
    private val repository: CatalogueRepository,
) {
    private val scope = CoroutineScope(Dispatchers.Main.immediate + SupervisorJob())
    private val _state = MutableStateFlow(PlaybackState())
    val state: StateFlow<PlaybackState> = _state.asStateFlow()

    private var controller: MediaController? = null
    private var playlist: List<CatalogueTrack> = emptyList()
    private var queueStart = 0
    private var localFiles: Map<String, String> = emptyMap()
    private val resolved = mutableMapOf<Int, String>()
    private var poll: Job? = null
    private var filling = false

    private val window = 3

    private val listener = object : Player.Listener {
        override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
            if (reason == Player.MEDIA_ITEM_TRANSITION_REASON_PLAYLIST_CHANGED) return
            fillAhead()
        }

        override fun onIsPlayingChanged(isPlaying: Boolean) {
            _state.value = _state.value.copy(playing = isPlaying)
        }

        override fun onPlaybackStateChanged(playbackState: Int) {
            _state.value = _state.value.copy(
                buffering = playbackState == Player.STATE_BUFFERING,
            )
        }

        override fun onShuffleModeEnabledChanged(shuffleModeEnabled: Boolean) {
            _state.value = _state.value.copy(shuffle = shuffleModeEnabled)
        }

        override fun onRepeatModeChanged(repeatMode: Int) {
            _state.value = _state.value.copy(
                repeatMode = when (repeatMode) {
                    Player.REPEAT_MODE_ALL -> RepeatMode.All
                    Player.REPEAT_MODE_ONE -> RepeatMode.One
                    else -> RepeatMode.Off
                },
            )
        }

        override fun onPlayerError(error: PlaybackException) {
            _state.value = _state.value.copy(error = "could not play that track")
        }
    }

    suspend fun connect() {
        if (controller != null) return
        val token = SessionToken(context, ComponentName(context, PlaybackService::class.java))
        val future = MediaController.Builder(context, token).buildAsync()
        val built = suspendCancellableCoroutine { continuation ->
            future.addListener(
                {
                    try {
                        continuation.resume(future.get())
                    } catch (error: Exception) {
                        continuation.resumeWithException(error)
                    }
                },
                ContextCompat.getMainExecutor(context),
            )
        }
        controller = built
        built.addListener(listener)
        _state.value = _state.value.copy(connected = true)
        poll = scope.launch {
            while (true) {
                update()
                delay(400)
            }
        }
    }

    fun play(
        tracks: List<CatalogueTrack>,
        startIndex: Int,
        files: Map<String, String> = emptyMap(),
    ) {
        if (tracks.isEmpty()) return
        scope.launch {
            playlist = tracks
            queueStart = startIndex.coerceIn(0, tracks.lastIndex)
            localFiles = files
            resolved.clear()
            val items = resolve(queueStart)
            if (items.isEmpty()) {
                _state.value = _state.value.copy(error = "could not play that track")
                return@launch
            }
            controller?.setMediaItems(items, 0, 0L)
            controller?.prepare()
            controller?.play()
        }
    }

    fun toggle() {
        val player = controller ?: return
        if (player.isPlaying) player.pause() else player.play()
    }

    fun next() {
        controller?.seekToNextMediaItem()
    }

    fun previous() {
        val player = controller ?: return
        if (player.currentPosition > 4000) {
            player.seekTo(0)
            return
        }
        player.seekToPreviousMediaItem()
    }

    fun seekTo(positionMs: Long) {
        controller?.seekTo(positionMs)
    }

    private suspend fun resolve(from: Int): List<MediaItem> = coroutineScope {
        val end = minOf(playlist.size, from + window)
        if (from >= end) return@coroutineScope emptyList()
        val items = (from until end).map { index ->
            async {
                val track = playlist[index]
                val local = localFiles[track.id]
                if (local != null) {
                    mediaItem(track, android.net.Uri.fromFile(java.io.File(local)).toString())
                } else {
                    val url = resolved[index] ?: runCatching {
                        repository.stream(track.id).url
                    }.getOrNull()?.also { resolved[index] = it }
                    url?.let { mediaItem(track, it) }
                }
            }
        }.awaitAll()
        items.filterNotNull()
    }

    private fun fillAhead() {
        if (filling) return
        filling = true
        scope.launch {
            try {
                val player = controller ?: return@launch
                val have = player.mediaItemCount
                val absolute = queueStart + player.currentMediaItemIndex
                val wanted = absolute + window + 1
                if (wanted > queueStart + have) {
                    val next = resolve(queueStart + have)
                    if (next.isNotEmpty()) {
                        player.addMediaItems(have, next)
                    }
                }
            } finally {
                filling = false
            }
        }
    }

    private fun mediaItem(track: CatalogueTrack, url: String): MediaItem {
        val metadata = MediaMetadata.Builder()
            .setTitle(track.title)
            .setArtist(track.artist)
            .setAlbumTitle(track.album)
            .setArtworkUri(track.cover?.let { android.net.Uri.parse(it) })
            .setIsBrowsable(false)
            .setIsPlayable(true)
            .build()
        return MediaItem.Builder()
            .setMediaId(track.id)
            .setUri(url)
            .setMediaMetadata(metadata)
            .build()
    }

    private fun update() {
        val player = controller ?: return
        val index = player.currentMediaItemIndex
        val absolute = queueStart + index
        val track = playlist.getOrNull(absolute)
        val duration = player.duration.takeIf { it > 0 } ?: 0L
        _state.value = _state.value.copy(
            track = track,
            positionMs = player.currentPosition.coerceAtLeast(0),
            durationMs = duration,
            bufferedMs = player.bufferedPosition.coerceAtLeast(0),
            buffering = player.playbackState == Player.STATE_BUFFERING,
            playing = player.isPlaying,
            queueIndex = absolute,
            queueSize = playlist.size,
            shuffle = player.shuffleModeEnabled,
            repeatMode = when (player.repeatMode) {
                Player.REPEAT_MODE_ALL -> RepeatMode.All
                Player.REPEAT_MODE_ONE -> RepeatMode.One
                else -> RepeatMode.Off
            },
        )
    }

    /** The current queue, in play order. */
    fun queueTracks(): List<CatalogueTrack> = playlist.toList()

    /** Moves the queue item at [from] to [to]. */
    fun moveInQueue(from: Int, to: Int) {
        if (from == to || from !in playlist.indices || to !in playlist.indices) return
        val oldStart = queueStart
        val mutable = playlist.toMutableList()
        val item = mutable.removeAt(from)
        mutable.add(to, item)
        playlist = mutable
        queueStart = run {
            var mapped = oldStart
            if (from == oldStart) {
                mapped = to
            } else {
                if (from < oldStart) mapped -= 1
                if (to <= mapped) mapped += 1
            }
            mapped
        }
        val player = controller ?: return
        if (player.mediaItemCount > 0) {
            val loadedFrom = from - oldStart
            val loadedTo = to - oldStart
            if (loadedFrom in 0 until player.mediaItemCount && loadedTo in 0 until player.mediaItemCount) {
                player.moveMediaItem(loadedFrom, loadedTo)
            }
        }
        _state.value = _state.value.copy(
            queueIndex = queueStart + (player.currentMediaItemIndex.coerceAtLeast(0)),
            queueSize = playlist.size,
        )
    }

    /** Shuffles the queue, starts at 0. */
    fun shuffle() {
        if (playlist.isEmpty()) return
        playlist = playlist.shuffled()
        queueStart = 0
        // Resolve from 0 is deferred: callers pick the starting track.
    }

    /** Toggles shuffle mode. */
    fun toggleShuffle() {
        val controller = controller ?: return
        controller.shuffleModeEnabled = !controller.shuffleModeEnabled
    }

    /** Cycles repeat mode: off -> all -> one -> off. */
    fun toggleRepeat() {
        val controller = controller ?: return
        controller.repeatMode = when (controller.repeatMode) {
            Player.REPEAT_MODE_OFF -> Player.REPEAT_MODE_ALL
            Player.REPEAT_MODE_ALL -> Player.REPEAT_MODE_ONE
            else -> Player.REPEAT_MODE_OFF
        }
    }

    fun playShuffled() {
        if (playlist.isEmpty()) return
        playlist = playlist.shuffled()
        queueStart = 0
        scope.launch {
            val items = resolve(0)
            if (items.isEmpty()) {
                _state.value = _state.value.copy(error = "could not play that track")
                return@launch
            }
            controller?.setMediaItems(items, 0, 0L)
            controller?.prepare()
            controller?.play()
        }
    }

    fun playAt(index: Int) {
        if (index !in playlist.indices) return
        scope.launch {
            queueStart = index
            val items = resolve(index)
            if (items.isEmpty()) {
                _state.value = _state.value.copy(error = "could not play that track")
                return@launch
            }
            controller?.setMediaItems(items, 0, 0L)
            controller?.prepare()
            controller?.play()
        }
    }

    fun release() {
        poll?.cancel()
        controller?.removeListener(listener)
        controller?.release()
        controller = null
    }
}
