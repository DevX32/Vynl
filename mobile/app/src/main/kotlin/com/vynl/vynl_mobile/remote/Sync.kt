package com.vynl.vynl_mobile.remote

import android.content.Context
import com.vynl.vynl_mobile.BuildConfig
import com.vynl.vynl_mobile.data.CatalogueTrack
import com.vynl.vynl_mobile.data.SyncPlaylistPayload
import com.vynl.vynl_mobile.data.SyncMark
import com.vynl.vynl_mobile.data.Store
import com.vynl.vynl_mobile.data.highResCover
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.net.URLEncoder
import java.util.concurrent.TimeUnit

data class SyncStatus(
    val signedIn: Boolean,
    val email: String?,
)

data class SyncAuthResult(
    val needsConfirmation: Boolean,
    val status: SyncStatus,
)

data class SyncResult(
    val pushed: Int,
    val pulled: Int,
    val deleted: Int,
    val failed: Int = 0,
)

private data class FoldResult(
    val rows: List<JSONObject>,
    val staleIds: List<String>,
)

private data class SyncSession(
    val email: String,
    val userId: String,
    val accessToken: String,
    val refreshToken: String,
    val expiresAt: Long,
)

class Sync(context: Context) {
    private val prefs = context.getSharedPreferences("vynl_cloud", Context.MODE_PRIVATE)
    private val http = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(20, TimeUnit.SECONDS)
        .writeTimeout(20, TimeUnit.SECONDS)
        .build()
    private val refreshLock = Any()

    private val baseUrl = BuildConfig.SUPABASE_URL.trimEnd('/')
    private val anonKey = BuildConfig.SUPABASE_ANON_KEY

    val status: SyncStatus
        get() {
            val session = session() ?: return SyncStatus(false, null)
            return SyncStatus(signedIn = true, email = session.email)
        }

    suspend fun signIn(email: String, password: String): SyncAuthResult =
        withContext(Dispatchers.IO) {
            val clean = email.trim().lowercase()
            if (!clean.contains('@')) throw IOException("Enter a valid email address.")
            if (password.isEmpty()) throw IOException("Enter your password.")
            val body = JSONObject()
                .put("email", clean)
                .put("password", password)
            val response = post("$baseUrl/auth/v1/token?grant_type=password", body, token = null)
            val session = parseSession(response.second, clean)
                ?: throw IOException("Vynl Sync returned an empty session.")
            saveSession(session)
            SyncAuthResult(needsConfirmation = false, status = status)
        }

    suspend fun signUp(email: String, password: String): SyncAuthResult =
        withContext(Dispatchers.IO) {
            val clean = email.trim().lowercase()
            if (!clean.contains('@')) throw IOException("Enter a valid email address.")
            if (password.length < 6) throw IOException("Use at least 6 characters for the password.")
            val body = JSONObject()
                .put("email", clean)
                .put("password", password)
            val response = post("$baseUrl/auth/v1/signup", body, token = null)
            val session = parseSession(response.second, clean)
            if (session == null) {
                SyncAuthResult(needsConfirmation = true, status = status)
            } else {
                saveSession(session)
                SyncAuthResult(needsConfirmation = false, status = status)
            }
        }

    suspend fun signOut() = withContext(Dispatchers.IO) {
        val session = session()
        if (session != null) {
            runCatching {
                val request = Request.Builder()
                    .url("$baseUrl/auth/v1/logout")
                    .header("apikey", anonKey)
                    .header("Authorization", "Bearer ${session.accessToken}")
                    .post("{}".toRequestBody(JSON))
                    .build()
                http.newCall(request).execute().close()
            }
        }
        clearSession()
    }

    suspend fun pushPlaylist(
        payload: SyncPlaylistPayload,
        includeDeletedAt: Boolean = false,
    ) = withContext(Dispatchers.IO) {
        val session = validSession()
        pushPlaylist(session, payload, includeDeletedAt)
    }

    private fun pushPlaylist(
        session: SyncSession,
        payload: SyncPlaylistPayload,
        includeDeletedAt: Boolean = false,
    ) {
        val body = playlistJson(session, payload, marksSupported, includeDeletedAt)
        val request = Request.Builder()
            .url("$baseUrl/rest/v1/playlists?on_conflict=user_id,local_id")
            .header("apikey", anonKey)
            .header("Authorization", "Bearer ${session.accessToken}")
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates,return=minimal")
            .post(JSONArray().put(body).toString().toRequestBody(JSON))
            .build()
        try {
            execute(request, "Could not save to Vynl Sync")
        } catch (error: IOException) {
            if (marksSupported && bodyMissingMarks(error.message.orEmpty())) {
                marksSupported = false
                val stripped = playlistJson(session, payload, false, includeDeletedAt)
                val retry = Request.Builder()
                    .url("$baseUrl/rest/v1/playlists?on_conflict=user_id,local_id")
                    .header("apikey", anonKey)
                    .header("Authorization", "Bearer ${session.accessToken}")
                    .header("Content-Type", "application/json")
                    .header("Prefer", "resolution=merge-duplicates,return=minimal")
                    .post(JSONArray().put(stripped).toString().toRequestBody(JSON))
                    .build()
                execute(retry, "Could not save to Vynl Sync")
                return
            }
            throw error
        }
    }

    suspend fun tombstone(localId: String) = withContext(Dispatchers.IO) {
        val session = validSession()
        val url = "$baseUrl/rest/v1/playlists?user_id=eq.${enc(session.userId)}&local_id=eq.${enc(localId)}"
        val request = Request.Builder()
            .url(url)
            .header("apikey", anonKey)
            .header("Authorization", "Bearer ${session.accessToken}")
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal")
            .patch(
                JSONObject().put("deleted_at", System.currentTimeMillis()).toString().toRequestBody(JSON),
            )
            .build()
        execute(request, "Could not update Vynl Sync")
    }

    @Volatile
    private var marksSupported = true

    suspend fun refresh(store: Store): SyncResult = withContext(Dispatchers.IO) {
        store.rekeyLegacyDtPlaylists()
        val session = validSession()
        val folded = foldRows(fetchRows(session))
        val rows = folded.rows
        val remoteOrder = fetchOrder(session)
        if (remoteOrder != null) {
            val remoteUpdated = remoteOrder.optLong("updated_at", 0L)
            if (remoteUpdated > store.storedCustomOrderUpdatedAt()) {
                val ids = remoteOrder
                    .optJSONArray("order_ids")
                    ?.let { array ->
                        (0 until array.length())
                            .map { array.optString(it) }
                            .filter { it.isNotBlank() }
                    }
                    .orEmpty()
                if (ids.isNotEmpty()) {
                    store.setCustomOrder(ids, remoteUpdated)
                }
            }
        }
        var pushed = 0
        var pulled = 0
        var deleted = 0
        var failed = 0
        var consumed = mutableSetOf<String>()
        val known = mutableSetOf<String>()
        // Folded-out stale duplicates must not be pushed back as new rows.
        known += folded.staleIds

        for (row in rows) {
            val canonical = row.optString("local_id")
            if (canonical.isBlank()) continue
            known += canonical
            val deletedAt = if (row.isNull("deleted_at")) null else row.optLong("deleted_at")

            if (deletedAt != null) {
                // Strict resolution: a tombstoned row may only delete a
                // playlist that exactly matches its own local_id. Name
                // matching here could rekey and delete a merged, live one.
                val strictLocal = store.playlist(canonical)
                when {
                    strictLocal == null -> {}
                    strictLocal.updatedAt <= deletedAt -> {
                        store.deletePlaylist(canonical)
                        deleted += 1
                    }
                    else -> {
                        val payload = store.syncPayload(canonical)
                        val ok = payload != null &&
                            runCatching { pushPlaylist(payload, includeDeletedAt = true) }.isSuccess
                        if (ok) pushed += 1 else failed += 1
                    }
                }
                continue
            }

            val localId = resolveLocalId(store, canonical, row, consumed)

            val remoteUpdated = row.optLong("updated_at")
            val remoteName = row.optString("name", "Playlist")
            val remoteTracks = parseTracks(row)
            val remoteAdds = parseMarks(row, "adds")
            val remoteRemovals = parseMarks(row, "removals")
            val before = store.snapshot(localId)
            store.mergePlaylist(
                localId = localId,
                name = remoteName,
                createdAt = row.optLong("created_at"),
                updatedAt = remoteUpdated,
                tracks = remoteTracks,
                adds = remoteAdds,
                removals = remoteRemovals,
            )
            val after = store.snapshot(localId)
            val payload = store.syncPayload(localId)
            val ok = payload != null && runCatching { pushPlaylist(payload) }.isSuccess
            if (ok) {
                pushed += 1
            } else {
                failed += 1
            }
            if (before != after) {
                pulled += 1
            }
        }

        for (playlist in store.playlists()) {
            val payload = store.syncPayload(playlist.id) ?: continue
            if (known.contains(payload.localId)) continue
            val ok = runCatching { pushPlaylist(payload) }.isSuccess
            if (ok) pushed += 1 else failed += 1
        }

        SyncResult(pushed = pushed, pulled = pulled, deleted = deleted, failed = failed)
    }

    private fun resolveLocalId(
        store: Store,
        canonical: String,
        row: JSONObject,
        consumed: MutableSet<String>,
    ): String {
        if (store.playlist(canonical) != null) return canonical
        val trackIds = mutableSetOf<String>()
        val array = row.optJSONArray("tracks") ?: JSONArray()
        for (index in 0 until array.length()) {
            array.optJSONObject(index)?.optString("id")?.takeIf { it.isNotBlank() }?.let(trackIds::add)
        }
        val match = store.findEquivalentPlaylist(row.optString("name", ""), trackIds)
        if (match != null && match.id !in consumed) {
            consumed += match.id
            store.renamePlaylistId(match.id, canonical)
            return canonical
        }
        return canonical
    }

    private fun fetchRows(session: SyncSession): List<JSONObject> {
        val request = Request.Builder()
            .url(
                "$baseUrl/rest/v1/playlists?select=local_id,name,tracks,created_at,updated_at,deleted_at" +
                    (if (marksSupported) ",adds,removals" else ""),
            )
            .header("apikey", anonKey)
            .header("Authorization", "Bearer ${session.accessToken}")
            .header("Accept", "application/json")
            .get()
            .build()
        val response = http.newCall(request).execute()
        response.use {
            val body = it.body?.string().orEmpty()
            if (!it.isSuccessful) {
                if (marksSupported && bodyMissingMarks(body)) {
                    marksSupported = false
                    return fetchRows(session)
                }
                throw IOException(
                    "Could not load synced playlists (${it.code}): ${body.trim().take(160)}",
                )
            }
            val array = runCatching { JSONArray(body) }.getOrNull() ?: JSONArray()
            return (0 until array.length()).mapNotNull { index ->
                array.optJSONObject(index)
            }
        }
    }

    private fun fetchOrder(session: SyncSession): org.json.JSONObject? {
        val request = Request.Builder()
            .url("$baseUrl/rest/v1/library_order?select=order_ids,updated_at&limit=1")
            .header("apikey", anonKey)
            .header("Authorization", "Bearer ${session.accessToken}")
            .header("Accept", "application/json")
            .get()
            .build()
        return runCatching {
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful) return@runCatching null
                val body = response.body?.string().orEmpty()
                val array = org.json.JSONArray(body)
                if (array.length() == 0) return@runCatching null
                array.optJSONObject(0)
            }
        }.getOrNull()
    }

    private fun foldRows(rows: List<JSONObject>): FoldResult {
        fun nameKey(raw: String?): String =
            raw.orEmpty()
                .split(Regex("\\s+"))
                .filter { it.isNotBlank() }
                .joinToString(" ")
                .lowercase()

        fun signatureOf(row: JSONObject): Set<String> {
            val array = row.optJSONArray("tracks") ?: return emptySet()
            val out = LinkedHashSet<String>()
            for (index in 0 until array.length()) {
                val id = array.optJSONObject(index)?.optString("id").orEmpty().trim()
                if (id.isNotEmpty()) out += id
            }
            return out
        }

        fun mergeMarkLists(lists: List<List<SyncMark>>): List<SyncMark> {
            val best = HashMap<String, Long>()
            for (list in lists) {
                for (mark in list) {
                    val existing = best[mark.trackId]
                    if (existing == null || mark.at > existing) {
                        best[mark.trackId] = mark.at
                    }
                }
            }
            return best.entries.map { SyncMark(it.key, it.value) }
        }

        fun marksArray(marks: List<SyncMark>): JSONArray {
            val array = JSONArray()
            marks.sortedWith(compareByDescending<SyncMark> { it.at }.thenBy { it.trackId })
                .take(10_000)
                .forEach { mark ->
                    array.put(
                        JSONObject().apply {
                            put("trackId", mark.trackId)
                            put("at", mark.at)
                        },
                    )
                }
            return array
        }

        val groups = LinkedHashMap<String, MutableList<JSONObject>>()
        for (row in rows) {
            if (!row.isNull("deleted_at")) continue
            val key = nameKey(row.optString("name"))
            if (key.isEmpty()) continue
            groups.getOrPut(key) { mutableListOf() } += row
        }

        val consumed = HashSet<JSONObject>()
        val syntheticByCanonical = HashMap<JSONObject, JSONObject>()
        for (group in groups.values) {
            if (group.size < 2) continue
            val ordered = group.sortedWith(
                compareBy<JSONObject> { it.optLong("created_at") }.thenBy { it.optString("local_id") },
            )
            val canonical = ordered[0]
            val canonicalSignature = signatureOf(canonical)
            val members = ordered.drop(1).filter { row ->
                val signature = signatureOf(row)
                canonicalSignature.isEmpty() || signature.isEmpty() ||
                    canonicalSignature.intersect(signature).isNotEmpty()
            }
            if (members.isEmpty()) continue

            val merged = JSONObject()
            merged.put("local_id", canonical.optString("local_id"))
            merged.put("name", canonical.optString("name"))
            merged.put("created_at", canonical.optLong("created_at"))
            merged.put("updated_at", members.plus(canonical).maxOf { it.optLong("updated_at") })
            merged.put("deleted_at", JSONObject.NULL)

            val seen = HashSet<String>()
            val tracks = JSONArray()
            for (row in listOf(canonical) + members) {
                val array = row.optJSONArray("tracks") ?: continue
                for (index in 0 until array.length()) {
                    val track = array.optJSONObject(index) ?: continue
                    val id = track.optString("id").trim()
                    if (id.isEmpty() || !seen.add(id)) continue
                    tracks.put(track)
                }
            }
            merged.put("tracks", tracks)
            val adds = mergeMarkLists((listOf(canonical) + members).map { parseMarks(it, "adds") })
            val removals = mergeMarkLists((listOf(canonical) + members).map { parseMarks(it, "removals") })
            merged.put("adds", marksArray(adds))
            merged.put("removals", marksArray(removals))

            for (row in members) consumed += row
            syntheticByCanonical[canonical] = merged
        }

        if (consumed.isEmpty()) return FoldResult(rows, emptyList())
        val staleIds = consumed.map { it.optString("local_id") }.filter { it.isNotBlank() }
        val out = ArrayList<JSONObject>(rows.size)
        for (row in rows) {
            if (row in consumed) continue
            out += syntheticByCanonical[row] ?: row
        }
        return FoldResult(out, staleIds)
    }

    private fun bodyMissingMarks(body: String): Boolean {
        val lowered = body.lowercase()
        val mentionsMark = lowered.contains("adds") || lowered.contains("removals")
        val schemaError = lowered.contains("column") || lowered.contains("schema cache")
        return mentionsMark && schemaError
    }

    private fun parseMarks(row: JSONObject, key: String): List<SyncMark> {
        val array = row.optJSONArray(key) ?: return emptyList()
        return (0 until array.length()).mapNotNull { index ->
            val mark = array.optJSONObject(index) ?: return@mapNotNull null
            val trackId = mark.optString("trackId").ifBlank { mark.optString("track_id") }
            if (trackId.isBlank() || !trackId.startsWith("ytm:")) return@mapNotNull null
            SyncMark(trackId, mark.optLong("at"))
        }
    }

    private fun playlistJson(
        session: SyncSession,
        payload: SyncPlaylistPayload,
        includeMarks: Boolean = true,
        includeDeletedAt: Boolean = false,
    ): JSONObject =
        JSONObject().apply {
            put("user_id", session.userId)
            put("local_id", payload.localId)
            put("name", payload.name)
            put(
                "tracks",
                JSONArray().apply {
                    for (track in payload.tracks) {
                        put(
                            JSONObject().apply {
                                put("id", track.id)
                                put("title", track.title)
                                put("artist", track.artist)
                                put("album", track.album ?: JSONObject.NULL)
                                put("duration", track.duration)
                                put("cover", highResCover(track.cover) ?: JSONObject.NULL)
                            },
                        )
                    }
                },
            )
            put("created_at", payload.createdAt)
            put("updated_at", payload.updatedAt)
            if (includeDeletedAt) {
                put("deleted_at", JSONObject.NULL)
            }
            if (includeMarks) {
                put("adds", marksJson(payload.adds))
                put("removals", marksJson(payload.removals))
            }
        }

    private fun marksJson(marks: List<SyncMark>): JSONArray =
        JSONArray().apply {
            val filtered = marks
                .filter { it.at > 0L && it.trackId.startsWith("ytm:") }
                .sortedWith(compareByDescending<SyncMark> { it.at }.thenBy { it.trackId })
                .take(10_000)
            for (mark in filtered) {
                put(
                    JSONObject().apply {
                        put("trackId", mark.trackId)
                        put("at", mark.at)
                    },
                )
            }
        }

    private fun parseTracks(row: JSONObject): List<CatalogueTrack> {
        val array = row.optJSONArray("tracks") ?: return emptyList()
        return (0 until array.length()).mapNotNull { index ->
            val track = array.optJSONObject(index) ?: return@mapNotNull null
            val id = track.optString("id")
            if (id.isBlank()) return@mapNotNull null
            CatalogueTrack(
                id = id,
                title = track.optString("title", "Unknown"),
                artist = track.optString("artist", ""),
                album = track.optString("album").takeIf { it.isNotBlank() },
                duration = track.optDouble("duration", 0.0),
                cover = highResCover(track.optString("cover").takeIf { it.isNotBlank() }),
                explicit = false,
            )
        }
    }

    private fun post(
        url: String,
        body: JSONObject,
        token: String?,
    ): Pair<Int, JSONObject> {
        val builder = Request.Builder()
            .url(url)
            .header("apikey", anonKey)
            .header("Content-Type", "application/json")
        if (token != null) builder.header("Authorization", "Bearer $token")
        val request = builder.post(body.toString().toRequestBody(JSON)).build()
        return try {
            http.newCall(request).execute().use { response ->
                val raw = response.body?.string().orEmpty()
                val json = runCatching { JSONObject(raw) }.getOrElse { JSONObject() }
                if (!response.isSuccessful) throw IOException(errorMessage(response.code, json, raw))
                response.code to json
            }
        } catch (error: IOException) {
            throw error
        } catch (error: Exception) {
            throw IOException("Could not reach Vynl Sync: ${error.message}")
        }
    }

    private fun execute(request: Request, fallback: String) {
        http.newCall(request).execute().use { response ->
            if (!response.isSuccessful) {
                val raw = response.body?.string().orEmpty()
                val json = runCatching { JSONObject(raw) }.getOrElse { JSONObject() }
                throw IOException(errorMessage(response.code, json, raw, fallback))
            }
        }
    }

    private fun errorMessage(
        code: Int,
        json: JSONObject,
        raw: String,
        fallback: String = "Sign-in failed",
    ): String {
        val errorCode = json.optString("error_code").ifBlank { json.optString("code") }
        return when (errorCode) {
            "invalid_credentials" -> "Wrong email or password."
            "email_not_confirmed" -> "Confirm your email first, then sign in."
            "user_already_exists", "email_exists" -> "That email is already registered."
            "weak_password" -> "That password is too weak."
            "over_email_send_rate_limit" -> "Too many emails sent — wait a minute and try again."
            "same_password" -> "Pick a different password."
            else -> {
                val message = json.optString("msg").ifBlank { json.optString("message") }
                when {
                    message.isNotBlank() -> message
                    code >= 500 -> "Vynl Sync had a problem ($code). Try again later."
                    else -> "$fallback ($code). ${raw.trim().take(120)}".trim()
                }
            }
        }
    }

    private fun parseSession(json: JSONObject, emailHint: String): SyncSession? {
        val accessToken = json.optString("access_token")
        if (accessToken.isBlank()) return null
        val refreshToken = json.optString("refresh_token")
        val expiresIn = json.optLong("expires_in", 3600)
        val user = json.optJSONObject("user") ?: return null
        val userId = user.optString("id")
        if (userId.isBlank()) return null
        val email = user.optString("email").ifBlank { emailHint }
        return SyncSession(
            email = email,
            userId = userId,
            accessToken = accessToken,
            refreshToken = refreshToken,
            expiresAt = System.currentTimeMillis() + expiresIn * 1000,
        )
    }

    private fun session(): SyncSession? {
        val token = prefs.getString(KEY_ACCESS, null) ?: return null
        val userId = prefs.getString(KEY_USER, null) ?: return null
        if (token.isBlank() || userId.isBlank()) return null
        return SyncSession(
            email = prefs.getString(KEY_EMAIL, "") ?: "",
            userId = userId,
            accessToken = token,
            refreshToken = prefs.getString(KEY_REFRESH, "") ?: "",
            expiresAt = prefs.getLong(KEY_EXPIRES, 0),
        )
    }

    private fun saveSession(session: SyncSession) {
        prefs.edit()
            .putString(KEY_EMAIL, session.email)
            .putString(KEY_USER, session.userId)
            .putString(KEY_ACCESS, session.accessToken)
            .putString(KEY_REFRESH, session.refreshToken)
            .putLong(KEY_EXPIRES, session.expiresAt)
            .apply()
    }

    private fun clearSession() {
        prefs.edit().clear().apply()
    }

    private fun postRaw(path: String, body: JSONObject): Pair<Int, JSONObject> {
        val request = Request.Builder()
            .url(path)
            .header("apikey", anonKey)
            .header("Content-Type", "application/json; charset=utf-8")
            .post(body.toString().toRequestBody(JSON))
            .build()
        http.newCall(request).execute().use { response ->
            val raw = response.body?.string().orEmpty()
            val json = runCatching { JSONObject(raw) }.getOrElse { JSONObject() }
            return response.code to json
        }
    }

    private fun validSession(): SyncSession {
        val current = session() ?: throw IOException("Sign in to Vynl first.")
        if (current.expiresAt > System.currentTimeMillis() + 30_000) return current
        synchronized(refreshLock) {
            val again = session() ?: throw IOException("Sign in to Vynl first.")
            if (again.expiresAt > System.currentTimeMillis() + 30_000) return again
            if (again.refreshToken.isBlank()) {
                clearSession()
                throw IOException("Your session expired. Sign in again.")
            }
            val body = JSONObject().put("refresh_token", again.refreshToken)
            val (code, json) = try {
                postRaw("$baseUrl/auth/v1/token?grant_type=refresh_token", body)
            } catch (error: IOException) {
                throw IOException("Couldn't reach Vynl Sync — check your connection.")
            }
            if (code in 200..299) {
                val fresh = parseSession(json, again.email) ?: run {
                    clearSession()
                    throw IOException("Your session expired. Sign in again.")
                }
                val withRefresh = if (fresh.refreshToken.isBlank()) {
                    fresh.copy(refreshToken = again.refreshToken)
                } else {
                    fresh
                }
                saveSession(withRefresh)
                return withRefresh
            }
            if (code == 400 || code == 401 || code == 403) {
                clearSession()
                throw IOException("Your session expired. Sign in again.")
            }
            throw IOException("Vynl Sync had a problem ($code). Try again later.")
        }
    }

    private fun enc(value: String): String = URLEncoder.encode(value, "UTF-8")

    companion object {
        private const val KEY_EMAIL = "email"
        private const val KEY_USER = "user_id"
        private const val KEY_ACCESS = "access_token"
        private const val KEY_REFRESH = "refresh_token"
        private const val KEY_EXPIRES = "expires_at"
        private val JSON = "application/json; charset=utf-8".toMediaType()
    }
}
