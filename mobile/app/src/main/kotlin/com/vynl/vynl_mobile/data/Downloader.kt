package com.vynl.vynl_mobile.data

import android.content.Context
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request

class Downloader(context: Context, private val repository: CatalogueRepository) {
    private val root = File(context.filesDir, "music")
    private val http = OkHttpClient.Builder().build()

    suspend fun audio(track: CatalogueTrack, onProgress: (Float) -> Unit): String =
        withContext(Dispatchers.IO) {
            val stream = repository.stream(track.id)
            val folder = File(root, "audio").apply { mkdirs() }
            val file = File(folder, "${track.id}.m4a")
            val temporary = File(folder, "${track.id}.part")

            val request = Request.Builder().url(stream.url).build()
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful) {
                    throw IllegalStateException("the download was refused")
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
                    }
                }
            }
            if (!temporary.renameTo(file)) {
                temporary.copyTo(file, overwrite = true)
                temporary.delete()
            }
            onProgress(1f)
            file.absolutePath
        }

    suspend fun cover(track: CatalogueTrack): String? = withContext(Dispatchers.IO) {
        val url = track.cover ?: return@withContext null
        val folder = File(root, "covers").apply { mkdirs() }
        val file = File(folder, "${track.id}.jpg")
        if (file.exists()) return@withContext file.absolutePath
        val request = Request.Builder().url(url).build()
        runCatching {
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful) return@use
                val body = response.body ?: return@use
                file.outputStream().use { output ->
                    body.byteStream().use { input -> input.copyTo(output) }
                }
            }
            file.absolutePath
        }.getOrNull()
    }
}
