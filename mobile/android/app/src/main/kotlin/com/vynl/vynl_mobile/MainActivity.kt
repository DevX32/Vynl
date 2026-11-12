package com.vynl.vynl_mobile

import android.os.Build
import com.ryanheise.audioservice.AudioServiceFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : AudioServiceFragmentActivity() {
    private val installChannel = "vynl/install"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, installChannel)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "canRequestInstalls" -> result.success(canRequestInstalls())
                    else -> result.notImplemented()
                }
            }
    }

    private fun canRequestInstalls(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return true
        return packageManager.canRequestPackageInstalls()
    }
}
