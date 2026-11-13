package com.vynl.vynl_mobile.ui

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import androidx.compose.ui.graphics.toArgb
import com.vynl.vynl_mobile.data.SettingsStore
import com.vynl.vynl_mobile.data.CatalogueAlbumDetail
import com.vynl.vynl_mobile.data.CatalogueArtistDetail
import com.vynl.vynl_mobile.data.CataloguePlaylistDetail
import com.vynl.vynl_mobile.data.CatalogueRepository
import com.vynl.vynl_mobile.data.CatalogueResults
import com.vynl.vynl_mobile.data.CatalogueTrack
import com.vynl.vynl_mobile.data.Downloader
import com.vynl.vynl_mobile.data.LocalPlaylist
import com.vynl.vynl_mobile.data.Lyrics
import com.vynl.vynl_mobile.data.LyricsStatus
import com.vynl.vynl_mobile.data.LocalTrack
import com.vynl.vynl_mobile.data.Store
import com.vynl.vynl_mobile.data.randomNickname
import com.vynl.vynl_mobile.data.resolveDisplayName
import com.vynl.vynl_mobile.playback.PlayerController
import com.vynl.vynl_mobile.remote.Desktop
import com.vynl.vynl_mobile.ui.theme.Accent
import com.vynl.vynl_mobile.update.AppUpdate
import com.vynl.vynl_mobile.update.Update
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class AppViewModel(application: Application) : AndroidViewModel(application) {
    private val repository = CatalogueRepository()
    private val store = Store(application)
    private val downloader = Downloader(application, repository)
    val player = PlayerController(application, repository)

    var tab by mutableStateOf(0)
        private set

    private val _mix = MutableStateFlow<List<CatalogueTrack>>(emptyList())
    val mix: StateFlow<List<CatalogueTrack>> = _mix.asStateFlow()

    var mixLoading by mutableStateOf(false)
        private set

    var mixError by mutableStateOf<String?>(null)
        private set

    private val _results = MutableStateFlow<CatalogueResults?>(null)
    val results: StateFlow<CatalogueResults?> = _results.asStateFlow()

    var browseQuery by mutableStateOf("")
        private set

    var browseSearching by mutableStateOf(false)
        private set

    var browseError by mutableStateOf<String?>(null)
        private set

    var detailTitle by mutableStateOf<String?>(null)
        private set

    var detailSubtitle by mutableStateOf<String?>(null)
        private set

    var detailCover by mutableStateOf<String?>(null)
        private set

    var detailTracks by mutableStateOf<List<CatalogueTrack>>(emptyList())
        private set

    var detailLoading by mutableStateOf(false)
        private set

    private val _library = MutableStateFlow<List<LocalTrack>>(emptyList())
    val library: StateFlow<List<LocalTrack>> = _library.asStateFlow()

    private val _playlists = MutableStateFlow<List<LocalPlaylist>>(emptyList())
    val playlists: StateFlow<List<LocalPlaylist>> = _playlists.asStateFlow()

    val playlistCovers: MutableStateFlow<Map<String, Pair<String?, String?>>> =
        MutableStateFlow(emptyMap())

    private val _playlistTracks = MutableStateFlow<List<LocalTrack>>(emptyList())
    val playlistTracks: StateFlow<List<LocalTrack>> = _playlistTracks.asStateFlow()

    var actionsTrack by mutableStateOf<CatalogueTrack?>(null)
        private set

    var playlistPickerFor by mutableStateOf<CatalogueTrack?>(null)
        private set

    var notice by mutableStateOf<String?>(null)
        private set

    var openPlaylistId by mutableStateOf<String?>(null)
        private set

    var lyricsOpen by mutableStateOf(false)
        private set

    private val _lyrics = MutableStateFlow<Lyrics?>(null)
    val lyrics: StateFlow<Lyrics?> = _lyrics.asStateFlow()

    var lyricsStatus by mutableStateOf(LyricsStatus.Loading)
        private set

    var lyricsAvailable by mutableStateOf(false)
        private set

    private var lyricsTrackId: String? = null

    private val settings = SettingsStore(application)
    private val updater = AppUpdate(application)
    val desktop = Desktop(application)

    var displayName by mutableStateOf(settings.displayName)
        private set

    var nickname by mutableStateOf(settings.nickname.ifEmpty { randomNickname() })
        private set

    val greetingName: String
        get() = resolveDisplayName(displayName, nickname)

    var onboarded by mutableStateOf(settings.onboarded)
        private set

    fun completeOnboarding(name: String) {
        val trimmed = name.trim().take(32)
        settings.displayName = trimmed
        settings.onboarded = true
        displayName = trimmed
        onboarded = true
    }

    private val _update = MutableStateFlow<Update?>(null)
    val update: StateFlow<Update?> = _update.asStateFlow()

    var updateBusy by mutableStateOf(false)
        private set

    var updateProgress by mutableStateOf(0f)
        private set

    var updateDownloaded by mutableStateOf(false)
        private set

    private var updateFile: java.io.File? = null

    private val _installedVersion = MutableStateFlow("")
    val installedVersion: StateFlow<String> = _installedVersion.asStateFlow()

    private val _downloadedSize = MutableStateFlow("0 MB")
    val downloadedSize: StateFlow<String> = _downloadedSize.asStateFlow()

    var pairing by mutableStateOf(false)
        private set

    var syncing by mutableStateOf(false)
        private set
    var syncStage by mutableStateOf("Syncing library")
        private set
    var syncProgressState by mutableStateOf(0f)
        private set
    private val _syncProgress = MutableStateFlow(0f)
    val syncProgress: StateFlow<Float> = _syncProgress.asStateFlow()

    init {
        viewModelScope.launch { player.connect() }
        if (settings.accentSet) {
            Accent.set(androidx.compose.ui.graphics.Color(settings.accent))
        }
        if (settings.nickname.isEmpty()) {
            settings.nickname = nickname
        }
        val (name, build) = updater.installedVersion()
        _installedVersion.value = name.ifEmpty { "$build" }
        measureDownloads()
        loadMix()
        refreshStore()
    }

    suspend fun pairDesktop(
        address: String,
        pin: String,
        alts: List<String> = emptyList(),
        autoSync: Boolean = true,
    ): String? {
        if (pairing) return "Already pairing"
        pairing = true
        return try {
            val candidates = (listOf(address) + alts)
                .map { it.trim().removePrefix("http://").removePrefix("https://").removeSuffix("/") }
                .filter { it.isNotEmpty() }
                .distinct()
            var failure: String? = null
            for (candidate in candidates) {
                desktop.address = candidate
                desktop.token = ""
                if (desktop.pair(pin)) {
                    notice = "Paired with the desktop"
                    if (autoSync) syncLibrary()
                    return null
                }
                failure = "That pin was not accepted"
            }
            failure ?: "No desktop address to try"
        } catch (error: Exception) {
            friendly(error)
        } finally {
            pairing = false
        }
    }

    fun pairDesktop(address: String, pin: String) {
        viewModelScope.launch {
            pairDesktop(address, pin, autoSync = true)?.let { notice = it }
        }
    }

    fun unpairDesktop() {
        desktop.address = ""
        desktop.token = ""
        notice = "The desktop is forgotten"
    }

    fun syncLibrary() {
        if (!desktop.paired) {
            notice = "Pair with the desktop first"
            return
        }
        if (syncing) return
        viewModelScope.launch {
            syncing = true
            notice = null
            syncStage = "Fetching catalog"
            syncProgressState = 0f
            try {
                val catalog = desktop.catalog()
                val manifest = desktop.syncManifest().associateBy { it.id }
                val playlists = desktop.playlists()
                val existing = withContext(Dispatchers.IO) {
                    store.tracks().associateBy { it.id }
                }
                val localAudioPaths = withContext(Dispatchers.IO) {
                    store.localAudioPaths().filterValues { path -> java.io.File(path).exists() }
                }

                val toFetch = catalog.filter { track ->
                    val localMtime = existing[track.id]?.mtime
                    val missing = !localAudioPaths.containsKey(track.id)
                    val changed = track.mtime != null && track.mtime > 0 && localMtime != null && localMtime != track.mtime
                    missing || changed
                }

                syncStage = "Updating tracks"
                var updated = 0
                for (track in catalog) {
                    val previous = existing[track.id]
                    val lyrics = if (previous != null && previous.mtime == track.mtime && !previous.lyrics.isNullOrBlank()) {
                        previous.lyrics
                    } else {
                        desktop.syncedLyrics(track.id)?.text
                    }
                    val ext = track.ext.ifBlank { manifest[track.id]?.ext ?: "m4a" }
                    val coverValue = track.cover
                    val unchanged = previous != null &&
                        previous.title == track.title &&
                        previous.artist == track.artist &&
                        previous.album == track.album &&
                        previous.duration == track.duration &&
                        previous.cover == coverValue &&
                        previous.ext.equals(ext, ignoreCase = true) &&
                        previous.mtime == track.mtime &&
                        previous.lyrics == lyrics
                    if (!unchanged) {
                        store.addTrack(
                            CatalogueTrack(
                                id = track.id,
                                title = track.title,
                                artist = track.artist,
                                album = track.album,
                                duration = track.duration,
                                cover = coverValue,
                                explicit = false,
                            ),
                            ext = ext,
                            mtime = manifest[track.id]?.mtime?.takeIf { it > 0 },
                            lyrics = lyrics,
                        )
                    }
                    updated += 1
                    syncProgressState = if (catalog.isNotEmpty()) updated / catalog.size.toFloat() else 0f
                    _syncProgress.value = syncProgressState
                }

                syncStage = if (toFetch.isEmpty()) "Finishing up" else "Downloading 0 of ${toFetch.size}"
                var done = 0
                var failed = 0
                toFetch.forEach { track ->
                    try {
                        val m = manifest[track.id]
                        if (m == null) {
                            failed += 1
                            return@forEach
                        }
                        val ext = m.ext.ifBlank { track.ext.ifBlank { "m4a" } }
                        val audioDest = store.audioFile(track.id, ext)
                        val audio = desktop.downloadAudio(track.id, audioDest, onProgress = { progress ->
                            val ratio = if (toFetch.isNotEmpty()) {
                                (done + progress) / toFetch.size.toFloat()
                            } else 0f
                            _syncProgress.value = ratio
                            syncProgressState = ratio
                        })
                        if (audio != null && audio > 0L) {
                            store.setAudioPath(track.id, audioDest.absolutePath, m.mtime)
                        } else if (audioDest.exists()) {
                            audioDest.delete()
                        }
                        done += 1
                        val ratio = done / toFetch.size.toFloat().coerceAtLeast(1f)
                        _syncProgress.value = ratio
                        syncProgressState = ratio
                        syncStage = "Downloading $done of ${toFetch.size}"
                        val coverDest = store.coverFile(track.id, "jpg")
                        if (m.hasCover && !coverDest.exists()) {
                            desktop.downloadCover(track.id, coverDest)?.let {
                                store.setCoverPath(track.id, coverDest.absolutePath)
                            }
                        }
                    } catch (error: Exception) {
                        failed += 1
                    }
                }

                syncStage = "Updating playlists"
                for (playlist in playlists) {
                    for (streamed in playlist.streamed) {
                        store.addTrack(
                            CatalogueTrack(
                                id = streamed.id,
                                title = streamed.title,
                                artist = streamed.artist,
                                album = streamed.album,
                                duration = streamed.duration ?: 0.0,
                                cover = streamed.cover,
                                explicit = false,
                            ),
                        )
                    }
                }

                store.replaceDesktopPlaylists(playlists)
                refreshStore()
                notice = when {
                    catalog.isEmpty() -> "The desktop has nothing to share"
                    toFetch.isEmpty() -> "Already up to date — ${catalog.size} songs, synced lyrics and playlists"
                    failed > 0 -> "$done of ${toFetch.size} songs came over, $failed failed — tap Sync to retry"
                    else -> "$done new songs came over, with lyrics and playlists"
                }
            } catch (error: Exception) {
                notice = friendly(error)
            } finally {
                syncing = false
                _syncProgress.value = 0f
                syncProgressState = 0f
                syncStage = "Syncing library"
            }
        }
    }

    fun saveDisplayName(value: String) {
        val clipped = value.trim().take(32)
        if (clipped == displayName) return
        displayName = clipped
        settings.displayName = clipped
    }

    fun rollNickname() {
        nickname = randomNickname()
        settings.nickname = nickname
    }

    fun setAccent(color: androidx.compose.ui.graphics.Color) {
        Accent.set(color)
        settings.accent = color.toArgb()
        settings.accentSet = true
    }

    private fun measureDownloads() {
        viewModelScope.launch {
            val bytes = withContext(Dispatchers.IO) {
                val folder = java.io.File(getApplication<Application>().filesDir, "music/audio")
                folder.listFiles()?.sumOf { it.length() } ?: 0L
            }
            val megabytes = bytes / (1024.0 * 1024.0)
            _downloadedSize.value = if (bytes > 1024L * 1024L) {
                String.format("%.1f MB", megabytes)
            } else {
                "${bytes / 1024} KB"
            }
        }
    }

    fun clearDownloads() {
        viewModelScope.launch {
            val freed = withContext(Dispatchers.IO) {
                val root = getApplication<Application>().filesDir
                val music = java.io.File(root, "music")
                var total = 0L
                for (folder in listOf("audio", "covers")) {
                    val dir = java.io.File(music, folder)
                    dir.listFiles()?.forEach { file ->
                        total += file.length()
                        file.delete()
                    }
                }
                total
            }
            withContext(Dispatchers.IO) {
                store.clearAudio()
                store.clearCovers()
            }
            measureDownloads()
            refreshStore()
            val megabytes = freed / (1024.0 * 1024.0)
            notice = if (megabytes >= 1.0) {
                String.format("Downloads removed — freed %.1f MB", megabytes)
            } else {
                "Downloads removed"
            }
        }
    }

    fun checkForUpdate() {
        if (updateBusy) return
        viewModelScope.launch {
            updateBusy = true
            try {
                val (name, build) = updater.installedVersion()
                _installedVersion.value = name.ifEmpty { "$build" }
                val found = updater.check()
                if (found == null) {
                    notice = "Could not reach the releases"
                } else if (!AppUpdate.isNewer(name, build, found, updater.abi())) {
                    notice = "You are on the latest build"
                    _update.value = null
                } else {
                    _update.value = found
                }
            } catch (error: Exception) {
                notice = friendly(error)
            } finally {
                updateBusy = false
            }
        }
    }

    fun downloadUpdate() {
        val found = _update.value ?: return
        if (updateBusy) return
        viewModelScope.launch {
            updateBusy = true
            try {
                val url = updater.urlFor(found)
                if (url == null) {
                    notice = "There is no build for this phone"
                    return@launch
                }
                updateFile = updater.download(url) { value -> updateProgress = value }
                updateDownloaded = true
            } catch (error: Exception) {
                notice = friendly(error)
            } finally {
                updateBusy = false
                updateProgress = 0f
            }
        }
    }

    fun installUpdate() {
        val file = updateFile ?: return
        if (!updater.canInstall()) {
            updater.installSettingsIntent().addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK)
                .let { intent ->
                    getApplication<Application>().startActivity(intent)
                }
            notice = "Allow Vynl to install apps, then try again."
            return
        }
        if (!updater.install(file)) {
            notice = "Could not open the installer."
        }
    }

    fun selectTab(index: Int) {
        tab = index
    }

    fun dismissNotice() {
        notice = null
    }

    fun toggleLyrics() {
        val track = player.state.value.track ?: return
        lyricsOpen = !lyricsOpen
        if (lyricsOpen && lyricsTrackId != track.id) {
            loadLyrics(track)
        } else if (lyricsOpen && _lyrics.value == null && lyricsStatus == LyricsStatus.Failed) {
            loadLyrics(track)
        }
    }

    fun ensureLyrics(track: CatalogueTrack) {
        if (lyricsTrackId == track.id) return
        lyricsTrackId = track.id
        loadLyrics(track)
    }

    fun retryLyrics() {
        val track = player.state.value.track ?: return
        lyricsTrackId = track.id
        loadLyrics(track)
    }

    private fun loadLyrics(track: CatalogueTrack) {
        _lyrics.value = null
        lyricsStatus = LyricsStatus.Loading
        lyricsAvailable = false
        viewModelScope.launch {
            try {
                val stored = withContext(Dispatchers.IO) { store.findPrivateTrack(track.id)?.lyrics }
                val result = if (!stored.isNullOrBlank()) {
                    Lyrics.parse(if (stored.contains("<tt")) "ttml" else "lrc", stored)
                } else {
                    repository.lyrics(track)
                }
                _lyrics.value = result
                val useful = result.hasDisplayable
                lyricsStatus = if (useful) LyricsStatus.Ready else LyricsStatus.Missing
                lyricsAvailable = useful
            } catch (error: Exception) {
                _lyrics.value = null
                lyricsStatus = LyricsStatus.Failed
                lyricsAvailable = false
            }
        }
    }

    fun refreshMix() = loadMix()

    private fun loadMix() {
        viewModelScope.launch {
            mixLoading = true
            try {
                val seeds = repository.search(DISCOVERY_QUERY).songs.take(6).map { it.id }
                _mix.value = repository.taste(seeds)
            } catch (error: Exception) {
                mixError = friendly(error)
            } finally {
                mixLoading = false
            }
        }
    }

    fun browse(query: String) {
        browseQuery = query
        if (query.isBlank()) {
            _results.value = null
            browseError = null
            return
        }
        viewModelScope.launch {
            browseSearching = true
            browseError = null
            try {
                _results.value = repository.search(query)
            } catch (error: Exception) {
                browseError = friendly(error)
            } finally {
                browseSearching = false
            }
        }
    }

    fun openAlbum(id: String) {
        viewModelScope.launch {
            detailLoading = true
            try {
                val detail: CatalogueAlbumDetail = repository.album(id)
                detailTitle = detail.album.title
                detailSubtitle = detail.album.artist
                detailCover = detail.album.cover
                detailTracks = detail.tracks
            } catch (error: Exception) {
                browseError = friendly(error)
            } finally {
                detailLoading = false
            }
        }
    }

    fun openArtist(id: String) {
        viewModelScope.launch {
            detailLoading = true
            try {
                val detail: CatalogueArtistDetail = repository.artist(id)
                detailTitle = detail.name
                detailSubtitle = "${detail.topTracks.size} songs"
                detailCover = detail.cover
                detailTracks = detail.topTracks
            } catch (error: Exception) {
                browseError = friendly(error)
            } finally {
                detailLoading = false
            }
        }
    }

    fun openPlaylist(id: String) {
        viewModelScope.launch {
            detailLoading = true
            try {
                val detail: CataloguePlaylistDetail = repository.playlist(id)
                detailTitle = detail.playlist.title
                detailSubtitle = detail.playlist.author ?: "Playlist"
                detailCover = detail.playlist.cover
                detailTracks = detail.tracks
            } catch (error: Exception) {
                browseError = friendly(error)
            } finally {
                detailLoading = false
            }
        }
    }

    fun closeDetail() {
        detailTitle = null
        detailTracks = emptyList()
    }

    fun refreshStore() {
        viewModelScope.launch {
            val library = withContext(Dispatchers.IO) { store.tracks() }
            val playlists = withContext(Dispatchers.IO) { store.playlists() }
            val covers = withContext(Dispatchers.IO) { store.firstTrackCovers() }
            _library.value = library
            _playlists.value = playlists
            playlistCovers.value = covers
        }
    }

    fun openActions(track: CatalogueTrack) {
        actionsTrack = track
    }

    fun closeActions() {
        actionsTrack = null
    }

    fun openPlaylistPicker(track: CatalogueTrack) {
        actionsTrack = null
        playlistPickerFor = track
    }

    fun closePlaylistPicker() {
        playlistPickerFor = null
    }

    fun addToPlaylist(playlistId: String, track: CatalogueTrack) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { store.addToPlaylist(playlistId, track) }
            closePlaylistPicker()
            refreshStore()
            notice = "Added to ${_playlists.value.firstOrNull { it.id == playlistId }?.name ?: "the playlist"}"
            cacheTrackForOffline(track)
        }
    }

    private val cachingTracks = java.util.Collections.synchronizedSet(mutableSetOf<String>())

    private fun cacheTrackForOffline(track: CatalogueTrack) {
        if (!cachingTracks.add(track.id)) return
        viewModelScope.launch {
            try {
                val existing = withContext(Dispatchers.IO) { store.findPrivateTrack(track.id) }
                val cached = existing?.audioPath?.takeIf { it.isNotEmpty() }
                if (cached != null && withContext(Dispatchers.IO) { java.io.File(cached).exists() }) {
                    return@launch
                }
                val cover = downloader.cover(track)
                val audio = downloader.audio(track) { }
                withContext(Dispatchers.IO) {
                    store.addTrack(track)
                    store.setAudioPath(track.id, audio)
                    if (cover != null) store.setCoverPath(track.id, cover)
                }
                refreshStore()
            } catch (error: Exception) {
                notice = "Could not cache ${track.title}"
            } finally {
                cachingTracks.remove(track.id)
            }
        }
    }

    fun createPlaylist(name: String, track: CatalogueTrack?) {
        viewModelScope.launch {
            val id = withContext(Dispatchers.IO) { store.createPlaylist(name) }
            if (track != null) {
                withContext(Dispatchers.IO) { store.addToPlaylist(id, track) }
                cacheTrackForOffline(track)
            }
            closePlaylistPicker()
            refreshStore()
            notice = "Playlist created"
        }
    }

    fun deletePlaylist(id: String) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { store.deletePlaylist(id) }
            refreshStore()
        }
    }

    fun openLocalPlaylist(id: String) {
        openPlaylistId = id
        viewModelScope.launch {
            _playlistTracks.value = withContext(Dispatchers.IO) { store.playlistTracks(id) }
        }
    }

    fun clearLocalPlaylist() {
        openPlaylistId = null
        _playlistTracks.value = emptyList()
    }

    fun playLocal(tracks: List<LocalTrack>, index: Int) {
        if (tracks.isEmpty()) return
        val catalogue = tracks.map { it.toCatalogue() }
        val files = tracks.mapNotNull { track ->
            track.audioPath?.takeIf { it.isNotEmpty() }?.let { track.id to it }
        }.toMap()
        player.play(catalogue, index.coerceIn(0, catalogue.lastIndex), files)
    }

    private fun friendly(error: Exception): String {
        val raw = error.message.orEmpty()
        if (raw.isBlank()) return "something went wrong"
        val at = raw.indexOf("http")
        if (at >= 0) return "the catalogue did not answer"
        return raw.take(160)
    }

    companion object {
        private const val DISCOVERY_QUERY = "daft punk"
    }
}
