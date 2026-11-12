package com.vynl.vynl_mobile

import android.Manifest
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import com.vynl.vynl_mobile.ui.components.SplashScreen
import com.vynl.vynl_mobile.ui.shell.HomeShell
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlTheme
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier

class MainActivity : ComponentActivity() {
    private val permissionLauncher =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
        )
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            window.isNavigationBarContrastEnforced = false
        }
        askForNotifications()
        setContent {
            VynlTheme {
                var ready by remember { mutableStateOf(false) }
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(VynlColors.bg),
                ) {
                    HomeShell()
                    if (!ready) {
                        SplashScreen(onExited = { ready = true })
                    }
                }
            }
        }
    }

    private fun askForNotifications() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        permissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
    }
}
