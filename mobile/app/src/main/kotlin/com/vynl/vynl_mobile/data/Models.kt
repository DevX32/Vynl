package com.vynl.vynl_mobile.data

import org.json.JSONArray
import org.json.JSONObject

data class CatalogueTrack(
    val id: String,
    val title: String,
    val artist: String,
    val album: String?,
    val albumId: String? = null,
    val artistId: String? = null,
    val duration: Double,
    val cover: String?,
    val explicit: Boolean,
)

data class CatalogueAlbum(
    val id: String,
    val title: String,
    val artist: String,
    val year: Int?,
    val cover: String?,
)

data class CataloguePlaylist(
    val id: String,
    val title: String,
    val author: String?,
    val cover: String?,
)

data class CatalogueResults(
    val songs: List<CatalogueTrack>,
    val albums: List<CatalogueAlbum>,
    val playlists: List<CataloguePlaylist>,
)

data class CatalogueAlbumDetail(
    val album: CatalogueAlbum,
    val description: String?,
    val tracks: List<CatalogueTrack>,
)

data class CatalogueArtistDetail(
    val id: String,
    val name: String,
    val description: String?,
    val cover: String?,
    val topTracks: List<CatalogueTrack>,
    val albums: List<CatalogueAlbum>,
)

data class CataloguePlaylistDetail(
    val playlist: CataloguePlaylist,
    val tracks: List<CatalogueTrack>,
)

data class StreamAddress(
    val url: String,
    val mime: String,
)

class CatalogueException(message: String) : Exception(message)

internal fun JSONObject.optStringOrNull(key: String): String? {
    if (!has(key) || isNull(key)) return null
    val value = optString(key, "")
    return value.ifEmpty { null }
}

internal fun JSONObject.optIntOrNull(key: String): Int? {
    if (!has(key) || isNull(key)) return null
    val value = optInt(key, 0)
    return if (value == 0) null else value
}

internal fun parseTrack(value: JSONObject): CatalogueTrack {
    val album = value.optJSONObject("album")
    val artists = value.optJSONArray("artists")
    val firstArtist = artists?.optJSONObject(0)
    return CatalogueTrack(
        id = value.optString("id", ""),
        title = value.optString("title", "Unknown"),
        artist = value.optString("artist", "Unknown"),
        album = album?.optStringOrNull("name"),
        albumId = album?.optStringOrNull("id"),
        artistId = firstArtist?.optStringOrNull("id"),
        duration = value.optDouble("duration", 0.0),
        cover = value.optStringOrNull("cover"),
        explicit = value.optBoolean("explicit", false),
    )
}

internal fun parseAlbum(value: JSONObject): CatalogueAlbum {
    return CatalogueAlbum(
        id = value.optString("id", ""),
        title = value.optString("title", "Unknown"),
        artist = value.optString("artist", "Unknown"),
        year = value.optIntOrNull("year"),
        cover = value.optStringOrNull("cover"),
    )
}

internal fun parsePlaylist(value: JSONObject): CataloguePlaylist {
    return CataloguePlaylist(
        id = value.optString("id", ""),
        title = value.optString("title", "Unknown"),
        author = value.optStringOrNull("author"),
        cover = value.optStringOrNull("cover"),
    )
}

internal fun parseTracks(value: JSONArray?): List<CatalogueTrack> {
    if (value == null) return emptyList()
    return buildList {
        for (index in 0 until value.length()) {
            val entry = value.optJSONObject(index) ?: continue
            add(parseTrack(entry))
        }
    }
}

internal fun trackToJson(track: CatalogueTrack): JSONObject =
    JSONObject()
        .put("id", track.id)
        .put("title", track.title)
        .put("artist", track.artist)
        .put("album", track.album ?: JSONObject.NULL)
        .put("albumId", track.albumId ?: JSONObject.NULL)
        .put("artistId", track.artistId ?: JSONObject.NULL)
        .put("duration", track.duration)
        .put("cover", track.cover ?: JSONObject.NULL)
        .put("explicit", track.explicit)

internal fun parseTrackLike(value: JSONObject): CatalogueTrack =
    CatalogueTrack(
        id = value.optString("id", ""),
        title = value.optString("title", "Unknown"),
        artist = value.optString("artist", "Unknown"),
        album = value.optStringOrNull("album"),
        albumId = value.optStringOrNull("albumId"),
        artistId = value.optStringOrNull("artistId"),
        duration = value.optDouble("duration", 0.0),
        cover = value.optStringOrNull("cover"),
        explicit = value.optBoolean("explicit", false),
    )

internal fun mixToJson(tracks: List<CatalogueTrack>): String {
    val array = JSONArray()
    for (track in tracks) {
        array.put(trackToJson(track))
    }
    return array.toString()
}

internal fun mixFromJson(text: String): List<CatalogueTrack> = runCatching {
    val array = JSONArray(text)
    buildList {
        for (index in 0 until array.length()) {
            val entry = array.optJSONObject(index) ?: continue
            add(parseTrackLike(entry))
        }
    }
}.getOrDefault(emptyList())

internal fun parseAlbums(value: JSONArray?): List<CatalogueAlbum> {
    if (value == null) return emptyList()
    return buildList {
        for (index in 0 until value.length()) {
            val entry = value.optJSONObject(index) ?: continue
            add(parseAlbum(entry))
        }
    }
}

internal fun parsePlaylists(value: JSONArray?): List<CataloguePlaylist> {
    if (value == null) return emptyList()
    return buildList {
        for (index in 0 until value.length()) {
            val entry = value.optJSONObject(index) ?: continue
            add(parsePlaylist(entry))
        }
    }
}
