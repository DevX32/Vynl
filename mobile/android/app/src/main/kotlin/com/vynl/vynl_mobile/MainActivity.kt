package com.vynl.vynl_mobile

import android.os.Build
import com.ryanheise.audioservice.AudioServiceFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : AudioServiceFragmentActivity() {
    private val installChannel = "vynl/install"
    private val catalogueChannel = "vynl/catalogue"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, installChannel)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "canRequestInstalls" -> result.success(canRequestInstalls())
                    else -> result.notImplemented()
                }
            }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, catalogueChannel)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "available" -> result.success(CatalogueBridge.isAvailable())
                    "search" -> respond(result) { CatalogueBridge.nativeSearch(call.argument<String>("value") ?: "") }
                    "album" -> respond(result) { CatalogueBridge.nativeAlbum(call.argument<String>("value") ?: "") }
                    "artist" -> respond(result) { CatalogueBridge.nativeArtist(call.argument<String>("value") ?: "") }
                    "playlist" -> respond(result) { CatalogueBridge.nativePlaylist(call.argument<String>("value") ?: "") }
                    "stream" -> respond(result) { CatalogueBridge.nativeStream(call.argument<String>("value") ?: "") }
                    else -> result.notImplemented()
                }
            }
    }

    private fun respond(result: MethodChannel.Result, call: () -> String) {
        try {
            result.success(call())
        } catch (error: Throwable) {
            result.success("{\"error\":\"${error.message ?: "the catalogue failed"}\"}")
        }
    }

    private fun canRequestInstalls(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return true
        return packageManager.canRequestPackageInstalls()
    }
}
