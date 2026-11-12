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
import com.vynl.vynl_mobile.data.LocalTrack
import com.vynl.vynl_mobile.data.Store
import com.vynl.vynl_mobile.playback.PlayerController
import com.vynl.vynl_mobile.remote.Desktop
import com.vynl.vynl_mobile.remote.DesktopPlayer
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

    private val _playlistTracks = MutableStateFlow<List<LocalTrack>>(emptyList())
    val playlistTracks: StateFlow<List<LocalTrack>> = _playlistTracks.asStateFlow()

    var actionsTrack by mutableStateOf<CatalogueTrack?>(null)
        private set

    var playlistPickerFor by mutableStateOf<CatalogueTrack?>(null)
        private set

    var downloadingId by mutableStateOf<String?>(null)
        private set

    var downloadProgress by mutableStateOf(0f)
        private set

    var notice by mutableStateOf<String?>(null)
        private set

    var openPlaylistId by mutableStateOf<String?>(null)
        private set

    var lyricsOpen by mutableStateOf(false)
        private set

    private val _lyrics = MutableStateFlow<Lyrics?>(null)
    val lyrics: StateFlow<Lyrics?> = _lyrics.asStateFlow()

    var lyricsLoading by mutableStateOf(false)
        private set

    private var lyricsTrackId: String? = null

    private val settings = SettingsStore(application)
    private val updater = AppUpdate(application)
    val desktop = Desktop(application)

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

    private val _remote = MutableStateFlow<DesktopPlayer?>(null)
    val remote: StateFlow<DesktopPlayer?> = _remote.asStateFlow()

    var pairing by mutableStateOf(false)
        private set

    init {
        viewModelScope.launch { player.connect() }
        if (settings.accentSet) {
            Accent.set(androidx.compose.ui.graphics.Color(settings.accent))
        }
        val (name, build) = updater.installedVersion()
        _installedVersion.value = name.ifEmpty { "$build" }
        measureDownloads()
        loadMix()
        refreshStore()
    }

    /** Pairs the phone with the desktop, using the pin it shows. */
    fun pairDesktop(address: String, pin: String) {
        if (pairing) return
        viewModelScope.launch {
            pairing = true
            try {
                desktop.address = address
                if (desktop.pair(pin)) {
                    notice = "Paired with the desktop"
                    refreshRemote()
                } else {
                    notice = "That pin was not accepted"
                }
            } catch (error: Exception) {
                notice = friendly(error)
            } finally {
                pairing = false
            }
        }
    }

    /** Asks the desktop what it is playing. */
    fun refreshRemote() {
        if (!desktop.paired) return
        viewModelScope.launch {
            _remote.value = desktop.player()
        }
    }

    /** Sends a command to the desktop. */
    fun remoteAction(name: String) {
        if (!desktop.paired) return
        viewModelScope.launch {
            desktop.action(name)
            refreshRemote()
        }
    }

    /** Pulls the songs the desktop holds into the phone. */
    fun syncLibrary() {
        if (!desktop.paired) return
        viewModelScope.launch {
            pairing = true
            try {
                val tracks = desktop.catalog()
                withContext(Dispatchers.IO) {
                    tracks.forEach { track ->
                        store.addTrack(
                            CatalogueTrack(
                                id = track.id,
                                title = track.title,
                                artist = track.artist,
                                album = track.album,
                                duration = track.duration,
                                cover = track.cover,
                                explicit = false,
                            ),
                        )
                    }
                }
                refreshStore()
                notice = "${tracks.size} songs came over"
            } catch (error: Exception) {
                notice = friendly(error)
            } finally {
                pairing = false
            }
        }
    }
    /** The colour the app wears. */
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
            withContext(Dispatchers.IO) {
                java.io.File(getApplication<Application>().filesDir, "music/audio")
                    .listFiles()
                    ?.forEach { it.delete() }
            }
            withContext(Dispatchers.IO) {
                store.clearAudio()
            }
            measureDownloads()
            refreshStore()
            notice = "The downloads are gone"
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
                } else if (!AppUpdate.isNewer(name, build, found)) {
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

    fun openLyrics(track: CatalogueTrack) {
        lyricsOpen = true
        if (lyricsTrackId == track.id && _lyrics.value != null) return
        lyricsTrackId = track.id
        _lyrics.value = null
        lyricsLoading = true
        viewModelScope.launch {
            try {
                _lyrics.value = repository.lyrics(track)
            } catch (error: Exception) {
                notice = friendly(error)
            } finally {
                lyricsLoading = false
            }
        }
    }

    fun closeLyrics() {
        lyricsOpen = false
    }

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
            _library.value = library
            _playlists.value = playlists
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
        }
    }

    fun createPlaylist(name: String, track: CatalogueTrack?) {
        viewModelScope.launch {
            val id = withContext(Dispatchers.IO) { store.createPlaylist(name) }
            if (track != null) {
                withContext(Dispatchers.IO) { store.addToPlaylist(id, track) }
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

    fun download(track: CatalogueTrack) {
        if (downloadingId != null) return
        viewModelScope.launch {
            downloadingId = track.id
            downloadProgress = 0f
            try {
                val cover = downloader.cover(track)
                val audio = downloader.audio(track) { value -> downloadProgress = value }
                withContext(Dispatchers.IO) {
                    store.addTrack(track)
                    store.setAudioPath(track.id, audio)
                    if (cover != null) store.setCoverPath(track.id, cover)
                }
                closeActions()
                refreshStore()
                notice = "Downloaded ${track.title}"
            } catch (error: Exception) {
                notice = friendly(error)
            } finally {
                downloadingId = null
                downloadProgress = 0f
            }
        }
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
