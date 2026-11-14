package com.vynl.vynl_mobile.remote

import android.content.Context
import com.vynl.vynl_mobile.BuildConfig
import com.vynl.vynl_mobile.data.CatalogueTrack
import com.vynl.vynl_mobile.data.CloudPlaylistPayload
import com.vynl.vynl_mobile.data.Store
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

data class CloudStatus(
    val signedIn: Boolean,
    val email: String?,
)

data class CloudAuthResult(
    val needsConfirmation: Boolean,
    val status: CloudStatus,
)

data class CloudSyncResult(
    val pushed: Int,
    val pulled: Int,
    val deleted: Int,
)

private data class CloudSession(
    val email: String,
    val userId: String,
    val accessToken: String,
    val refreshToken: String,
    val expiresAt: Long,
)

class Cloud(context: Context) {
    private val prefs = context.getSharedPreferences("vynl_cloud", Context.MODE_PRIVATE)
    private val http = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(20, TimeUnit.SECONDS)
        .writeTimeout(20, TimeUnit.SECONDS)
        .build()
    private val refreshLock = Any()

    private val baseUrl = BuildConfig.SUPABASE_URL.trimEnd('/')
    private val anonKey = BuildConfig.SUPABASE_ANON_KEY

    val status: CloudStatus
        get() {
            val session = session() ?: return CloudStatus(false, null)
            return CloudStatus(signedIn = true, email = session.email)
        }

    suspend fun signIn(email: String, password: String): CloudAuthResult =
        withContext(Dispatchers.IO) {
            val clean = email.trim().lowercase()
            if (!clean.contains('@')) throw IOException("Enter a valid email address.")
            if (password.isEmpty()) throw IOException("Enter your password.")
            val body = JSONObject()
                .put("email", clean)
                .put("password", password)
            val response = post("$baseUrl/auth/v1/token?grant_type=password", body, token = null)
            val session = parseSession(response.second, clean)
                ?: throw IOException("The cloud returned an empty session.")
            saveSession(session)
            CloudAuthResult(needsConfirmation = false, status = status)
        }

    suspend fun signUp(email: String, password: String): CloudAuthResult =
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
                CloudAuthResult(needsConfirmation = true, status = status)
            } else {
                saveSession(session)
                CloudAuthResult(needsConfirmation = false, status = status)
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

    suspend fun pushPlaylist(payload: CloudPlaylistPayload) = withContext(Dispatchers.IO) {
        val session = validSession()
        val body = JSONArray().put(playlistJson(session, payload))
        val request = Request.Builder()
            .url("$baseUrl/rest/v1/playlists?on_conflict=user_id,local_id")
            .header("apikey", anonKey)
            .header("Authorization", "Bearer ${session.accessToken}")
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates,return=minimal")
            .post(body.toString().toRequestBody(JSON))
            .build()
        execute(request, "Could not save to the cloud")
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
        execute(request, "Could not update the cloud")
    }

    suspend fun sync(store: Store): CloudSyncResult = withContext(Dispatchers.IO) {
        val session = validSession()
        val rows = fetchRows(session)
        var pushed = 0
        var pulled = 0
        var deleted = 0
        val known = mutableSetOf<String>()

        for (row in rows) {
            val canonical = row.optString("local_id")
            if (canonical.isBlank()) continue
            known += canonical
            val deletedAt = if (row.isNull("deleted_at")) null else row.optLong("deleted_at")
            val localId = store.localIdForCloud(canonical)
            val local = store.playlist(localId)

            if (deletedAt != null) {
                when {
                    local == null -> {}
                    local.updatedAt <= deletedAt -> {
                        store.deletePlaylist(localId)
                        deleted += 1
                    }
                    else -> {
                        store.cloudPayload(localId)?.let { pushPlaylist(it) }
                        pushed += 1
                    }
                }
                continue
            }

            val cloudUpdated = row.optLong("updated_at")
            when {
                local == null -> {
                    store.upsertCloudPlaylist(
                        localId = canonical,
                        name = row.optString("name", "Playlist"),
                        createdAt = row.optLong("created_at"),
                        updatedAt = cloudUpdated,
                        tracks = parseTracks(row),
                    )
                    pulled += 1
                }
                local.updatedAt < cloudUpdated -> {
                    store.upsertCloudPlaylist(
                        localId = localId,
                        name = row.optString("name", local.name),
                        createdAt = row.optLong("created_at", local.createdAt),
                        updatedAt = cloudUpdated,
                        tracks = parseTracks(row),
                    )
                    pulled += 1
                }
                local.updatedAt > cloudUpdated -> {
                    store.cloudPayload(localId)?.let { pushPlaylist(it) }
                    pushed += 1
                }
            }
        }

        for (playlist in store.playlists()) {
            val payload = store.cloudPayload(playlist.id) ?: continue
            if (known.contains(payload.localId)) continue
            pushPlaylist(payload)
            pushed += 1
        }

        CloudSyncResult(pushed = pushed, pulled = pulled, deleted = deleted)
    }

    private fun fetchRows(session: CloudSession): List<JSONObject> {
        val request = Request.Builder()
            .url(
                "$baseUrl/rest/v1/playlists" +
                    "?select=local_id,name,tracks,created_at,updated_at,deleted_at",
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
                throw IOException(
                    "Could not load cloud playlists (${it.code}): ${body.trim().take(160)}",
                )
            }
            val array = runCatching { JSONArray(body) }.getOrNull() ?: JSONArray()
            return (0 until array.length()).mapNotNull { index ->
                array.optJSONObject(index)
            }
        }
    }

    private fun playlistJson(session: CloudSession, payload: CloudPlaylistPayload): JSONObject =
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
                                put("cover", track.cover ?: JSONObject.NULL)
                            },
                        )
                    }
                },
            )
            put("created_at", payload.createdAt)
            put("updated_at", payload.updatedAt)
            put("deleted_at", JSONObject.NULL)
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
                cover = track.optString("cover").takeIf { it.isNotBlank() },
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
            throw IOException("Could not reach the cloud: ${error.message}")
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
                    code >= 500 -> "The cloud had a problem ($code). Try again later."
                    else -> "$fallback ($code). ${raw.trim().take(120)}".trim()
                }
            }
        }
    }

    private fun parseSession(json: JSONObject, emailHint: String): CloudSession? {
        val accessToken = json.optString("access_token")
        if (accessToken.isBlank()) return null
        val refreshToken = json.optString("refresh_token")
        val expiresIn = json.optLong("expires_in", 3600)
        val user = json.optJSONObject("user") ?: return null
        val userId = user.optString("id")
        if (userId.isBlank()) return null
        val email = user.optString("email").ifBlank { emailHint }
        return CloudSession(
            email = email,
            userId = userId,
            accessToken = accessToken,
            refreshToken = refreshToken,
            expiresAt = System.currentTimeMillis() + expiresIn * 1000,
        )
    }

    private fun session(): CloudSession? {
        val token = prefs.getString(KEY_ACCESS, null) ?: return null
        val userId = prefs.getString(KEY_USER, null) ?: return null
        if (token.isBlank() || userId.isBlank()) return null
        return CloudSession(
            email = prefs.getString(KEY_EMAIL, "") ?: "",
            userId = userId,
            accessToken = token,
            refreshToken = prefs.getString(KEY_REFRESH, "") ?: "",
            expiresAt = prefs.getLong(KEY_EXPIRES, 0),
        )
    }

    private fun saveSession(session: CloudSession) {
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

    private fun validSession(): CloudSession {
        val current = session() ?: throw IOException("Sign in to Vynl Cloud first.")
        if (current.expiresAt > System.currentTimeMillis() + 30_000) return current
        synchronized(refreshLock) {
            val again = session() ?: throw IOException("Sign in to Vynl Cloud first.")
            if (again.expiresAt > System.currentTimeMillis() + 30_000) return again
            if (again.refreshToken.isBlank()) {
                clearSession()
                throw IOException("Your session expired. Sign in again.")
            }
            val body = JSONObject().put("refresh_token", again.refreshToken)
            val response = try {
                post("$baseUrl/auth/v1/token?grant_type=refresh_token", body, token = null)
            } catch (error: IOException) {
                clearSession()
                throw IOException("Your session expired. Sign in again.")
            }
            val fresh = parseSession(response.second, again.email) ?: run {
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
