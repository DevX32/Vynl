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

data class DesktopPlayer(
    val playing: Boolean,
    val position: Double,
    val duration: Double,
    /** The desktop's volume, 0 to 100, exactly as it reports it. */
    val volume: Int,
    val track: DesktopTrack?,
    val queue: List<DesktopTrack> = emptyList(),
    val shuffle: Boolean = false,
    val loopMode: String = "off",
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
                val queue = json.optJSONArray("queue")?.let { items ->
                    (0 until items.length()).mapNotNull { index ->
                        items.optJSONObject(index)?.let(::track)
                    }
                } ?: emptyList()
                DesktopPlayer(
                    playing = json.optBoolean("playing", false),
                    position = json.optDouble("position", 0.0),
                    duration = json.optDouble("duration", 0.0),
                    volume = json.optInt("volume", 75).coerceIn(0, 100),
                    track = json.optJSONObject("track")?.let { track(it) },
                    queue = queue,
                    shuffle = json.optBoolean("shuffle", false),
                    loopMode = json.optString("loopMode", "off").ifBlank { "off" },
                )
            }
        }.getOrNull()
    }

    suspend fun action(name: String, time: Double? = null, volume: Int? = null, index: Int? = null): Boolean = withContext(Dispatchers.IO) {
        val body = JSONObject().put("action", name)
        if (time != null) body.put("time", time)
        if (volume != null) body.put("volume", volume)
        if (index != null) body.put("index", index)
        val request = request("/v1/player/action")
            .post(body.toString().toRequestBody("application/json".toMediaType()))
            .build()
        runCatching {
            http.newCall(request).execute().use { it.isSuccessful }
        }.getOrDefault(false)
    }

    /** The songs the desktop holds. */
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

    /** What the desktop has, with sizes and mtimes so the phone knows what it has. */
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

    /** The playlists the desktop has, with their tracks resolved to ids. */
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

    /** The lyrics of a track, plain text, as the desktop knows them. */
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

    /** Downloads the audio of a track into the given destination. */
    suspend fun downloadAudio(trackId: String, dest: File, onProgress: (Float) -> Unit): Long? =
        withContext(Dispatchers.IO) {
            downloadInto("/v1/tracks/$trackId/audio", dest) { onProgress(it) }
        }

    /** Downloads the cover of a track into the given destination, null if it hasn't one. */
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

    private fun track(json: JSONObject): DesktopTrack = DesktopTrack(
        id = json.optString("id", ""),
        title = json.optString("title", ""),
        artist = json.optString("artist", ""),
        album = json.optString("album", ""),
        duration = json.optDouble("duration", 0.0),
        cover = json.optString("cover", "").ifBlank { null },
    )

    companion object {
        private const val KEY_ADDRESS = "desktop_address"
        private const val KEY_TOKEN = "desktop_token"
    }
}
