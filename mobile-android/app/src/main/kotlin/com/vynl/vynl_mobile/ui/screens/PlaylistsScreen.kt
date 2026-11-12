package com.vynl.vynl_mobile.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.QueueMusic
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.EmptyState
import com.vynl.vynl_mobile.ui.components.SectionHeader
import com.vynl.vynl_mobile.ui.components.TrackTile
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlaylistsScreen(viewModel: AppViewModel) {
    val playlists by viewModel.playlists.collectAsStateWithLifecycle()
    val playlistTracks by viewModel.playlistTracks.collectAsStateWithLifecycle()
    val playback by viewModel.player.state.collectAsStateWithLifecycle()

    if (playlists.isEmpty()) {
        EmptyState(
            icon = Icons.Rounded.QueueMusic,
            title = "Playlists",
            subtitle = "Make a playlist and the songs you add to it live here.",
            modifier = Modifier.fillMaxSize(),
        )
        return
    }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = androidx.compose.foundation.layout.PaddingValues(bottom = 30.dp),
    ) {
        item { SectionHeader("YOUR PLAYLISTS") }
        items(playlists, key = { it.id }) { playlist ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(VynlRadius.tile))
                    .clickable { viewModel.openLocalPlaylist(playlist.id) }
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier = Modifier
                        .size(60.dp)
                        .clip(RoundedCornerShape(VynlRadius.control))
                        .background(VynlColors.surfaceRaised),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Rounded.QueueMusic,
                        contentDescription = null,
                        tint = VynlColors.faint,
                        modifier = Modifier.size(22.dp),
                    )
                }
                Spacer(Modifier.width(14.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = playlist.name,
                        maxLines = 1,
                        color = VynlColors.text,
                        fontSize = 14.5.sp,
                        fontWeight = FontWeight.W700,
                        fontFamily = VynlFonts.display,
                    )
                    Spacer(Modifier.height(2.dp))
                    Text(
                        text = if (playlist.trackCount == 1) "1 track" else "${playlist.trackCount} tracks",
                        color = VynlColors.faint,
                        fontSize = 11.5.sp,
                        fontFamily = VynlFonts.display,
                    )
                }
            }
        }
    }

    if (playlistTracks.isNotEmpty()) {
        ModalBottomSheet(
            onDismissRequest = { viewModel.clearLocalPlaylist() },
            containerColor = VynlColors.surface,
        ) {
            Column(modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 30.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = playlists.firstOrNull { it.id == viewModel.openPlaylistId }?.name ?: "Playlist",
                        color = VynlColors.text,
                        fontSize = 17.sp,
                        fontWeight = FontWeight.W800,
                        fontFamily = VynlFonts.display,
                    )
                    Spacer(Modifier.weight(1f))
                    Icon(
                        Icons.Rounded.PlayArrow,
                        contentDescription = null,
                        tint = VynlColors.accent,
                        modifier = Modifier.size(20.dp),
                    )
                }
                Spacer(Modifier.height(10.dp))
                playlistTracks.forEach { track ->
                    TrackTile(
                        track = track.toCatalogue(),
                        coverPath = track.coverPath,
                        active = playback.track?.id == track.id,
                        onTap = {
                            val index = playlistTracks.indexOf(track)
                            viewModel.playLocal(playlistTracks, index)
                        },
                    )
                }
            }
        }
    }
}
