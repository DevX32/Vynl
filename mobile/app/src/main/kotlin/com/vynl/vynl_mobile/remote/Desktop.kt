package com.vynl.vynl_mobile.remote

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream

data class DesktopTrack(
    val id: String,
    val title: String,
    val artist: String,
    val album: String?,
    val duration: Double,
    val cover: String?,
    val ext: String = "",
    val mtime: Long? = null,
)

data class DesktopManifestEntry(
    val id: String,
    val mtime: Long?,
    val size: Long,
    val hasCover: Boolean,
    val ext: String,
)

data class DesktopStreamed(
    val id: String,
    val title: String,
    val artist: String,
    val album: String?,
    val duration: Double?,
    val cover: String?,
)

data class DesktopPlaylist(
    val id: String,
    val name: String,
    val trackIds: List<String>,
    val streamed: List<DesktopStreamed>,
    val createdAt: Long,
    val updatedAt: Long,
)

data class DesktopLyrics(
    val kind: String,
    val text: String,
    val source: String,
)

class Desktop(context: Context) {
    private val prefs = context.getSharedPreferences("vynl", Context.MODE_PRIVATE)
    private val http = OkHttpClient.Builder()
        .connectTimeout(2_000, java.util.concurrent.TimeUnit.MILLISECONDS)
        .readTimeout(5_000, java.util.concurrent.TimeUnit.MILLISECONDS)
        .writeTimeout(5_000, java.util.concurrent.TimeUnit.MILLISECONDS)
        .build()

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

    suspend fun catalog(): List<DesktopTrack> = withContext(Dispatchers.IO) {
        parseEntries("/v1/catalog") { entry ->
            DesktopTrack(
                id = entry.optString("id", ""),
                title = entry.optString("title", "Unknown"),
                artist = entry.optString("artist", "Unknown"),
                album = entry.optString("album", ""),
                duration = entry.optDouble("duration", 0.0),
                cover = null,
                ext = entry.optString("ext", "m4a").ifBlank { "m4a" },
                mtime = entry.optLong("mtime", 0L),
            )
        }
    }

    suspend fun syncManifest(): List<DesktopManifestEntry> = withContext(Dispatchers.IO) {
        parseEntries("/v1/sync-manifest") { entry ->
            DesktopManifestEntry(
                id = entry.optString("id", ""),
                mtime = entry.optLong("mtime", 0L),
                size = entry.optLong("size", 0L),
                hasCover = entry.optBoolean("hasCover", false),
                ext = entry.optString("ext", "").ifBlank { "m4a" },
            )
        }
    }

    suspend fun playlists(): List<DesktopPlaylist> = withContext(Dispatchers.IO) {
        parseEntries("/v1/playlists") { entry ->
            DesktopPlaylist(
                id = entry.optString("id", ""),
                name = entry.optString("name", "Playlist"),
                trackIds = entry.optJSONArray("trackIds")?.let { ids ->
                    (0 until ids.length()).map { i -> ids.optString(i) }.filter { it.isNotBlank() }
                } ?: emptyList(),
                streamed = entry.optJSONArray("streamed")?.let { streams ->
                    (0 until streams.length()).mapNotNull { i ->
                        val s = streams.optJSONObject(i) ?: return@mapNotNull null
                        DesktopStreamed(
                            id = s.optString("id", ""),
                            title = s.optString("title", ""),
                            artist = s.optString("artist", ""),
                            album = s.optString("album", ""),
                            duration = s.optDouble("duration", 0.0).takeIf { it > 0.0 },
                            cover = s.optString("cover", "").ifBlank { null },
                        )
                    }
                } ?: emptyList(),
                createdAt = entry.optLong("createdAt", 0L),
                updatedAt = entry.optLong("updatedAt", 0L),
            )
        }
    }

    suspend fun syncedLyrics(trackId: String): DesktopLyrics? = withContext(Dispatchers.IO) {
        runCatching {
            http.newCall(request("/v1/tracks/$trackId/synced-lyrics").build()).execute().use { response ->
                if (!response.isSuccessful) return@use null
                val json = JSONObject(response.body?.string().orEmpty())
                DesktopLyrics(
                    kind = json.optString("kind", "lrc"),
                    text = json.optString("text", ""),
                    source = json.optString("source", ""),
                ).takeIf { it.text.isNotBlank() }
            }
        }.getOrNull()
    }

    suspend fun downloadAudio(trackId: String, dest: File, onProgress: (Float) -> Unit): Long? =
        withContext(Dispatchers.IO) {
            downloadInto("/v1/tracks/$trackId/audio", dest) { onProgress(it) }
        }

    suspend fun downloadCover(trackId: String, dest: File): Boolean? = withContext(Dispatchers.IO) {
        downloadInto("/v1/tracks/$trackId/cover", dest) { }; true
    }

    private suspend inline fun <T> parseEntries(
        path: String,
        crossinline entry: (JSONObject) -> T,
    ): List<T> {
        return runCatching {
            http.newCall(request(path).build()).execute().use { response ->
                if (!response.isSuccessful) return@use emptyList()
                val body = response.body?.string().orEmpty()
                val trimmed = body.trim()
                val array = if (trimmed.startsWith("[")) {
                    org.json.JSONArray(trimmed)
                } else {
                    JSONObject(trimmed).optJSONArray("items")?.takeIf { it.length() > 0 }
                        ?: org.json.JSONArray(trimmed)
                }
                (0 until array.length()).mapNotNull { index ->
                    val obj = array.optJSONObject(index) ?: return@mapNotNull null
                    entry(obj).takeIf { obj.optString("id", "").isNotBlank() }
                }
            }
        }.getOrDefault(emptyList())
    }

    private suspend inline fun downloadInto(
        path: String,
        dest: File,
        crossinline onProgress: (Float) -> Unit,
    ): Long? = withContext(Dispatchers.IO) {
        val request = request(path).build()
        try {
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    response.close()
                    return@use null
                }
                val total = response.body?.contentLength() ?: -1L
                FileOutputStream(dest).use { out ->
                    var read = 0L
                    val buf = ByteArray(64 * 1024)
                    val src = response.body?.byteStream() ?: return@use null
                    while (true) {
                        val n = src.read(buf)
                        if (n <= 0) break
                        out.write(buf, 0, n)
                        read += n
                        if (total > 0) onProgress(read.toFloat() / total)
                    }
                }
                dest.length()
            }
        } catch (e: Exception) {
            dest.delete()
            null
        }
    }

    companion object {
        private const val KEY_ADDRESS = "desktop_address"
        private const val KEY_TOKEN = "desktop_token"
    }
}
