package com.vynl.vynl_mobile.data

import com.vynl.vynl_mobile.CatalogueBridge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

/**
 * The catalogue client, written in Rust and shared with the desktop. Every
 * call blocks, so they all run off the main thread.
 */
class CatalogueRepository {

    suspend fun search(query: String): CatalogueResults = withContext(Dispatchers.IO) {
        val body = JSONObject(CatalogueBridge.nativeSearch(query))
        CatalogueResults(
            songs = parseTracks(body.optJSONArray("songs")),
            albums = parseAlbums(body.optJSONArray("albums")),
            playlists = parsePlaylists(body.optJSONArray("playlists")),
        )
    }

    suspend fun album(id: String): CatalogueAlbumDetail = withContext(Dispatchers.IO) {
        val body = JSONObject(CatalogueBridge.nativeAlbum(id))
        val album = body.optJSONObject("album") ?: JSONObject()
        CatalogueAlbumDetail(
            album = parseAlbum(album),
            description = body.optStringOrNull("description"),
            tracks = parseTracks(body.optJSONArray("tracks")),
        )
    }

    suspend fun artist(id: String): CatalogueArtistDetail = withContext(Dispatchers.IO) {
        val body = JSONObject(CatalogueBridge.nativeArtist(id))
        CatalogueArtistDetail(
            id = body.optString("id", ""),
            name = body.optString("name", "Unknown"),
            description = body.optStringOrNull("description"),
            cover = body.optStringOrNull("cover"),
            topTracks = parseTracks(body.optJSONArray("topTracks")),
            albums = parseAlbums(body.optJSONArray("albums")),
        )
    }

    suspend fun playlist(id: String): CataloguePlaylistDetail = withContext(Dispatchers.IO) {
        val body = JSONObject(CatalogueBridge.nativePlaylist(id))
        val playlist = body.optJSONObject("playlist") ?: JSONObject()
        CataloguePlaylistDetail(
            playlist = parsePlaylist(playlist),
            tracks = parseTracks(body.optJSONArray("tracks")),
        )
    }

    suspend fun stream(id: String): StreamAddress = withContext(Dispatchers.IO) {
        val body = JSONObject(CatalogueBridge.nativeStream(id))
        StreamAddress(
            url = body.optString("url", ""),
            mime = body.optString("mime", ""),
        )
    }

    suspend fun taste(seeds: List<String>): List<CatalogueTrack> = withContext(Dispatchers.IO) {
        val body = JSONObject(CatalogueBridge.nativeTaste(JSONArray(seeds).toString()))
        parseTracks(body.optJSONArray("tracks"))
    }
}
