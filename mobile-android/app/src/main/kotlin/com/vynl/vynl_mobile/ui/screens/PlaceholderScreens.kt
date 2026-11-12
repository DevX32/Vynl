package com.vynl.vynl_mobile.ui.screens

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.LibraryMusic
import androidx.compose.material.icons.rounded.QueueMusic
import androidx.compose.material.icons.rounded.Tune
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.EmptyState

@Composable
fun LibraryScreen(viewModel: AppViewModel) {
    EmptyState(
        icon = Icons.Rounded.LibraryMusic,
        title = "Library",
        subtitle = "Downloaded songs and the playlists you make will live here.",
        modifier = Modifier
            .fillMaxSize()
            .padding(top = 40.dp),
    )
}

@Composable
fun PlaylistsScreen(viewModel: AppViewModel) {
    EmptyState(
        icon = Icons.Rounded.QueueMusic,
        title = "Playlists",
        subtitle = "Your playlists will live here.",
        modifier = Modifier
            .fillMaxSize()
            .padding(top = 40.dp),
    )
}

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
