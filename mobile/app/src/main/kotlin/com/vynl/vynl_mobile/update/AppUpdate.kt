package com.vynl.vynl_mobile.update

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject

private const val MANIFEST_URL =
    "https://github.com/DevX32/Vynl/releases/latest/download/latest-mobile.json"

data class Update(
    val version: String,
    val build: Int,
    val notes: String,
    val apks: Map<String, String>,
    val builds: Map<String, Int> = emptyMap(),
) {
    val short: String
        get() = version.substringBefore('+')

    /** The build number of the apk made for a certain phone. */
    fun buildFor(abi: String?): Int = abi?.let { builds[it] } ?: build
}

/**
 * The updater. It reads the manifest the release publishes, works out whether
 * the build on the phone is older, fetches the right apk and hands it to the
 * system installer.
 */
class AppUpdate(private val context: Context) {
    private val http = OkHttpClient.Builder().build()

    fun installedVersion(): Pair<String, Int> {
        val info = context.packageManager.getPackageInfo(context.packageName, 0)
        val name = info.versionName.orEmpty()
        val build = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            info.longVersionCode.toInt()
        } else {
            @Suppress("DEPRECATION")
            info.versionCode
        }
        return name to build
    }

    suspend fun check(): Update? = withContext(Dispatchers.IO) {
        val request = Request.Builder().url(MANIFEST_URL).build()
        val response = http.newCall(request).execute()
        if (!response.isSuccessful) return@withContext null
        val body = response.body?.string() ?: return@withContext null
        parse(body)
    }

    private fun parse(body: String): Update? {
        val json = JSONObject(body)
        val version = json.optString("version", "")
        if (version.isEmpty()) return null
        val build = json.optString("build", "0").toIntOrNull() ?: 0
        val notes = json.optString("notes", "")
        val apks = mutableMapOf<String, String>()
        json.optJSONObject("apks")?.let { raw ->
            raw.keys().forEach { key ->
                val url = raw.optString(key, "")
                if (url.isNotEmpty()) apks[key] = url
            }
        }
        val builds = mutableMapOf<String, Int>()
        json.optJSONObject("builds")?.let { raw ->
            raw.keys().forEach { key ->
                val code = raw.optInt(key, -1)
                if (code > 0) builds[key] = code
            }
        }
        return Update(version, build, notes, apks, builds)
    }

    /** The abi of this phone. */
    fun abi(): String? = Build.SUPPORTED_ABIS.firstOrNull()

    /** The apk that suits this phone. */
    fun urlFor(update: Update): String? {
        val abi = abi() ?: return null
        return update.apks[abi] ?: update.apks.values.firstOrNull()
    }

    suspend fun download(url: String, onProgress: (Float) -> Unit): File = withContext(Dispatchers.IO) {
        val folder = File(context.filesDir, "updates").apply { mkdirs() }
        val file = File(folder, "vynl-update.apk")
        val request = Request.Builder().url(url).build()
        http.newCall(request).execute().use { response ->
            if (!response.isSuccessful) throw IllegalStateException("the update was refused")
            val body = response.body ?: throw IllegalStateException("the update was empty")
            val total = body.contentLength()
            body.byteStream().use { input ->
                file.outputStream().use { output ->
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
        onProgress(1f)
        file
    }

    /** Whether the system will let the app install packages. */
    fun canInstall(): Boolean =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.packageManager.canRequestPackageInstalls()
        } else {
            true
        }

    /** The screen where the permission is given. */
    fun installSettingsIntent(): Intent =
        Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${context.packageName}"))

    /** Hands the apk to the system installer. */
    fun install(file: File): Boolean {
        val uri = FileProvider.getUriForFile(
            context,
            "${context.packageName}.fileprovider",
            file,
        )
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, "application/vnd.android.package-archive")
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        return try {
            context.startActivity(intent)
            true
        } catch (error: Exception) {
            false
        }
    }

    companion object {
        /** Whether the update on offer is newer than what is installed. */
        fun isNewer(current: String, currentBuild: Int, update: Update, abi: String? = null): Boolean {
            val target = update.buildFor(abi)
            if (target != currentBuild) return target > currentBuild
            val left = current.substringBefore('+').split('.')
            val right = update.short.split('.')
            for (index in 0 until maxOf(left.size, right.size)) {
                val a = left.getOrNull(index)?.toIntOrNull() ?: 0
                val b = right.getOrNull(index)?.toIntOrNull() ?: 0
                if (a != b) return b > a
            }
            return false
        }
    }
}
