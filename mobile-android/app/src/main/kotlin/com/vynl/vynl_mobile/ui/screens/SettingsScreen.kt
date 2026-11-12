package com.vynl.vynl_mobile.ui.screens

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Tune
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.vynl.vynl_mobile.ui.components.EmptyState

@Composable
fun SettingsScreen(onClose: () -> Unit) {
    EmptyState(
        icon = Icons.Rounded.Tune,
        title = "Settings",
        subtitle = "Accent, storage and the updater land in the next pass.",
        modifier = Modifier
            .fillMaxSize()
            .padding(top = 120.dp),
    )
}
