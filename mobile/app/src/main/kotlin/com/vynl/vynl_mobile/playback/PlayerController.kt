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
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
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
    val queueVersion: Int = 0,
)

class PlayerController(
    private val context: Context,
    private val repository: CatalogueRepository,
) {
    private val failureHandler = kotlinx.coroutines.CoroutineExceptionHandler { _, error ->
        android.util.Log.e("Vynl", "player task failed", error)
        _state.value = _state.value.copy(error = "Something went wrong — try again")
    }
    private val scope = CoroutineScope(Dispatchers.Main.immediate + SupervisorJob() + failureHandler)
    private val _state = MutableStateFlow(PlaybackState())
    val state: StateFlow<PlaybackState> = _state.asStateFlow()

    private var controller: MediaController? = null
    private var playlist: List<CatalogueTrack> = emptyList()
    private var queueStart = 0
    private var localFiles: Map<String, String> = emptyMap()
    private val resolved = mutableMapOf<Int, String>()
    private val resolvedAt = mutableMapOf<Int, Long>()
    private val resolvedTtlMs = 5L * 60L * 60L * 1000L

    private fun cachedUrl(index: Int): String? {
        val at = resolvedAt[index] ?: return null
        if (System.currentTimeMillis() - at > resolvedTtlMs) {
            resolved.remove(index)
            resolvedAt.remove(index)
            return null
        }
        return resolved[index]
    }

    private fun cacheUrl(index: Int, url: String) {
        resolved[index] = url
        resolvedAt[index] = System.currentTimeMillis()
    }

    private fun clearResolved() {
        resolved.clear()
        resolvedAt.clear()
    }
    private var poll: Job? = null
    private var filling = false
    private val connectMutex = Mutex()
    private var queueVersion = 0
    private var listEpoch = 0

    private val window = 3

    private val listener = object : Player.Listener {
        override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
            if (reason == Player.MEDIA_ITEM_TRANSITION_REASON_PLAYLIST_CHANGED) return
            fillAhead()
        }

        override fun onIsPlayingChanged(isPlaying: Boolean) {
            _state.value = _state.value.copy(playing = isPlaying)
            if (isPlaying) {
                lastRetryIndex = -1
            }
        }

        override fun onPlaybackStateChanged(playbackState: Int) {
            _state.value = _state.value.copy(
                buffering = playbackState == Player.STATE_BUFFERING,
            )
        }

        override fun onShuffleModeEnabledChanged(shuffleModeEnabled: Boolean) {
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
            val player = controller ?: return
            val absolute = queueStart + player.currentMediaItemIndex
            resolved.remove(absolute)
            resolvedAt.remove(absolute)
            val failedTrack = playlist.getOrNull(absolute)
            if (failedTrack != null && absolute != lastRetryIndex) {
                lastRetryIndex = absolute
                scope.launch {
                    if (player.currentMediaItemIndex != absolute - queueStart) return@launch
                    val item = playlist.getOrNull(absolute) ?: return@launch
                    val local = localFiles[item.id]
                    val url = local?.let { android.net.Uri.fromFile(java.io.File(it)).toString() }
                        ?: runCatching { repository.stream(item.id).url }.getOrNull()
                        ?.takeIf { it.isNotBlank() }
                    if (url == null) {
                        skipErroredItem(player)
                        return@launch
                    }
                    cacheUrl(absolute, url)
                    if (player.currentMediaItemIndex != absolute - queueStart) return@launch
                    val position = player.currentPosition
                    val replacement = mediaItem(item, url)
                    player.replaceMediaItem(player.currentMediaItemIndex, replacement)
                    player.prepare()
                    player.seekTo(position)
                    player.play()
                }
                return
            }
            skipErroredItem(player)
        }
    }

    private var lastRetryIndex = -1

    private fun skipErroredItem(player: androidx.media3.session.MediaController) {
        if (player.hasNextMediaItem()) {
            _state.value = _state.value.copy(
                error = "could not play that track — skipping to the next one",
            )
            player.seekToNextMediaItem()
            player.prepare()
            player.play()
        } else {
            _state.value = _state.value.copy(
                error = "that track can't be played — nothing else follows it",
            )
        }
    }

    suspend fun connect() {
        connectMutex.withLock {
            if (controller != null) return@withLock
            try {
                connectOnce()
            } catch (error: Exception) {
                _state.value = _state.value.copy(connected = false, error = "could not connect to the player")
            }
        }
    }

    private suspend fun connectOnce() {
        if (controller != null) return
        val token = SessionToken(context, ComponentName(context, PlaybackService::class.java))
        val controllerListener = object : MediaController.Listener {
            override fun onDisconnected(controller: MediaController) {
                poll?.cancel()
                poll = null
                if (this@PlayerController.controller === controller) {
                    this@PlayerController.controller = null
                }
                _state.value = _state.value.copy(
                    connected = false,
                    playing = false,
                    buffering = false,
                    track = null,
                )
            }
        }
        val future = MediaController.Builder(context, token)
            .setListener(controllerListener)
            .buildAsync()
        val built = suspendCancellableCoroutine { continuation ->
            future.addListener(
                {
                    if (!future.isCancelled && continuation.isActive) {
                        try {
                            continuation.resume(future.get())
                        } catch (error: Exception) {
                            runCatching { continuation.resumeWithException(error) }
                        }
                    }
                },
                ContextCompat.getMainExecutor(context),
            )
            continuation.invokeOnCancellation { future.cancel(true) }
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
            try {
                connect()
            } catch (_: Exception) {
                return@launch
            }
            playlist = tracks
            queueStart = startIndex.coerceIn(0, tracks.lastIndex)
            localFiles = files
            clearResolved()
            listEpoch += 1
            bumpQueueVersion()
            val expectedEpoch = listEpoch
            val c = controller ?: return@launch
            val items = resolve(queueStart)
            if (expectedEpoch != listEpoch) return@launch
            if (items.isEmpty()) {
                _state.value = _state.value.copy(error = "could not play that track")
                return@launch
            }
            c.setMediaItems(items, 0, 0L)
            c.prepare()
            c.play()
        }
    }

    fun toggle() {
        val player = controller ?: return
        if (player.isPlaying) {
            player.pause()
            return
        }
        if (player.playerError != null) {
            if (player.hasNextMediaItem()) {
                player.seekToNextMediaItem()
                player.prepare()
                player.play()
            }
            return
        }
        if (player.playbackState == Player.STATE_ENDED) {
            player.prepare()
        }
        player.play()
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
        if (player.currentMediaItemIndex <= 0 && queueStart > 0) {
            playAt(queueStart - 1)
            return
        }
        player.seekToPreviousMediaItem()
    }

    fun seekTo(positionMs: Long) {
        controller?.seekTo(positionMs)
    }

    private fun bumpQueueVersion() {
        queueVersion += 1
        _state.value = _state.value.copy(queueVersion = queueVersion)
    }

    fun clearError() {
        _state.value = _state.value.copy(error = null)
    }

    private suspend fun resolve(from: Int): List<MediaItem> = coroutineScope {
        val snapshot = playlist
        val locals = localFiles
        val snapStart = queueStart
        val end = minOf(snapshot.size, from + window)
        if (from >= end) return@coroutineScope emptyList()
        val results = (from until end).map { index ->
            async {
                val track = snapshot[index]
                val local = locals[track.id]
                if (local != null) {
                    mediaItem(track, android.net.Uri.fromFile(java.io.File(local)).toString())
                } else {
                    val url = cachedUrl(index) ?: runCatching {
                        repository.stream(track.id).url
                    }.getOrNull()?.takeIf { it.isNotBlank() }?.also { cacheUrl(index, it) }
                    url?.let { mediaItem(track, it) }
                }
            }
        }.awaitAll()
        val failed = (from until end).filter { results[it - from] == null }
        if (failed.isNotEmpty()) {
            if (playlist === snapshot) {
                val removedBeforeQueueStart = failed.count { it < snapStart }
                playlist = playlist.filterIndexed { idx, _ -> idx !in failed }
                if (removedBeforeQueueStart > 0) {
                    queueStart = (queueStart - removedBeforeQueueStart).coerceAtLeast(0)
                }
                clearResolved()
                _state.value = _state.value.copy(
                    error = "${failed.size} track(s) can't be played from the catalog",
                )
                bumpQueueVersion()
            } else {
                clearResolved()
            }
        }
        results.filterNotNull()
    }

    private fun fillAhead() {
        if (filling) return
        filling = true
        scope.launch {
            try {
                val expectedEpoch = listEpoch
                val player = controller ?: return@launch
                val have = player.mediaItemCount
                val absolute = queueStart + player.currentMediaItemIndex
                val wanted = absolute + window + 1
                if (wanted > queueStart + have) {
                    val next = resolve(queueStart + have)
                    if (expectedEpoch != listEpoch) {
                        return@launch
                    }
                    val currentPlayer = controller ?: return@launch
                    val currentHave = currentPlayer.mediaItemCount.coerceAtLeast(0)
                    if (currentHave < have || next.isEmpty()) {
                        return@launch
                    }
                    try {
                        currentPlayer.addMediaItems(have, next)
                    } catch (_: IndexOutOfBoundsException) {
                    }
                }
            } finally {
                filling = false
            }
        }
    }

    private fun artworkUri(cover: String): android.net.Uri {
        val trimmed = cover.trim()
        return if (trimmed.startsWith("content://")
            || trimmed.startsWith("file://")
            || trimmed.startsWith("http://")
            || trimmed.startsWith("https://")
            || trimmed.startsWith("android.resource://")
        ) {
            android.net.Uri.parse(trimmed)
        } else {
            android.net.Uri.fromFile(java.io.File(trimmed))
        }
    }

    private fun mediaItem(track: CatalogueTrack, url: String): MediaItem {
        val metadata = MediaMetadata.Builder()
            .setTitle(track.title)
            .setArtist(track.artist)
            .setAlbumTitle(track.album)
            .setArtworkUri(track.cover?.let { artworkUri(it) })
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
        val track = playlist.getOrNull(absolute) ?: _state.value.track
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
            repeatMode = when (player.repeatMode) {
                Player.REPEAT_MODE_ALL -> RepeatMode.All
                Player.REPEAT_MODE_ONE -> RepeatMode.One
                else -> RepeatMode.Off
            },
        )
    }

    fun queueTracks(): List<CatalogueTrack> = playlist.toList()

    fun moveInQueue(from: Int, to: Int) {
        if (from == to || from !in playlist.indices || to !in playlist.indices) return
        val oldPosition = controller?.currentPosition ?: 0L
        val play = controller?.isPlaying != false
        val mutable = playlist.toMutableList()
        val item = mutable.removeAt(from)
        mutable.add(to, item)
        playlist = mutable
        listEpoch += 1
        bumpQueueVersion()
        val playing = controller?.let { queueStart + it.currentMediaItemIndex } ?: from
        val newPlaying = when {
            playing == from -> to
            from < playing && playing <= to -> playing - 1
            to <= playing && playing < from -> playing + 1
            else -> playing
        }
        queueStart = newPlaying
        clearResolved()
        val player = controller ?: run {
            return
        }
        if (player.mediaItemCount == 0) {
            return
        }
        val expectedEpoch = listEpoch
        scope.launch {
            val items = resolve(newPlaying)
            if (expectedEpoch != listEpoch) return@launch
            if (items.isEmpty()) {
                return@launch
            }
            player.setMediaItems(items, 0, oldPosition)
            player.prepare()
            if (play) {
                player.play()
            }
        }
        _state.value = _state.value.copy(
            queueIndex = newPlaying,
            queueSize = playlist.size,
        )
    }

    private var canonicalPlaylist: List<CatalogueTrack>? = null

    fun toggleShuffle() {
        val player = controller ?: return
        val want = !_state.value.shuffle
        if (playlist.isEmpty()) {
            player.shuffleModeEnabled = false
            _state.value = _state.value.copy(shuffle = false)
            return
        }
        val playing = (queueStart + player.currentMediaItemIndex).coerceIn(0, playlist.lastIndex)
        val currentTrack = playlist.getOrNull(playing)
        if (want) {
            canonicalPlaylist = playlist
            val head = playlist.subList(0, playing + 1)
            val tail = playlist.subList(playing + 1, playlist.size).shuffled()
            playlist = head + tail
            queueStart = playing
        } else {
            val restored = canonicalPlaylist ?: playlist
            playlist = restored
            canonicalPlaylist = null
            queueStart = currentTrack
                ?.let { track -> restored.indexOfFirst { it.id == track.id } }
                ?.takeIf { it >= 0 }
                ?: playing.coerceAtMost(restored.lastIndex)
        }
        listEpoch += 1
        bumpQueueVersion()
        clearResolved()
        val oldPosition = player.currentPosition
        val wasPlaying = player.isPlaying
        val expectedEpoch = listEpoch
        scope.launch {
            try {
                connect()
                val items = resolve(queueStart)
                if (expectedEpoch != listEpoch) return@launch
                if (items.isEmpty()) {
                    _state.value = _state.value.copy(error = "could not play that track")
                    return@launch
                }
                player.setMediaItems(items, 0, oldPosition)
                player.prepare()
                if (wasPlaying) {
                    player.play()
                }
            } catch (_: Exception) {
            }
        }
        player.shuffleModeEnabled = false
        _state.value = _state.value.copy(shuffle = want)
    }

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
        canonicalPlaylist = playlist
        playlist = playlist.shuffled()
        queueStart = 0
        listEpoch += 1
        bumpQueueVersion()
        val expectedEpoch = listEpoch
        clearResolved()
        scope.launch {
            try {
                connect()
            } catch (_: Exception) {
                return@launch
            }
            val items = resolve(0)
            if (expectedEpoch != listEpoch) return@launch
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
        listEpoch += 1
        bumpQueueVersion()
        val expectedEpoch = listEpoch
        scope.launch {
            queueStart = index
            try {
                connect()
            } catch (_: Exception) {
                return@launch
            }
            val items = resolve(index)
            if (expectedEpoch != listEpoch) return@launch
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
        scope.cancel()
        controller?.removeListener(listener)
        controller?.release()
        controller = null
    }
}
