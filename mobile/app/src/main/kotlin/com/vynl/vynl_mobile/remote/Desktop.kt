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

private fun encPath(value: String): String =
    java.net.URLEncoder.encode(value, "UTF-8").replace("+", "%20")

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

data class DesktopTasteTrack(
    val id: String,
    val title: String,
    val artist: String,
    val album: String?,
    val duration: Double,
    val cover: String?,
)

data class DesktopLyrics(
    val kind: String,
    val text: String,
    val source: String,
)

class DesktopHttpException(val code: Int, path: String) :
    java.io.IOException("desktop returned HTTP $code for $path")

class Desktop(context: Context) {
    private val prefs = context.getSharedPreferences("vynl", Context.MODE_PRIVATE)

    private val pinnedTrustManager = object : javax.net.ssl.X509TrustManager {
        override fun checkServerTrusted(
            chain: Array<out java.security.cert.X509Certificate>,
            authType: String,
        ) {
            val leaf = chain.firstOrNull()
                ?: throw java.security.cert.CertificateException("the desktop presented no certificate")
            try {
                leaf.checkValidity()
            } catch (error: java.security.cert.CertificateExpiredException) {
                throw java.security.cert.CertificateException(
                    "the desktop certificate expired — rotate it in the desktop app",
                    error,
                )
            } catch (error: java.security.cert.CertificateNotYetValidException) {
                throw java.security.cert.CertificateException(
                    "the desktop certificate is not valid yet",
                    error,
                )
            }
            val expected = fingerprint
            if (expected.isEmpty()) {
                throw java.security.cert.CertificateException("the desktop certificate is not pinned — pair again with the QR code")
            }
            if (!expected.matches(Regex("^[0-9a-f]{64}$"))) {
                throw java.security.cert.CertificateException("the desktop fingerprint is malformed — pair again with the QR code")
            }
            val actual = java.security.MessageDigest.getInstance("SHA-256").digest(leaf.encoded)
            val expectedBytes = expected.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
            if (!java.security.MessageDigest.isEqual(actual, expectedBytes)) {
                throw java.security.cert.CertificateException("the desktop certificate changed")
            }
        }

        override fun checkClientTrusted(
            chain: Array<out java.security.cert.X509Certificate>,
            authType: String,
        ) = Unit

        override fun getAcceptedIssuers(): Array<java.security.cert.X509Certificate> =
            emptyArray()
    }

    private val http: OkHttpClient = run {
        val sslContext = javax.net.ssl.SSLContext.getInstance("TLS")
        sslContext.init(null, arrayOf(pinnedTrustManager), java.security.SecureRandom())
        OkHttpClient.Builder()
            .connectTimeout(2_000, java.util.concurrent.TimeUnit.MILLISECONDS)
            .readTimeout(5_000, java.util.concurrent.TimeUnit.MILLISECONDS)
            .writeTimeout(5_000, java.util.concurrent.TimeUnit.MILLISECONDS)
            .sslSocketFactory(sslContext.socketFactory, pinnedTrustManager)
            .hostnameVerifier { _, _ -> true }
            .build()
    }

    var address: String
        get() = SecurePrefs.getString(prefs, KEY_ADDRESS, "") ?: ""
        set(value) {
            val editor = prefs.edit()
            SecurePrefs.putString(editor, KEY_ADDRESS, value.trim())
            editor.apply()
        }

    var token: String
        get() = SecurePrefs.getString(prefs, KEY_TOKEN, "") ?: ""
        set(value) {
            val editor = prefs.edit()
            SecurePrefs.putString(editor, KEY_TOKEN, value.trim())
            editor.apply()
        }

    var fingerprint: String
        get() = SecurePrefs.getString(prefs, KEY_FINGERPRINT, "") ?: ""
        set(value) {
            val editor = prefs.edit()
            SecurePrefs.putString(editor, KEY_FINGERPRINT, value.trim().lowercase())
            editor.apply()
            http.connectionPool.evictAll()
        }

    val paired: Boolean
        get() = address.isNotEmpty() && token.isNotEmpty() && fingerprint.isNotEmpty()

    private fun url(path: String): String {
        val host = address.trim().removePrefix("http://").removePrefix("https://").removeSuffix("/")
        return "https://$host$path"
    }

    private fun request(path: String): Request.Builder {
        val builder = Request.Builder().url(url(path))
        if (token.isNotEmpty()) builder.header("Authorization", "Bearer $token")
        return builder
    }

    suspend fun pair(pin: String): String? = withContext(Dispatchers.IO) {
        val body = JSONObject().put("pin", pin.trim()).toString()
            .toRequestBody("application/json".toMediaType())
        val request = Request.Builder()
            .url(url("/pair"))
            .post(body)
            .build()
        runCatching {
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful) return@use "That pin was not accepted"
                val json = JSONObject(response.body?.string().orEmpty())
                val newToken = json.optString("token", "")
                if (newToken.isEmpty()) return@use "That pin was not accepted"
                token = newToken
                null
            }
        }.getOrElse { error ->
            error.message?.takeIf { it.isNotBlank() } ?: "Could not reach the desktop"
        }
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

    suspend fun taste(): List<DesktopTasteTrack> = withContext(Dispatchers.IO) {
        http.newCall(request("/v1/taste").build()).execute().use { response ->
            if (!response.isSuccessful) {
                throw DesktopHttpException(response.code, "/v1/taste")
            }
            val body = response.body?.string().orEmpty()
            if (body.isBlank()) {
                throw java.io.IOException("desktop returned an empty /v1/taste body")
            }
            val liked = JSONObject(body).optJSONArray("liked")
                ?: throw java.io.IOException("desktop /v1/taste response has no liked list")
            (0 until liked.length()).mapNotNull { index ->
                val s = liked.optJSONObject(index) ?: return@mapNotNull null
                val id = s.optString("id")
                if (id.isBlank()) return@mapNotNull null
                DesktopTasteTrack(
                    id = id,
                    title = s.optString("title", "Unknown"),
                    artist = s.optString("artist", "Unknown"),
                    album = s.optString("album", "").ifBlank { null },
                    duration = s.optDouble("duration", 0.0),
                    cover = s.optString("cover", "").ifBlank { null },
                )
            }
        }
    }

    suspend fun syncedLyrics(trackId: String): DesktopLyrics? = withContext(Dispatchers.IO) {
        runCatching {
            http.newCall(request("/v1/tracks/${encPath(trackId)}/synced-lyrics").build()).execute().use { response ->
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
            downloadInto("/v1/tracks/${encPath(trackId)}/audio", dest) { onProgress(it) }
        }

    suspend fun downloadCover(trackId: String, dest: File): Boolean? = withContext(Dispatchers.IO) {
        val written = downloadInto("/v1/tracks/${encPath(trackId)}/cover", dest) { }
        written != null && dest.isFile && dest.length() > 0
    }

    private suspend inline fun <T> parseEntries(
        path: String,
        crossinline entry: (JSONObject) -> T,
    ): List<T> {
        return http.newCall(request(path).build()).execute().use { response ->
            if (!response.isSuccessful) {
                throw DesktopHttpException(response.code, path)
            }
            val body = response.body?.string().orEmpty()
            val trimmed = body.trim()
            val array = if (trimmed.startsWith("[")) {
                org.json.JSONArray(trimmed)
            } else {
                JSONObject(trimmed).optJSONArray("items") ?: org.json.JSONArray()
            }
            (0 until array.length()).mapNotNull { index ->
                val obj = array.optJSONObject(index) ?: return@mapNotNull null
                entry(obj).takeIf { obj.optString("id", "").isNotBlank() }
            }
        }
    }

    private suspend inline fun downloadInto(
        path: String,
        dest: File,
        crossinline onProgress: (Float) -> Unit,
    ): Long? = withContext(Dispatchers.IO) {
        val request = request(path).build()
        val tmp = File(
            dest.parentFile ?: dest.absoluteFile.parentFile,
            "${dest.name}.${System.nanoTime()}.part",
        )
        try {
            http.newCall(request).execute().use httpUse@{ response ->
                if (!response.isSuccessful) {
                    response.close()
                    tmp.delete()
                    return@httpUse null
                }
                val total = response.body?.contentLength() ?: -1L
                FileOutputStream(tmp).use { out ->
                    var read = 0L
                    val buf = ByteArray(64 * 1024)
                    val src = response.body?.byteStream()
                    if (src == null) {
                        tmp.delete()
                        return@httpUse null
                    }
                    while (true) {
                        val n = src.read(buf)
                        if (n <= 0) break
                        out.write(buf, 0, n)
                        read += n
                        if (total > 0) onProgress(read.toFloat() / total)
                    }
                }
                if (!tmp.renameTo(dest)) {
                    val staged = File(
                        dest.parentFile ?: dest.absoluteFile.parentFile,
                        "${dest.name}.${System.nanoTime()}.part",
                    )
                    try {
                        tmp.copyTo(staged, overwrite = true)
                        tmp.delete()
                    } catch (error: Exception) {
                        staged.delete()
                        throw error
                    }
                    if (dest.exists()) {
                        dest.delete()
                    }
                    if (!staged.renameTo(dest)) {
                        staged.copyTo(dest, overwrite = true)
                        staged.delete()
                    }
                }
                dest.length()
            }
        } catch (e: Exception) {
            tmp.delete()
            null
        }
    }

    companion object {
        private const val KEY_ADDRESS = "desktop_address"
        private const val KEY_TOKEN = "desktop_token"
        private const val KEY_FINGERPRINT = "desktop_fingerprint"
    }
}
