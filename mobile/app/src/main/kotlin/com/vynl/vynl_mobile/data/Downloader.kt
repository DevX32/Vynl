package com.vynl.vynl_mobile.data

import android.content.Context
import java.io.File
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request

class Downloader(context: Context, private val repository: CatalogueRepository) {
    private val root = File(context.filesDir, "music")
    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .writeTimeout(60, TimeUnit.SECONDS)
        .build()

    suspend fun audio(track: CatalogueTrack, onProgress: (Float) -> Unit): String =
        withContext(Dispatchers.IO) {
            val stream = repository.stream(track.id)
            val extension = audioExtension(stream.mime)
            val folder = File(root, "audio").apply { mkdirs() }
            val name = safeName(track.id)
            val file = File(folder, "$name.$extension")
            val temporary = File(folder, "$name.$extension.${System.nanoTime()}.${System.currentTimeMillis()}.part")

            try {
                val request = Request.Builder().url(stream.url).build()
                http.newCall(request).execute().use { response ->
                    if (!response.isSuccessful) {
                        throw IllegalStateException("the download was refused (${response.code})")
                    }
                    val body = response.body ?: throw IllegalStateException("the download was empty")
                    val total = body.contentLength()
                    body.byteStream().use { input ->
                        temporary.outputStream().use { output ->
                            val buffer = ByteArray(64 * 1024)
                            var written = 0L
                            while (true) {
                                val read = input.read(buffer)
                                if (read <= 0) break
                                output.write(buffer, 0, read)
                                written += read
                                if (total > 0) onProgress(written.toFloat() / total)
                            }
                            if (written == 0L) {
                                temporary.delete()
                                throw IllegalStateException("the download was empty")
                            }
                        }
                    }
                }
                if (!temporary.renameTo(file)) {
                    temporary.copyTo(file, overwrite = true)
                    temporary.delete()
                }
            } catch (error: Exception) {
                runCatching { if (temporary.exists()) temporary.delete() }
                throw error
            }
            onProgress(1f)
            file.absolutePath
        }

    suspend fun cover(track: CatalogueTrack): String? = withContext(Dispatchers.IO) {
        runCatching {
            val url = track.cover?.takeIf { it.isNotBlank() } ?: return@runCatching null
            val folder = File(root, "covers").apply { mkdirs() }
            val file = File(folder, "${safeName(track.id)}.jpg")
            if (file.exists()) return@runCatching file.absolutePath
            if (!url.startsWith("http://") && !url.startsWith("https://")) {
                val local = File(url.removePrefix("file://"))
                val absolutePath = local.absolutePath
                val filesPath = root.absolutePath
                return@runCatching if (absolutePath.startsWith(filesPath + File.separator) && local.exists()) {
                    local.absolutePath
                } else {
                    null
                }
            }
            val request = Request.Builder().url(url).build()
            val temporary = File(folder, "${safeName(track.id)}.jpg.${System.nanoTime()}.part")
            if (temporary.exists()) temporary.delete()
            try {
                http.newCall(request).execute().use { response ->
                    if (!response.isSuccessful) return@use null
                    val contentType = response.header("Content-Type")?.lowercase().orEmpty()
                    if (!contentType.startsWith("image/") && !contentType.startsWith("application/octet-stream")) {
                        return@use null
                    }
                    val body = response.body ?: return@use null
                    temporary.outputStream().use { output ->
                        body.byteStream().use { input -> input.copyTo(output) }
                    }
                    null
                }
                if (!temporary.exists() || temporary.length() == 0L) return@runCatching null
                if (!looksLikeImage(temporary)) {
                    temporary.delete()
                    return@runCatching null
                }
                if (!temporary.exists()) return@runCatching null
                if (!temporary.renameTo(file)) {
                    temporary.copyTo(file, overwrite = true)
                    temporary.delete()
                }
                file.absolutePath
            } catch (error: Exception) {
                runCatching { if (temporary.exists()) temporary.delete() }
                null
            }
        }.getOrNull()
    }

    private fun audioExtension(mime: String): String {
        val value = mime.lowercase()
        return when {
            "webm" in value || "opus" in value -> "webm"
            "mpeg" in value || "mp3" in value -> "mp3"
            "aac" in value -> "aac"
            "ogg" in value || "vorbis" in value -> "ogg"
            else -> "m4a"
        }
    }

    private fun looksLikeImage(file: File): Boolean {
        if (file.length() < 4L) return false
        val head = ByteArray(16)
        val read = runCatching {
            file.inputStream().use { it.read(head) }
        }.getOrDefault(-1)
        if (read < 4) return false
        val jpeg = head[0] == 0xFF.toByte() && head[1] == 0xD8.toByte()
        val png = head[0] == 0x89.toByte() && head[1] == 0x50.toByte()
            && head[2] == 0x4E.toByte() && head[3] == 0x47.toByte()
        val gif = head[0] == 0x47.toByte() && head[1] == 0x49.toByte() && head[2] == 0x46.toByte()
        val webp = head[0] == 0x52.toByte() && head[1] == 0x49.toByte() && head[2] == 0x46.toByte()
            && head[3] == 0x46.toByte()
        return jpeg || png || gif || webp
    }

    private fun safeName(value: String): String {
        val sanitized = value.replace(Regex("[^A-Za-z0-9_-]"), "_").take(48)
        if (sanitized == value) return sanitized
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
        val hex = digest.joinToString("") { "%02x".format(it) }.take(16)
        return "${sanitized}_$hex".take(64)
    }
}
