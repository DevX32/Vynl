package com.vynl.vynl_mobile.data

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import com.vynl.vynl_mobile.remote.DesktopPlaylist
import java.io.File

private const val SCHEMA_VERSION = 3

class Store(context: Context) {
    private val files = File(context.filesDir, "music")

    fun coverFile(trackId: String, ext: String = "jpg"): File {
        val folder = File(files, "covers").apply { mkdirs() }
        return File(folder, "${safeName(trackId)}.${ext}")
    }

    fun audioFile(trackId: String, ext: String): File {
        val folder = File(files, "audio").apply { mkdirs() }
        val safeExt = ext.lowercase().ifBlank { "m4a" }.replace(Regex("[^a-z0-9]"), "")
        return File(folder, "${safeName(trackId)}.$safeExt")
    }

    private fun safeName(value: String): String =
        value.replace(Regex("[^A-Za-z0-9_\\-]"), "_").take(64)

    private class Helper(context: Context) :
        SQLiteOpenHelper(context, "vynl.db", null, SCHEMA_VERSION) {
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
                    ext TEXT,
                    mtime INTEGER,
                    lyrics TEXT,
                    added_at INTEGER NOT NULL
                )
                """.trimIndent(),
            )
            db.execSQL(
                """
                CREATE TABLE playlists (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL DEFAULT 0
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

        override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
            if (oldVersion < 2) {
                db.execSQL("ALTER TABLE tracks ADD COLUMN ext TEXT")
                db.execSQL("ALTER TABLE tracks ADD COLUMN mtime INTEGER")
                db.execSQL("ALTER TABLE tracks ADD COLUMN lyrics TEXT")
            }
            if (oldVersion < 3) {
                db.execSQL("ALTER TABLE playlists ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0")
                db.execSQL("UPDATE playlists SET updated_at = created_at")
            }
        }
    }

    private val helper = Helper(context)

    fun addTrack(
        track: CatalogueTrack,
        ext: String? = null,
        mtime: Long? = null,
        lyrics: String? = null,
    ) {
        val values = ContentValues().apply {
            put("id", track.id)
            put("title", track.title)
            put("artist", track.artist)
            put("album", track.album)
            put("duration", track.duration)
            put("cover", track.cover)
            put("ext", ext)
            put("mtime", mtime)
            put("lyrics", lyrics)
            put("added_at", System.currentTimeMillis())
        }
        helper.writableDatabase.insertWithOnConflict("tracks", null, values, SQLiteDatabase.CONFLICT_REPLACE)
    }

    fun setAudioPath(trackId: String, path: String, mtime: Long? = null) {
        helper.writableDatabase.update(
            "tracks",
            ContentValues().apply {
                put("audio_path", path)
                if (mtime != null) put("mtime", mtime)
            },
            "id = ?",
            arrayOf(trackId),
        )
    }

    fun clearAudio() {
        helper.writableDatabase.update(
            "tracks",
            ContentValues().apply { put("audio_path", "") },
            null,
            null,
        )
    }

    fun clearCovers() {
        helper.writableDatabase.update(
            "tracks",
            ContentValues().apply { put("cover_path", "") },
            null,
            null,
        )
    }

    fun localMtimes(): Map<String, Long> {
        val result = mutableMapOf<String, Long>()
        helper.readableDatabase.rawQuery(
            "SELECT id, mtime FROM tracks WHERE mtime IS NOT NULL",
            null,
        ).use { cursor ->
            while (cursor.moveToNext()) {
                result[cursor.getString(0)] = cursor.getLong(1)
            }
        }
        return result
    }

    fun localAudioPaths(): Map<String, String> {
        val result = mutableMapOf<String, String>()
        helper.readableDatabase.rawQuery(
            "SELECT id, audio_path FROM tracks WHERE audio_path IS NOT NULL AND audio_path != ''",
            null,
        ).use { cursor ->
            while (cursor.moveToNext()) {
                result[cursor.getString(0)] = cursor.getString(1)
            }
        }
        return result
    }

    fun setCoverPath(trackId: String, path: String) {
        helper.writableDatabase.update(
            "tracks",
            ContentValues().apply { put("cover_path", path) },
            "id = ?",
            arrayOf(trackId),
        )
    }

    fun firstTrackCovers(): Map<String, Pair<String?, String?>> {
        val result = mutableMapOf<String, Pair<String?, String?>>()
        helper.readableDatabase.rawQuery(
            """
            SELECT pt.playlist_id, t.cover_path, t.cover
            FROM playlist_tracks pt
            JOIN tracks t ON t.id = pt.track_id
            WHERE pt.position = (
                SELECT MIN(position) FROM playlist_tracks WHERE playlist_id = pt.playlist_id
            )
            """.trimIndent(),
            null,
        ).use { cursor ->
            while (cursor.moveToNext()) {
                val local = cursor.getString(1)?.takeIf { it.isNotBlank() }
                val remote = cursor.getString(2)?.takeIf { it.isNotBlank() }
                result[cursor.getString(0)] = Pair(local, remote)
            }
        }
        return result
    }

    fun tracks(): List<LocalTrack> {
        val result = mutableListOf<LocalTrack>()
        helper.readableDatabase.rawQuery(
            "SELECT id, title, artist, album, duration, cover, audio_path, cover_path, ext, mtime, lyrics FROM tracks ORDER BY added_at DESC",
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
                    ext = cursor.getString(8),
                    mtime = cursor.getLong(9).takeIf { !cursor.isNull(9) },
                    lyrics = cursor.getString(10),
                )
            }
        }
        return result
    }

    fun playlists(): List<LocalPlaylist> {
        val result = mutableListOf<LocalPlaylist>()
        helper.readableDatabase.rawQuery(
            """
            SELECT p.id, p.name, p.created_at, p.updated_at,
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
                    updatedAt = cursor.getLong(3),
                    trackCount = cursor.getInt(4),
                )
            }
        }
        return result
    }

    fun playlist(id: String): LocalPlaylist? {
        helper.readableDatabase.rawQuery(
            """
            SELECT p.id, p.name, p.created_at, p.updated_at,
                   (SELECT COUNT(*) FROM playlist_tracks t WHERE t.playlist_id = p.id)
            FROM playlists p WHERE p.id = ?
            """.trimIndent(),
            arrayOf(id),
        ).use { cursor ->
            if (cursor.moveToNext()) {
                return LocalPlaylist(
                    id = cursor.getString(0),
                    name = cursor.getString(1),
                    createdAt = cursor.getLong(2),
                    updatedAt = cursor.getLong(3),
                    trackCount = cursor.getInt(4),
                )
            }
        }
        return null
    }

    fun createPlaylist(name: String): String {
        val id = "pl-${System.currentTimeMillis()}"
        val now = System.currentTimeMillis()
        helper.writableDatabase.insert(
            "playlists",
            null,
            ContentValues().apply {
                put("id", id)
                put("name", name.trim().ifEmpty { "New playlist" })
                put("created_at", now)
                put("updated_at", now)
            },
        )
        return id
    }

    fun renamePlaylist(id: String, name: String) {
        helper.writableDatabase.update(
            "playlists",
            ContentValues().apply {
                put("name", name.trim().ifEmpty { "New playlist" })
                put("updated_at", System.currentTimeMillis())
            },
            "id = ?",
            arrayOf(id),
        )
    }

    private fun touchPlaylist(id: String) {
        helper.writableDatabase.update(
            "playlists",
            ContentValues().apply { put("updated_at", System.currentTimeMillis()) },
            "id = ?",
            arrayOf(id),
        )
    }

    fun deletePlaylist(id: String) {
        val db = helper.writableDatabase
        db.delete("playlist_tracks", "playlist_id = ?", arrayOf(id))
        db.delete("playlists", "id = ?", arrayOf(id))
    }

    fun replaceDesktopPlaylists(lists: List<DesktopPlaylist>) {
        val db = helper.writableDatabase
        db.beginTransaction()
        try {
            db.delete("playlist_tracks", "playlist_id LIKE ?", arrayOf("dt-%"))
            db.delete("playlists", "id LIKE ?", arrayOf("dt-%"))
            for (list in lists) {
                val id = "dt-${list.id}"
                db.insertWithOnConflict(
                    "playlists",
                    null,
                    ContentValues().apply {
                        put("id", id)
                        put("name", list.name)
                        put("created_at", list.createdAt)
                        put("updated_at", list.updatedAt.takeIf { it > 0 } ?: list.createdAt)
                    },
                    SQLiteDatabase.CONFLICT_REPLACE,
                )
                var position = 0
                list.trackIds.forEach { trackId ->
                    db.insertWithOnConflict(
                        "playlist_tracks",
                        null,
                        ContentValues().apply {
                            put("playlist_id", id)
                            put("track_id", trackId)
                            put("position", position++)
                        },
                        SQLiteDatabase.CONFLICT_REPLACE,
                    )
                }
            }
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
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
        touchPlaylist(playlistId)
    }

    fun removeFromPlaylist(playlistId: String, trackId: String) {
        helper.writableDatabase.delete(
            "playlist_tracks",
            "playlist_id = ? AND track_id = ?",
            arrayOf(playlistId, trackId),
        )
        touchPlaylist(playlistId)
    }

    fun playlistTracks(playlistId: String): List<LocalTrack> {
        val result = mutableListOf<LocalTrack>()
        helper.readableDatabase.rawQuery(
            """
            SELECT t.id, t.title, t.artist, t.album, t.duration, t.cover, t.audio_path, t.cover_path, t.ext, t.mtime, t.lyrics
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
                    ext = cursor.getString(8),
                    mtime = cursor.getLong(9).takeIf { !cursor.isNull(9) },
                    lyrics = cursor.getString(10),
                )
            }
        }
        return result
    }

    fun findPrivateTrack(trackId: String): LocalTrack? {
        helper.readableDatabase.rawQuery(
            "SELECT id, title, artist, album, duration, cover, audio_path, cover_path, ext, mtime, lyrics FROM tracks WHERE id = ?",
            arrayOf(trackId),
        ).use { cursor ->
            if (cursor.moveToNext()) {
                return LocalTrack(
                    id = cursor.getString(0),
                    title = cursor.getString(1),
                    artist = cursor.getString(2),
                    album = cursor.getString(3),
                    duration = cursor.getDouble(4),
                    cover = cursor.getString(5),
                    audioPath = cursor.getString(6),
                    coverPath = cursor.getString(7),
                    ext = cursor.getString(8),
                    mtime = cursor.getLong(9).takeIf { !cursor.isNull(9) },
                    lyrics = cursor.getString(10),
                )
            }
        }
        return null
    }

    fun cloudPayload(id: String): CloudPlaylistPayload? {
        val playlist = playlist(id) ?: return null
        val canonical = id.removePrefix("dt-")
        return CloudPlaylistPayload(
            localId = canonical,
            name = playlist.name,
            createdAt = playlist.createdAt,
            updatedAt = playlist.updatedAt.takeIf { it > 0 } ?: playlist.createdAt,
            tracks = playlistTracks(id).map { it.toCatalogue() },
        )
    }

    fun localIdForCloud(canonical: String): String {
        if (playlist(canonical) != null) return canonical
        val synced = "dt-$canonical"
        if (playlist(synced) != null) return synced
        return canonical
    }

    fun upsertCloudPlaylist(
        localId: String,
        name: String,
        createdAt: Long,
        updatedAt: Long,
        tracks: List<CatalogueTrack>,
    ) {
        val db = helper.writableDatabase
        db.beginTransaction()
        try {
            db.insertWithOnConflict(
                "playlists",
                null,
                ContentValues().apply {
                    put("id", localId)
                    put("name", name.trim().ifEmpty { "Playlist" })
                    put("created_at", createdAt)
                    put("updated_at", updatedAt)
                },
                SQLiteDatabase.CONFLICT_REPLACE,
            )
            db.delete("playlist_tracks", "playlist_id = ?", arrayOf(localId))
            var position = 0
            for (track in tracks) {
                upsertTrackMetadata(db, track)
                db.insertWithOnConflict(
                    "playlist_tracks",
                    null,
                    ContentValues().apply {
                        put("playlist_id", localId)
                        put("track_id", track.id)
                        put("position", position++)
                    },
                    SQLiteDatabase.CONFLICT_REPLACE,
                )
            }
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    private fun upsertTrackMetadata(db: SQLiteDatabase, track: CatalogueTrack) {
        val exists = db.rawQuery("SELECT 1 FROM tracks WHERE id = ?", arrayOf(track.id)).use {
            it.moveToFirst()
        }
        if (!exists) {
            db.insertOrThrow(
                "tracks",
                null,
                ContentValues().apply {
                    put("id", track.id)
                    put("title", track.title)
                    put("artist", track.artist)
                    put("album", track.album)
                    put("duration", track.duration)
                    put("cover", track.cover)
                    put("added_at", System.currentTimeMillis())
                },
            )
        } else {
            db.update(
                "tracks",
                ContentValues().apply {
                    put("title", track.title)
                    put("artist", track.artist)
                    put("album", track.album)
                    if (track.duration > 0) put("duration", track.duration)
                    put("cover", track.cover)
                },
                "id = ?",
                arrayOf(track.id),
            )
        }
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
    val ext: String? = null,
    val mtime: Long? = null,
    val lyrics: String? = null,
) {
    val isDownloaded: Boolean
        get() = !audioPath.isNullOrEmpty()

    fun toCatalogue(): CatalogueTrack = CatalogueTrack(
        id = id,
        title = title,
        artist = artist,
        album = album,
        duration = duration,
        cover = coverPath ?: cover,
        explicit = false,
    )
}

data class LocalPlaylist(
    val id: String,
    val name: String,
    val createdAt: Long,
    val updatedAt: Long,
    val trackCount: Int,
)

data class CloudPlaylistPayload(
    val localId: String,
    val name: String,
    val createdAt: Long,
    val updatedAt: Long,
    val tracks: List<CatalogueTrack>,
)
