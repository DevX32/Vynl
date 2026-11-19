package com.vynl.vynl_mobile.data

import android.content.ContentValues
import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import com.vynl.vynl_mobile.remote.DesktopPlaylist
import org.json.JSONArray
import java.io.File

private const val SCHEMA_VERSION = 6

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

    private fun safeName(value: String): String {
        val sanitized = value.replace(Regex("[^A-Za-z0-9_-]"), "_").take(48)
        if (sanitized == value) return sanitized
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
        val hex = digest.joinToString("") { "%02x".format(it) }.take(16)
        return "${sanitized}_$hex".take(64)
    }

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
            db.execSQL(
                """
                CREATE TABLE desktop_playlists (
                    playlist_id TEXT NOT NULL PRIMARY KEY
                )
                """.trimIndent(),
            )
            db.execSQL(
                """
                CREATE TABLE playlist_marks (
                    playlist_id TEXT NOT NULL,
                    track_id TEXT NOT NULL,
                    add_at INTEGER NOT NULL DEFAULT 0,
                    remove_at INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY (playlist_id, track_id)
                )
                """.trimIndent(),
            )
            db.execSQL(
                "CREATE INDEX IF NOT EXISTS index_playlist_marks_track_id ON playlist_marks (track_id)",
            )
            db.execSQL(
                "CREATE INDEX IF NOT EXISTS index_playlist_tracks_track_id ON playlist_tracks (track_id)",
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
            if (oldVersion < 4) {
                db.execSQL(
                    """
                    CREATE TABLE IF NOT EXISTS desktop_playlists (
                        playlist_id TEXT NOT NULL PRIMARY KEY
                    )
                    """.trimIndent(),
                )
                db.execSQL(
                    "INSERT OR IGNORE INTO desktop_playlists (playlist_id) " +
                        "SELECT id FROM playlists WHERE id LIKE 'dt-%'",
                )
            }
            if (oldVersion < 5) {
                db.execSQL(
                    """
                    CREATE TABLE IF NOT EXISTS playlist_marks (
                        playlist_id TEXT NOT NULL,
                        track_id TEXT NOT NULL,
                        add_at INTEGER NOT NULL DEFAULT 0,
                        remove_at INTEGER NOT NULL DEFAULT 0,
                        PRIMARY KEY (playlist_id, track_id)
                    )
                    """.trimIndent(),
                )
            }
            if (oldVersion < 6) {
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS index_playlist_tracks_track_id ON playlist_tracks (track_id)",
                )
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS index_playlist_marks_track_id ON playlist_marks (track_id)",
                )
            }
        }
    }

    private var helper = Helper(context)
    private val appContext = context.applicationContext

    private fun recreateCorruptedDb() {
        runCatching { helper.close() }
        for (suffix in listOf("", "-wal", "-shm")) {
            runCatching { appContext.getDatabasePath("vynl.db$suffix")?.delete() }
        }
        helper = Helper(appContext)
    }

    private fun writableDb(): SQLiteDatabase = try {
        helper.writableDatabase
    } catch (error: android.database.sqlite.SQLiteDatabaseCorruptException) {
        android.util.Log.e("Vynl", "database is corrupt; rebuilding", error)
        recreateCorruptedDb()
        helper.writableDatabase
    }

    private fun readableDb(): SQLiteDatabase = try {
        helper.readableDatabase
    } catch (error: android.database.sqlite.SQLiteDatabaseCorruptException) {
        android.util.Log.e("Vynl", "database is corrupt; rebuilding", error)
        recreateCorruptedDb()
        helper.readableDatabase
    }

    fun addTrack(
        track: CatalogueTrack,
        ext: String? = null,
        mtime: Long? = null,
        lyrics: String? = null,
    ) {
        val db = writableDb()
        val exists = db.rawQuery("SELECT 1 FROM tracks WHERE id = ?", arrayOf(track.id)).use {
            it.moveToFirst()
        }
        if (exists) {
            val values = ContentValues().apply {
                put("title", track.title)
                put("artist", track.artist)
                put("album", track.album)
                put("duration", track.duration)
                put("cover", track.cover)
                if (ext != null) put("ext", ext)
                if (mtime != null) put("mtime", mtime)
                if (lyrics != null) put("lyrics", lyrics)
            }
            db.update("tracks", values, "id = ?", arrayOf(track.id))
        } else {
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
            db.insertWithOnConflict("tracks", null, values, SQLiteDatabase.CONFLICT_REPLACE)
        }
    }

    fun setAudioPath(trackId: String, path: String, mtime: Long? = null) {
        writableDb().update(
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
        writableDb().update(
            "tracks",
            ContentValues().apply { put("audio_path", "") },
            null,
            null,
        )
    }

    fun clearCovers() {
        writableDb().update(
            "tracks",
            ContentValues().apply { put("cover_path", "") },
            null,
            null,
        )
    }

    fun localMtimes(): Map<String, Long> {
        val result = mutableMapOf<String, Long>()
        readableDb().rawQuery(
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
        readableDb().rawQuery(
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
        writableDb().update(
            "tracks",
            ContentValues().apply { put("cover_path", path) },
            "id = ?",
            arrayOf(trackId),
        )
    }

    fun firstTrackCovers(): Map<String, Pair<String?, String?>> {
        val result = mutableMapOf<String, Pair<String?, String?>>()
        readableDb().rawQuery(
            """
            SELECT pt.playlist_id, t.cover_path, t.cover
            FROM playlist_tracks pt
            JOIN tracks t ON t.id = pt.track_id
            WHERE pt.playlist_id != 'liked'
            AND pt.position = (
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
        readableDb().rawQuery(
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
        readableDb().rawQuery(
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
        val order = storedCustomOrder()
        return if (order.isNullOrEmpty()) {
            result
        } else {
            val rank = order.withIndex().associate { it.value to it.index }
            result.sortedWith(
                compareBy<LocalPlaylist> {
                    rank[it.id] ?: Int.MAX_VALUE
                }.thenBy { it.createdAt },
            )
        }
    }

    fun storedCustomOrder(): List<String>? {
        val file = File(files, "library_order.json")
        if (!file.isFile) return null
        return runCatching {
            val json = org.json.JSONObject(file.readText(Charsets.UTF_8))
            val array = json.optJSONArray("orderIds") ?: return@runCatching null
            (0 until array.length())
                .map { array.optString(it) }
                .filter { it.isNotBlank() }
                .takeIf { it.isNotEmpty() }
        }.getOrNull()
    }

    fun storedCustomOrderUpdatedAt(): Long {
        val file = File(files, "library_order.json")
        if (!file.isFile) return 0L
        return runCatching {
            org.json.JSONObject(file.readText(Charsets.UTF_8))
                .optLong("updatedAt", 0L)
        }.getOrDefault(0L)
    }

    fun setCustomOrder(ids: List<String>, updatedAt: Long = System.currentTimeMillis()) {
        files.mkdirs()
        val json = org.json.JSONObject().apply {
            put("orderIds", org.json.JSONArray(ids))
            put("updatedAt", updatedAt)
        }
        val file = File(files, "library_order.json")
        val tmp = File(files, "library_order.json.tmp")
        tmp.writeText(json.toString(), Charsets.UTF_8)
        if (!tmp.renameTo(file)) {
            file.writeBytes(tmp.readBytes())
            tmp.delete()
        }
    }

    fun playlist(id: String): LocalPlaylist? {
        readableDb().rawQuery(
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

    @Synchronized
    fun createPlaylist(name: String): String {
        val id = "pl-${System.currentTimeMillis().toString(36)}-${(0..0xffff).random().toString(16).padStart(4, '0')}"
        val now = System.currentTimeMillis()
        writableDb().insert(
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
        writableDb().update(
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
        writableDb().update(
            "playlists",
            ContentValues().apply { put("updated_at", System.currentTimeMillis()) },
            "id = ?",
            arrayOf(id),
        )
    }

    @Synchronized
    fun deletePlaylist(id: String) {
        val db = writableDb()
        db.beginTransaction()
        try {
            db.delete("playlist_tracks", "playlist_id = ?", arrayOf(id))
            db.delete("playlists", "id = ?", arrayOf(id))
            db.delete("desktop_playlists", "playlist_id = ?", arrayOf(id))
            db.delete("playlist_marks", "playlist_id = ?", arrayOf(id))
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    @Synchronized
    @Suppress("LongMethod")
    fun rekeyLegacyDtPlaylists() {
        val db = writableDb()
        db.beginTransaction()
        try {
            val dtIds = mutableListOf<String>()
            db.rawQuery("SELECT id FROM playlists WHERE id LIKE 'dt-%'", null).use { cursor ->
                while (cursor.moveToNext()) dtIds += cursor.getString(0)
            }
            for (dtId in dtIds) {
                val canonical = dtId.removePrefix("dt-")
                val exists = db.rawQuery(
                    "SELECT 1 FROM playlists WHERE id = ?",
                    arrayOf(canonical),
                ).use { it.moveToFirst() }
                if (!exists) {
                    db.update(
                        "playlists",
                        ContentValues().apply { put("id", canonical) },
                        "id = ?",
                        arrayOf(dtId),
                    )
                    db.update(
                        "playlist_tracks",
                        ContentValues().apply { put("playlist_id", canonical) },
                        "playlist_id = ?",
                        arrayOf(dtId),
                    )
                    db.update(
                        "playlist_marks",
                        ContentValues().apply { put("playlist_id", canonical) },
                        "playlist_id = ?",
                        arrayOf(dtId),
                    )
                    db.update(
                        "desktop_playlists",
                        ContentValues().apply { put("playlist_id", canonical) },
                        "playlist_id = ?",
                        arrayOf(dtId),
                    )
                    continue
                }
                val dtName = db.rawQuery(
                    "SELECT name, created_at, updated_at FROM playlists WHERE id = ?",
                    arrayOf(dtId),
                ).use { cursor ->
                    if (cursor.moveToFirst()) {
                        Triple(cursor.getString(0), cursor.getLong(1), cursor.getLong(2))
                    } else {
                        null
                    }
                }
                if (dtName == null) {
                    deletePlaylist(dtId, db)
                    continue
                }
                val dtTracks = playlistTracks(dtId).map { it.toCatalogue().copy(cover = it.cover) }
                val dtMarks = readMarks(db, dtId)
                val dtAdds = dtMarks.map { (trackId, pair) -> SyncMark(trackId, pair.first) }
                val dtRemovals = dtMarks.map { (trackId, pair) -> SyncMark(trackId, pair.second) }
                mergePlaylistInTransaction(
                    db,
                    canonical,
                    dtName.first,
                    dtName.second,
                    dtName.third,
                    dtTracks,
                    dtAdds,
                    dtRemovals,
                )
                deletePlaylist(dtId, db)
            }
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    private fun deletePlaylist(id: String, db: SQLiteDatabase) {
        db.delete("playlist_tracks", "playlist_id = ?", arrayOf(id))
        db.delete("playlists", "id = ?", arrayOf(id))
        db.delete("desktop_playlists", "playlist_id = ?", arrayOf(id))
        db.delete("playlist_marks", "playlist_id = ?", arrayOf(id))
    }

    private fun readMarks(
        db: SQLiteDatabase,
        playlistId: String,
    ): Map<String, Pair<Long, Long>> {
        val marks = mutableMapOf<String, Pair<Long, Long>>()
        db.rawQuery(
            "SELECT track_id, add_at, remove_at FROM playlist_marks WHERE playlist_id = ?",
            arrayOf(playlistId),
        ).use { cursor ->
            while (cursor.moveToNext()) {
                marks[cursor.getString(0)] = cursor.getLong(1) to cursor.getLong(2)
            }
        }
        return marks
    }

    private fun readTrackKeys(db: SQLiteDatabase, playlistId: String): List<String> {
        val ids = mutableListOf<String>()
        db.rawQuery(
            "SELECT track_id FROM playlist_tracks WHERE playlist_id = ? ORDER BY position ASC",
            arrayOf(playlistId),
        ).use { cursor ->
            while (cursor.moveToNext()) ids += cursor.getString(0)
        }
        return ids.map { "ytm:$it" }
    }

    private fun trackKey(track: CatalogueTrack): String = "ytm:${track.id.trim()}"

    private fun isDesktopPlaylist(id: String): Boolean {
        if (id.startsWith("dt-")) return true
        return writableDb().rawQuery(
            "SELECT 1 FROM desktop_playlists WHERE playlist_id = ?",
            arrayOf(id),
        ).use { cursor -> cursor.moveToFirst() }
    }

    @Synchronized
    fun replaceDesktopPlaylists(lists: List<DesktopPlaylist>, protect: Set<String> = emptySet()) {
        rekeyLegacyDtPlaylists()
        val db = writableDb()
        db.beginTransaction()
        try {
            val owned = mutableSetOf<String>()
            db.rawQuery("SELECT playlist_id FROM desktop_playlists", null).use { cursor ->
                while (cursor.moveToNext()) owned += cursor.getString(0)
            }
            db.rawQuery("SELECT id FROM playlists WHERE id LIKE 'dt-%'", null).use { cursor ->
                while (cursor.moveToNext()) owned += cursor.getString(0)
            }
            val incoming = lists.map { it.id }.toSet()
            for (id in owned) {
                val stale = id.startsWith("dt-") || id !in incoming
                if (!stale || id in protect) continue
                deletePlaylist(id, db)
            }
            db.delete("desktop_playlists", null, null)

            val now = System.currentTimeMillis()
            for (list in lists) {
                val id = list.id
                val existingMarks = readMarks(db, id)
                val localTrackIds = mutableListOf<String>()
                db.rawQuery(
                    "SELECT track_id FROM playlist_tracks WHERE playlist_id = ? ORDER BY position ASC",
                    arrayOf(id),
                ).use { cursor ->
                    while (cursor.moveToNext()) localTrackIds += cursor.getString(0)
                }
                val incomingKeys = (list.trackIds + list.streamed.map { it.id })
                    .distinct()
                    .map { "ytm:$it" }

                val visibleOrder = mutableListOf<String>()
                for (key in incomingKeys) {
                    val (add, remove) = existingMarks[key] ?: (0L to 0L)
                    if (add == 0L && remove == 0L) {
                        visibleOrder += key
                    } else if (add >= remove) {
                        visibleOrder += key
                    }
                }
                val incomingSet = incomingKeys.toHashSet()
                for (trackId in localTrackIds) {
                    val key = "ytm:$trackId"
                    if (key !in incomingSet) {
                        visibleOrder += key
                    }
                }

                val created = list.createdAt.takeIf { it > 0 } ?: now
                val updated = list.updatedAt.takeIf { it > 0 } ?: now
                db.insertWithOnConflict(
                    "playlists",
                    null,
                    ContentValues().apply {
                        put("id", id)
                        put("name", list.name)
                        put("created_at", created)
                        put("updated_at", updated)
                    },
                    SQLiteDatabase.CONFLICT_REPLACE,
                )
                db.insertWithOnConflict(
                    "desktop_playlists",
                    null,
                    ContentValues().apply { put("playlist_id", id) },
                    SQLiteDatabase.CONFLICT_REPLACE,
                )
                db.delete("playlist_tracks", "playlist_id = ?", arrayOf(id))
                var position = 0
                for (key in visibleOrder) {
                    db.insertWithOnConflict(
                        "playlist_tracks",
                        null,
                        ContentValues().apply {
                            put("playlist_id", id)
                            put("track_id", key.removePrefix("ytm:"))
                            put("position", position++)
                        },
                        SQLiteDatabase.CONFLICT_REPLACE,
                    )
                }
                for (key in incomingKeys) {
                    val existing = existingMarks[key]
                    val add = existing?.first ?: 0L
                    val remove = existing?.second ?: 0L
                    if (add == 0L && remove == 0L) {
                        upsertMark(db, id, key, add_at = now, remove_at = 0L)
                    }
                }
            }
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    private fun capMarks(map: MutableMap<String, Long>) {
        val keep = map.entries
            .sortedWith(compareByDescending<Map.Entry<String, Long>> { it.value }.thenBy { it.key })
            .take(500)
            .map { it.key }
            .toSet()
        if (keep.size < map.size) {
            map.keys.retainAll(keep)
        }
    }

    private fun upsertMark(
        db: SQLiteDatabase,
        playlistId: String,
        trackKey: String,
        add_at: Long,
        remove_at: Long,
    ) {
        db.insertWithOnConflict(
            "playlist_marks",
            null,
            ContentValues().apply {
                put("playlist_id", playlistId)
                put("track_id", trackKey)
                put("add_at", add_at)
                put("remove_at", remove_at)
            },
            SQLiteDatabase.CONFLICT_REPLACE,
        )
    }

    @Synchronized
    fun mergePlaylist(
        localId: String,
        name: String,
        createdAt: Long,
        updatedAt: Long,
        tracks: List<CatalogueTrack>,
        adds: List<SyncMark>,
        removals: List<SyncMark>,
    ) {
        val db = writableDb()
        db.beginTransaction()
        try {
            mergePlaylistInTransaction(
                db,
                localId,
                name,
                createdAt,
                updatedAt,
                tracks,
                adds,
                removals,
            )
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    private fun mergePlaylistInTransaction(
        db: SQLiteDatabase,
        localId: String,
        name: String,
        createdAt: Long,
        updatedAt: Long,
        tracks: List<CatalogueTrack>,
        adds: List<SyncMark>,
        removals: List<SyncMark>,
    ) {
        val localRow = db.rawQuery(
            "SELECT name, created_at, updated_at FROM playlists WHERE id = ?",
            arrayOf(localId),
        ).use { cursor ->
            if (cursor.moveToFirst()) {
                Triple(cursor.getString(0), cursor.getLong(1), cursor.getLong(2))
            } else {
                null
            }
        }
        val localName = localRow?.first ?: name
        val localCreated = localRow?.second ?: createdAt
        val localUpdated = localRow?.third ?: 0L
        val localFirst = localUpdated >= updatedAt

        val addMap = mutableMapOf<String, Long>()
        val remMap = mutableMapOf<String, Long>()
        for ((key, pair) in readMarks(db, localId)) {
            addMap[key] = pair.first
            remMap[key] = pair.second
        }
        for (mark in adds) {
            addMap[mark.trackId] = maxOf(addMap[mark.trackId] ?: 0L, mark.at)
        }
        for (mark in removals) {
            remMap[mark.trackId] = maxOf(remMap[mark.trackId] ?: 0L, mark.at)
        }
        capMarks(addMap)
        capMarks(remMap)
        val keptKeys = addMap.keys + remMap.keys
        for (key in readMarks(db, localId).keys) {
            if (key !in keptKeys) {
                db.delete(
                    "playlist_marks",
                    "playlist_id = ? AND track_id = ?",
                    arrayOf(localId, key),
                )
            }
        }
        val allKeys = addMap.keys + remMap.keys
        fun markVisible(key: String): Boolean =
                (addMap[key] ?: 0L) >= (remMap[key] ?: 0L)

        val remoteKeys = tracks.map(::trackKey)
        val localKeys = readTrackKeys(db, localId)
        val ordered = mutableListOf<String>()
        if (localFirst) {
            for (key in localKeys) if (key !in ordered) ordered += key
            for (key in remoteKeys) if (key !in ordered) ordered += key
        } else {
            for (key in remoteKeys) if (key !in ordered) ordered += key
            for (key in localKeys) if (key !in ordered) ordered += key
        }
        val orderedFiltered = ordered.filter(::markVisible).distinct()

        val finalName = if (localFirst) localName else name
        db.insertWithOnConflict(
            "playlists",
            null,
            ContentValues().apply {
                put("id", localId)
                put("name", finalName.ifBlank { "Playlist" })
                put("created_at", minOf(localCreated, createdAt))
                put("updated_at", maxOf(localUpdated, updatedAt))
            },
            SQLiteDatabase.CONFLICT_REPLACE,
        )
        for (track in tracks) {
            if (track.id.isBlank()) continue
            upsertTrackDisplay(db, track)
        }
        db.delete("playlist_tracks", "playlist_id = ?", arrayOf(localId))
        var position = 0
        for (key in orderedFiltered) {
            db.insertWithOnConflict(
                "playlist_tracks",
                null,
                ContentValues().apply {
                    put("playlist_id", localId)
                    put("track_id", key.removePrefix("ytm:"))
                    put("position", position++)
                },
                SQLiteDatabase.CONFLICT_REPLACE,
            )
        }
        for (key in allKeys) {
            upsertMark(
                db,
                localId,
                key,
                add_at = addMap[key] ?: 0L,
                remove_at = remMap[key] ?: 0L,
            )
        }
    }

    private fun upsertTrackDisplay(db: SQLiteDatabase, track: CatalogueTrack) {
        val exists = db.rawQuery("SELECT 1 FROM tracks WHERE id = ?", arrayOf(track.id)).use {
            it.moveToFirst()
        }
        val values = ContentValues().apply {
            put("title", track.title)
            put("artist", track.artist)
            put("album", track.album)
            put("duration", track.duration)
            put("cover", track.cover)
        }
        if (exists) {
            db.update("tracks", values, "id = ?", arrayOf(track.id))
        } else {
            values.put("id", track.id)
            values.put("ext", "m4a")
            values.put("mtime", 0L)
            values.put("lyrics", "")
            values.put("added_at", System.currentTimeMillis())
            db.insertWithOnConflict("tracks", null, values, SQLiteDatabase.CONFLICT_REPLACE)
        }
    }

    @Synchronized
    fun findEquivalentPlaylist(name: String, trackIds: Set<String>): LocalPlaylist? {
        val wanted = name.trim().lowercase()
        var best: LocalPlaylist? = null
        readableDb().rawQuery(
            "SELECT p.id, p.name, p.created_at, p.updated_at, (SELECT COUNT(*) FROM playlist_tracks t WHERE t.playlist_id = p.id) AS cnt FROM playlists p",
            null,
        ).use { cursor ->
            while (cursor.moveToNext()) {
                val id = cursor.getString(0)
                val name = cursor.getString(1)
                if (!name.trim().lowercase().equals(wanted)) continue
                if (id == "liked" || isDesktopPlaylist(id)) continue
                val ids = mutableSetOf<String>()
                readableDb().rawQuery(
                    "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?",
                    arrayOf(id),
                ).use { inner ->
                    while (inner.moveToNext()) ids += inner.getString(0)
                }
                val compatible = ids.isEmpty() || trackIds.isEmpty() || ids.intersect(trackIds).isNotEmpty()
                if (compatible) {
                    val candidate = LocalPlaylist(
                        id = id,
                        name = name,
                        createdAt = cursor.getLong(2),
                        updatedAt = cursor.getLong(3),
                        trackCount = cursor.getInt(4),
                    )
                    val current = best
                    if (current == null || candidate.updatedAt > current.updatedAt) {
                        best = candidate
                    }
                }
            }
        }
        return best
    }

    @Synchronized
    fun renamePlaylistId(oldId: String, newId: String) {
        val db = writableDb()
        db.beginTransaction()
        try {
            db.update(
                "playlists",
                ContentValues().apply { put("id", newId) },
                "id = ?",
                arrayOf(oldId),
            )
            db.update(
                "playlist_tracks",
                ContentValues().apply { put("playlist_id", newId) },
                "playlist_id = ?",
                arrayOf(oldId),
            )
            db.update(
                "playlist_marks",
                ContentValues().apply { put("playlist_id", newId) },
                "playlist_id = ?",
                arrayOf(oldId),
            )
            db.update(
                "desktop_playlists",
                ContentValues().apply { put("playlist_id", newId) },
                "playlist_id = ?",
                arrayOf(oldId),
            )
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    @Synchronized
    fun snapshot(id: String): PlaylistSnapshot? {
        val db = writableDb()
        db.beginTransaction()
        try {
            val pl = db.rawQuery(
                """
                SELECT p.id, p.name, p.created_at, p.updated_at,
                       (SELECT COUNT(*) FROM playlist_tracks t WHERE t.playlist_id = p.id)
                FROM playlists p WHERE p.id = ?
                """.trimIndent(),
                arrayOf(id),
            ).use { cursor ->
                if (!cursor.moveToNext()) {
                    null
                } else {
                    val name = cursor.getString(1)
                    LocalPlaylist(
                        id = cursor.getString(0),
                        name = name,
                        createdAt = cursor.getLong(2),
                        updatedAt = cursor.getLong(3),
                        trackCount = cursor.getInt(4),
                    )
                }
            } ?: run {
                db.endTransaction()
                return null
            }
            val marks = readMarks(db, id)
            val ordered = readTrackKeys(db, id)
            db.setTransactionSuccessful()
            return PlaylistSnapshot(
                name = pl.name,
                createdAt = pl.createdAt,
                updatedAt = pl.updatedAt,
                orderedTrackIds = ordered,
                marks = marks,
            )
        } finally {
            db.endTransaction()
        }
    }

    @Synchronized
    fun addToPlaylist(playlistId: String, track: CatalogueTrack) {
        addTrack(track)
        val db = writableDb()
        db.beginTransaction()
        try {
            val position = db.rawQuery(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?",
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
            val now = System.currentTimeMillis()
            val existing = readMarks(db, playlistId)["ytm:${track.id}"]
            upsertMark(
                db,
                playlistId,
                "ytm:${track.id}",
                add_at = maxOf(now, existing?.first ?: 0L),
                remove_at = existing?.second ?: 0L,
            )
            touchPlaylist(playlistId)
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    @Synchronized
    fun removeFromPlaylist(playlistId: String, trackId: String) {
        val db = writableDb()
        db.beginTransaction()
        try {
            db.delete(
                "playlist_tracks",
                "playlist_id = ? AND track_id = ?",
                arrayOf(playlistId, trackId),
            )
            val now = System.currentTimeMillis()
            val existing = readMarks(db, playlistId)["ytm:$trackId"]
            upsertMark(
                db,
                playlistId,
                "ytm:$trackId",
                add_at = existing?.first ?: 0L,
                remove_at = maxOf(now, existing?.second ?: 0L),
            )
            touchPlaylist(playlistId)
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    @Synchronized
    fun deleteTrack(trackId: String): List<String> {
        val db = writableDb()
        val affected = mutableListOf<String>()
        db.beginTransaction()
        try {
            db.rawQuery(
                "SELECT DISTINCT playlist_id FROM playlist_tracks WHERE track_id = ?",
                arrayOf(trackId),
            ).use { cursor ->
                while (cursor.moveToNext()) {
                    affected += cursor.getString(0)
                }
            }
            val now = System.currentTimeMillis()
            for (id in affected) {
                val existing = readMarks(db, id)["ytm:$trackId"]
                upsertMark(
                    db,
                    id,
                    "ytm:$trackId",
                    add_at = existing?.first ?: 0L,
                    remove_at = maxOf(now, existing?.second ?: 0L),
                )
            }
            db.delete("playlist_tracks", "track_id = ?", arrayOf(trackId))
            db.delete("tracks", "id = ?", arrayOf(trackId))
            for (id in affected) {
                touchPlaylist(id)
            }
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
        return affected
    }

    fun playlistTracks(playlistId: String): List<LocalTrack> {
        val result = mutableListOf<LocalTrack>()
        readableDb().rawQuery(
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
        readableDb().rawQuery(
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

    @Synchronized
    fun syncPayload(id: String): SyncPlaylistPayload? {
        val playlist = playlist(id) ?: return null
        val db = readableDb()
        val marks = readMarks(db, id)
        return SyncPlaylistPayload(
            localId = id,
            name = playlist.name,
            createdAt = playlist.createdAt,
            updatedAt = playlist.updatedAt.takeIf { it > 0 } ?: playlist.createdAt,
            tracks = playlistTracks(id).map { it.toCatalogue().copy(cover = it.cover) },
            adds = marks.map { (key, pair) -> SyncMark(key, pair.first) },
            removals = marks.map { (key, pair) -> SyncMark(key, pair.second) },
        )
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

data class SyncPlaylistPayload(
    val localId: String,
    val name: String,
    val createdAt: Long,
    val updatedAt: Long,
    val tracks: List<CatalogueTrack>,
    val adds: List<SyncMark> = emptyList(),
    val removals: List<SyncMark> = emptyList(),
)

data class SyncMark(
    val trackId: String,
    val at: Long,
)

data class PlaylistSnapshot(
    val name: String,
    val createdAt: Long,
    val updatedAt: Long,
    val orderedTrackIds: List<String>,
    val marks: Map<String, Pair<Long, Long>>,
)
