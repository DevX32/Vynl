package com.vynl.vynl_mobile.data

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper

/**
 * The phone's own store: the songs it has, the playlists the user makes and
 * where the downloaded files live.
 */
class Store(context: Context) {
    private val helper = Helper(context.applicationContext)

    private class Helper(context: Context) :
        SQLiteOpenHelper(context, "vynl.db", null, 1) {
        override fun onCreate(db: SQLiteDatabase) {
            db.execSQL(
                """
                CREATE TABLE tracks (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    artist TEXT NOT NULL,
                    album TEXT,
                    duration REAL NOT NULL DEFAULT 0,
                    cover TEXT,
                    audio_path TEXT,
                    cover_path TEXT,
                    added_at INTEGER NOT NULL
                )
                """.trimIndent(),
            )
            db.execSQL(
                """
                CREATE TABLE playlists (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                )
                """.trimIndent(),
            )
            db.execSQL(
                """
                CREATE TABLE playlist_tracks (
                    playlist_id TEXT NOT NULL,
                    track_id TEXT NOT NULL,
                    position INTEGER NOT NULL,
                    PRIMARY KEY (playlist_id, track_id)
                )
                """.trimIndent(),
            )
        }

        override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) = Unit
    }

    fun addTrack(track: CatalogueTrack) {
        val db = helper.writableDatabase
        val values = ContentValues().apply {
            put("id", track.id)
            put("title", track.title)
            put("artist", track.artist)
            put("album", track.album)
            put("duration", track.duration)
            put("cover", track.cover)
            put("added_at", System.currentTimeMillis())
        }
        db.insertWithOnConflict("tracks", null, values, SQLiteDatabase.CONFLICT_REPLACE)
    }

    fun setAudioPath(trackId: String, path: String) {
        helper.writableDatabase.update(
            "tracks",
            ContentValues().apply { put("audio_path", path) },
            "id = ?",
            arrayOf(trackId),
        )
    }

    /** Forgets where the downloaded files were, they are gone from the phone. */
    fun clearAudio() {
        helper.writableDatabase.update(
            "tracks",
            ContentValues().apply { put("audio_path", "") },
            null,
            null,
        )
    }

    fun setCoverPath(trackId: String, path: String) {
        helper.writableDatabase.update(
            "tracks",
            ContentValues().apply { put("cover_path", path) },
            "id = ?",
            arrayOf(trackId),
        )
    }

    fun tracks(): List<LocalTrack> {
        val result = mutableListOf<LocalTrack>()
        helper.readableDatabase.rawQuery(
            "SELECT id, title, artist, album, duration, cover, audio_path, cover_path FROM tracks ORDER BY added_at DESC",
            null,
        ).use { cursor ->
            while (cursor.moveToNext()) {
                result += LocalTrack(
                    id = cursor.getString(0),
                    title = cursor.getString(1),
                    artist = cursor.getString(2),
                    album = cursor.getString(3),
                    duration = cursor.getDouble(4),
                    cover = cursor.getString(5),
                    audioPath = cursor.getString(6),
                    coverPath = cursor.getString(7),
                )
            }
        }
        return result
    }

    fun playlists(): List<LocalPlaylist> {
        val result = mutableListOf<LocalPlaylist>()
        helper.readableDatabase.rawQuery(
            """
            SELECT p.id, p.name, p.created_at,
                   (SELECT COUNT(*) FROM playlist_tracks t WHERE t.playlist_id = p.id)
            FROM playlists p ORDER BY p.created_at ASC
            """.trimIndent(),
            null,
        ).use { cursor ->
            while (cursor.moveToNext()) {
                result += LocalPlaylist(
                    id = cursor.getString(0),
                    name = cursor.getString(1),
                    createdAt = cursor.getLong(2),
                    trackCount = cursor.getInt(3),
                )
            }
        }
        return result
    }

    fun createPlaylist(name: String): String {
        val id = "pl-${System.currentTimeMillis()}"
        helper.writableDatabase.insert(
            "playlists",
            null,
            ContentValues().apply {
                put("id", id)
                put("name", name.trim().ifEmpty { "New playlist" })
                put("created_at", System.currentTimeMillis())
            },
        )
        return id
    }

    fun renamePlaylist(id: String, name: String) {
        helper.writableDatabase.update(
            "playlists",
            ContentValues().apply { put("name", name.trim().ifEmpty { "New playlist" }) },
            "id = ?",
            arrayOf(id),
        )
    }

    fun deletePlaylist(id: String) {
        val db = helper.writableDatabase
        db.delete("playlist_tracks", "playlist_id = ?", arrayOf(id))
        db.delete("playlists", "id = ?", arrayOf(id))
    }

    fun addToPlaylist(playlistId: String, track: CatalogueTrack) {
        addTrack(track)
        val db = helper.writableDatabase
        val position = db.rawQuery(
            "SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?",
            arrayOf(playlistId),
        ).use { cursor ->
            if (cursor.moveToFirst()) cursor.getInt(0) else 0
        }
        db.insertWithOnConflict(
            "playlist_tracks",
            null,
            ContentValues().apply {
                put("playlist_id", playlistId)
                put("track_id", track.id)
                put("position", position)
            },
            SQLiteDatabase.CONFLICT_REPLACE,
        )
    }

    fun removeFromPlaylist(playlistId: String, trackId: String) {
        helper.writableDatabase.delete(
            "playlist_tracks",
            "playlist_id = ? AND track_id = ?",
            arrayOf(playlistId, trackId),
        )
    }

    fun playlistTracks(playlistId: String): List<LocalTrack> {
        val result = mutableListOf<LocalTrack>()
        helper.readableDatabase.rawQuery(
            """
            SELECT t.id, t.title, t.artist, t.album, t.duration, t.cover, t.audio_path, t.cover_path
            FROM playlist_tracks p
            JOIN tracks t ON t.id = p.track_id
            WHERE p.playlist_id = ?
            ORDER BY p.position ASC
            """.trimIndent(),
            arrayOf(playlistId),
        ).use { cursor ->
            while (cursor.moveToNext()) {
                result += LocalTrack(
                    id = cursor.getString(0),
                    title = cursor.getString(1),
                    artist = cursor.getString(2),
                    album = cursor.getString(3),
                    duration = cursor.getDouble(4),
                    cover = cursor.getString(5),
                    audioPath = cursor.getString(6),
                    coverPath = cursor.getString(7),
                )
            }
        }
        return result
    }
}

data class LocalTrack(
    val id: String,
    val title: String,
    val artist: String,
    val album: String?,
    val duration: Double,
    val cover: String?,
    val audioPath: String?,
    val coverPath: String?,
) {
    val isDownloaded: Boolean
        get() = !audioPath.isNullOrEmpty()

    fun toCatalogue(): CatalogueTrack = CatalogueTrack(
        id = id,
        title = title,
        artist = artist,
        album = album,
        duration = duration,
        cover = cover,
        explicit = false,
    )
}

data class LocalPlaylist(
    val id: String,
    val name: String,
    val createdAt: Long,
    val trackCount: Int,
)
