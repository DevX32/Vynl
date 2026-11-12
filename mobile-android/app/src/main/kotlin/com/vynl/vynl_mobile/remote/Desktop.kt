package com.vynl.vynl_mobile.remote

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject

data class DesktopTrack(
    val id: String,
    val title: String,
    val artist: String,
    val album: String?,
    val duration: Double,
    val cover: String?,
)

data class DesktopPlayer(
    val playing: Boolean,
    val position: Double,
    val duration: Double,
    val volume: Double,
    val track: DesktopTrack?,
)

/**
 * The link to the desktop: pairing with the pin it shows, the player it is
 * playing and the library it holds.
 */
class Desktop(context: Context) {
    private val prefs = context.getSharedPreferences("vynl", Context.MODE_PRIVATE)
    private val http = OkHttpClient.Builder().build()

    var address: String
        get() = prefs.getString(KEY_ADDRESS, "") ?: ""
        set(value) = prefs.edit().putString(KEY_ADDRESS, value.trim()).apply()

    var token: String
        get() = prefs.getString(KEY_TOKEN, "") ?: ""
        set(value) = prefs.edit().putString(KEY_TOKEN, value.trim()).apply()

    val paired: Boolean
        get() = address.isNotEmpty() && token.isNotEmpty()

    private fun url(path: String): String {
        val host = address.trim().removePrefix("http://").removeSuffix("/")
        return "http://$host$path"
    }

    private fun request(path: String): Request.Builder {
        val builder = Request.Builder().url(url(path))
        if (token.isNotEmpty()) builder.header("Authorization", "Bearer $token")
        return builder
    }

    /** Trades the pin for a token. */
    suspend fun pair(pin: String): Boolean = withContext(Dispatchers.IO) {
        val body = JSONObject().put("pin", pin.trim()).toString()
            .toRequestBody("application/json".toMediaType())
        val request = Request.Builder()
            .url(url("/pair"))
            .post(body)
            .build()
        runCatching {
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful) return@use false
                val json = JSONObject(response.body?.string().orEmpty())
                val newToken = json.optString("token", "")
                if (newToken.isEmpty()) return@use false
                token = newToken
                true
            }
        }.getOrDefault(false)
    }

    suspend fun player(): DesktopPlayer? = withContext(Dispatchers.IO) {
        runCatching {
            http.newCall(request("/v1/player").build()).execute().use { response ->
                if (!response.isSuccessful) return@use null
                val json = JSONObject(response.body?.string().orEmpty())
                DesktopPlayer(
                    playing = json.optBoolean("playing", false),
                    position = json.optDouble("position", 0.0),
                    duration = json.optDouble("duration", 0.0),
                    volume = json.optDouble("volume", 1.0),
                    track = json.optJSONObject("track")?.let { track(it) },
                )
            }
        }.getOrNull()
    }

    suspend fun action(name: String, time: Double? = null): Boolean = withContext(Dispatchers.IO) {
        val body = JSONObject().put("action", name)
        if (time != null) body.put("time", time)
        val request = request("/v1/player/action")
            .post(body.toString().toRequestBody("application/json".toMediaType()))
            .build()
        runCatching {
            http.newCall(request).execute().use { it.isSuccessful }
        }.getOrDefault(false)
    }

    /** The songs the desktop holds. */
    suspend fun catalog(): List<DesktopTrack> = withContext(Dispatchers.IO) {
        runCatching {
            http.newCall(request("/v1/catalog").build()).execute().use { response ->
                if (!response.isSuccessful) return@use emptyList()
                val array = org.json.JSONArray(response.body?.string().orEmpty())
                buildList {
                    for (index in 0 until array.length()) {
                        val entry = array.optJSONObject(index) ?: continue
                        add(
                            DesktopTrack(
                                id = entry.optString("id", ""),
                                title = entry.optString("title", "Unknown"),
                                artist = entry.optString("artist", "Unknown"),
                                album = entry.optString("album", ""),
                                duration = entry.optDouble("duration", 0.0),
                                cover = null,
                            ),
                        )
                    }
                }
            }
        }.getOrDefault(emptyList())
    }

    /** The songs the user likes, as the desktop knows them. */
    suspend fun liked(): List<DesktopTrack> = withContext(Dispatchers.IO) {
        runCatching {
            http.newCall(request("/v1/taste").build()).execute().use { response ->
                if (!response.isSuccessful) return@use emptyList()
                val json = JSONObject(response.body?.string().orEmpty())
                val array = json.optJSONArray("liked")
                buildList {
                    for (index in 0 until (array?.length() ?: 0)) {
                        val entry = array?.optJSONObject(index) ?: continue
                        add(track(entry))
                    }
                }
            }
        }.getOrDefault(emptyList())
    }

    private fun track(json: JSONObject): DesktopTrack = DesktopTrack(
        id = json.optString("id", ""),
        title = json.optString("title", "Unknown"),
        artist = json.optString("artist", "Unknown"),
        album = json.optStringOrNull("album"),
        duration = json.optDouble("duration", 0.0),
        cover = json.optStringOrNull("cover"),
    )

    private fun JSONObject.optStringOrNull(key: String): String? {
        if (!has(key) || isNull(key)) return null
        val value = optString(key, "")
        return value.ifEmpty { null }
    }

    companion object {
        const val KEY_ADDRESS = "desktop_address"
        const val KEY_TOKEN = "desktop_token"
    }
}
