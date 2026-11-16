package com.vynl.vynl_mobile.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Download
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.automirrored.rounded.PlaylistAdd
import androidx.compose.material.icons.rounded.PlaylistRemove
import androidx.compose.material.icons.automirrored.rounded.QueueMusic
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.ArtTile
import com.vynl.vynl_mobile.ui.components.EmptyState
import com.vynl.vynl_mobile.ui.components.LocalTrackAction
import com.vynl.vynl_mobile.ui.components.LocalTrackActionsSheet
import com.vynl.vynl_mobile.ui.components.TrackTile
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.data.LocalTrack
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@Composable
fun PlaylistDetailScreen(viewModel: AppViewModel, onClose: () -> Unit) {
    val playlists by viewModel.playlists.collectAsStateWithLifecycle()
    val covers by viewModel.playlistCovers.collectAsStateWithLifecycle()
    val playlistTracks by viewModel.playlistTracks.collectAsStateWithLifecycle()
    val playback by viewModel.player.state.collectAsStateWithLifecycle()
    val playlist = playlists.firstOrNull { it.id == viewModel.openPlaylistId }
    val (localCover, remoteCover) = covers[viewModel.openPlaylistId.orEmpty()] ?: Pair(null, null)
    var actionsFor by remember { mutableStateOf<LocalTrack?>(null) }
    var query by remember { mutableStateOf("") }

    val needle = query.trim().lowercase()
    val shown = playlistTracks.filter { track ->
        needle.isEmpty() ||
            track.title.lowercase().contains(needle) ||
            track.artist.lowercase().contains(needle) ||
            track.album.orEmpty().lowercase().contains(needle)
    }

    BackHandler { onClose() }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg)
            .statusBarsPadding()
            .navigationBarsPadding(),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onClose) {
                Icon(
                    Icons.Rounded.KeyboardArrowDown,
                    contentDescription = "Close the playlist",
                    tint = VynlColors.dim,
                    modifier = Modifier.size(28.dp),
                )
            }
            Text(
                text = "PLAYLIST",
                modifier = Modifier.weight(1f),
                textAlign = TextAlign.Center,
                color = VynlColors.faint,
                fontSize = 11.sp,
                fontWeight = androidx.compose.ui.text.font.FontWeight.W800,
                letterSpacing = 1.3.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.width(48.dp))
        }

        if (playlistTracks.isNotEmpty()) {
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = 16.dp, end = 16.dp, top = 2.dp, bottom = 6.dp),
                singleLine = true,
                textStyle = androidx.compose.ui.text.TextStyle(
                    fontSize = 14.sp,
                    color = VynlColors.text,
                    fontFamily = VynlFonts.display,
                ),
                placeholder = {
                    Text(
                        "Search this playlist",
                        color = VynlColors.faint,
                        fontSize = 14.sp,
                        fontFamily = VynlFonts.display,
                    )
                },
                leadingIcon = {
                    Icon(
                        Icons.Rounded.Search,
                        contentDescription = null,
                        tint = VynlColors.faint,
                        modifier = Modifier.size(20.dp),
                    )
                },
                shape = RoundedCornerShape(VynlRadius.control),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedContainerColor = VynlColors.surface,
                    unfocusedContainerColor = VynlColors.surface,
                    focusedBorderColor = VynlColors.line,
                    unfocusedBorderColor = VynlColors.line,
                    cursorColor = VynlColors.accent,
                    focusedTextColor = VynlColors.text,
                    unfocusedTextColor = VynlColors.text,
                ),
            )
        }

        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(bottom = 48.dp),
        ) {
            item {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 22.dp, vertical = 6.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    ArtTile(
                        coverUrl = remoteCover,
                        coverPath = localCover,
                        size = 208.dp,
                        radius = VynlRadius.tile,
                    )
                    Spacer(Modifier.height(16.dp))
                    Text(
                        text = playlist?.name ?: "Playlist",
                        color = VynlColors.text,
                        fontSize = 22.sp,
                        fontWeight = androidx.compose.ui.text.font.FontWeight.W800,
                        letterSpacing = (-0.4).sp,
                        textAlign = TextAlign.Center,
                        fontFamily = VynlFonts.display,
                    )
                    Spacer(Modifier.height(4.dp))
                    Text(
                        text = if (playlistTracks.size == 1) "1 track" else "${playlistTracks.size} tracks",
                        color = VynlColors.dim,
                        fontSize = 12.5.sp,
                        fontFamily = VynlFonts.display,
                    )
                    Spacer(Modifier.height(18.dp))
                    if (playlistTracks.isNotEmpty()) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(18.dp),
                        ) {
                            VynlControl(
                                onTap = { viewModel.playLocal(playlistTracks, 0) },
                                icon = Icons.Rounded.PlayArrow,
                                style = VynlControlStyle.accent,
                                color = VynlColors.accent,
                                size = 52.dp,
                                iconSize = 30.dp,
                            )
                            VynlControl(
                                onTap = { viewModel.downloadOpenPlaylist() },
                                icon = Icons.Rounded.Download,
                                style = VynlControlStyle.raised,
                                size = 44.dp,
                                iconSize = 22.dp,
                            )
                        }
                    }
                    Spacer(Modifier.height(8.dp))
                }
            }

            if (playlistTracks.isEmpty()) {
                item {
                    EmptyState(
                        icon = Icons.AutoMirrored.Rounded.QueueMusic,
                        title = "Nothing here yet",
                        subtitle = "Play something and press the three dots to add songs to this playlist.",
                    )
                }
            } else if (shown.isEmpty()) {
                item {
                    EmptyState(
                        icon = Icons.Rounded.Search,
                        title = "No matches",
                        subtitle = "No songs in this playlist match “${query.trim()}”.",
                    )
                }
            }

            itemsIndexed(shown, key = { index, track -> "$index-${track.id}" }) { index, track ->
                TrackTile(
                    track = track.toCatalogue(),
                    coverPath = track.coverPath,
                    active = playback.track?.id == track.id,
                    onTap = { viewModel.playLocal(shown, index) },
                    onMore = { actionsFor = track },
                )
            }
        }

        val menuTrack = actionsFor
        if (menuTrack != null) {
            LocalTrackActionsSheet(
                track = menuTrack.toCatalogue(),
                actions = listOf(
                    LocalTrackAction(
                        icon = Icons.AutoMirrored.Rounded.PlaylistAdd,
                        title = "Add to playlist",
                        onClick = { viewModel.openPlaylistPicker(menuTrack.toCatalogue()) },
                    ),
                    LocalTrackAction(
                        icon = Icons.Rounded.PlaylistRemove,
                        title = "Remove from playlist",
                        destructive = true,
                        onClick = {
                            viewModel.removeFromPlaylist(viewModel.openPlaylistId.orEmpty(), menuTrack.id)
                        },
                    ),
                ),
                onDismiss = { actionsFor = null },
            )
        }
    }
}
