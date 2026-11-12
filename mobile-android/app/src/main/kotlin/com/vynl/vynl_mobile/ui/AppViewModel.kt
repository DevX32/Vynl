package com.vynl.vynl_mobile.ui

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.vynl.vynl_mobile.data.CatalogueAlbumDetail
import com.vynl.vynl_mobile.data.CatalogueArtistDetail
import com.vynl.vynl_mobile.data.CataloguePlaylistDetail
import com.vynl.vynl_mobile.data.CatalogueRepository
import com.vynl.vynl_mobile.data.CatalogueResults
import com.vynl.vynl_mobile.data.CatalogueTrack
import com.vynl.vynl_mobile.playback.PlayerController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class AppViewModel(application: Application) : AndroidViewModel(application) {
    private val repository = CatalogueRepository()
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

    init {
        viewModelScope.launch { player.connect() }
        loadMix()
    }

    fun selectTab(index: Int) {
        tab = index
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
