package com.vynl.vynl_mobile.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.ArtTile
import com.vynl.vynl_mobile.ui.components.CatalogueCard
import com.vynl.vynl_mobile.ui.components.EmptyState
import com.vynl.vynl_mobile.ui.components.SectionHeader
import com.vynl.vynl_mobile.ui.components.TrackTile
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BrowseScreen(viewModel: AppViewModel) {
    val results by viewModel.results.collectAsStateWithLifecycle()
    val playback by viewModel.player.state.collectAsStateWithLifecycle()
    val query = remember { androidx.compose.runtime.mutableStateOf(viewModel.browseQuery) }

    Column(modifier = Modifier.fillMaxSize()) {
        OutlinedTextField(
            value = query.value,
            onValueChange = { query.value = it },
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 16.dp, end = 16.dp, top = 4.dp, bottom = 10.dp),
            singleLine = true,
            textStyle = androidx.compose.ui.text.TextStyle(
                fontSize = 14.sp,
                color = VynlColors.text,
                fontFamily = VynlFonts.display,
            ),
            placeholder = {
                Text(
                    "Search the catalogue",
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
            trailingIcon = {
                if (viewModel.browseSearching) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(16.dp),
                        strokeWidth = 2.dp,
                        color = VynlColors.accent,
                    )
                }
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
            keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(
                imeAction = androidx.compose.ui.text.input.ImeAction.Search,
            ),
            keyboardActions = androidx.compose.foundation.text.KeyboardActions(
                onSearch = { viewModel.browse(query.value) },
            ),
        )

        val error = viewModel.browseError
        when {
            error != null -> {
                EmptyState(
                    icon = Icons.Rounded.Search,
                    title = "Search failed",
                    subtitle = error,
                    modifier = Modifier.fillMaxSize(),
                )
            }

            results == null -> {
                val mix by viewModel.mix.collectAsStateWithLifecycle()
                if (viewModel.mixError != null) {
                    Column(modifier = Modifier.fillMaxSize()) {
                        SectionHeader("MADE FOR YOU")
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 22.dp, vertical = 8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Icon(
                                Icons.Rounded.Search,
                                contentDescription = null,
                                tint = VynlColors.faint,
                                modifier = Modifier.size(16.dp),
                            )
                            Spacer(Modifier.width(8.dp))
                            Text(
                                text = "The mix didn't load",
                                color = VynlColors.dim,
                                fontSize = 13.sp,
                                fontFamily = VynlFonts.display,
                                modifier = Modifier.weight(1f),
                            )
                            Text(
                                text = "Retry",
                                color = VynlColors.accent,
                                fontSize = 13.sp,
                                fontWeight = FontWeight.W700,
                                fontFamily = VynlFonts.display,
                                modifier = Modifier
                                    .clickable { viewModel.refreshMix() }
                                    .padding(horizontal = 8.dp, vertical = 8.dp),
                            )
                        }
                    }
                } else if (viewModel.mixLoading) {
                    Column(modifier = Modifier.fillMaxSize()) {
                        SectionHeader("MADE FOR YOU")
                        repeat(6) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(horizontal = 16.dp, vertical = 6.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(56.dp)
                                        .clip(RoundedCornerShape(VynlRadius.thumb))
                                        .background(VynlColors.surfaceRaised),
                                )
                                Spacer(Modifier.width(14.dp))
                                Column {
                                    Box(
                                        modifier = Modifier
                                            .width(140.dp)
                                            .height(11.dp)
                                            .clip(RoundedCornerShape(2.dp))
                                            .background(VynlColors.surfaceRaised),
                                    )
                                    Spacer(Modifier.height(6.dp))
                                    Box(
                                        modifier = Modifier
                                            .width(90.dp)
                                            .height(9.dp)
                                            .clip(RoundedCornerShape(2.dp))
                                            .background(VynlColors.surfaceRaised),
                                    )
                                }
                            }
                        }
                    }
                } else if (mix.isNotEmpty()) {
                    Column(modifier = Modifier.fillMaxSize()) {
                        SectionHeader("MADE FOR YOU")
                        LazyColumn(
                            modifier = Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(bottom = 30.dp),
                        ) {
                            items(mix, key = { it.id }) { track ->
                                TrackTile(
                                    track = track,
                                    active = playback.track?.id == track.id,
                                    onTap = { viewModel.player.play(mix, mix.indexOf(track)) },
                                    onMore = { viewModel.openActions(track) },
                                )
                            }
                        }
                    }
                } else {
                    EmptyState(
                        icon = Icons.Rounded.Search,
                        title = "Browse",
                        subtitle = "Search for a song, album, or artist and play it right from the phone.",
                        modifier = Modifier.fillMaxSize(),
                    )
                }
            }

            else -> {
                val found = results!!
                if (found.songs.isEmpty() && found.albums.isEmpty() && found.playlists.isEmpty()) {
                    EmptyState(
                        icon = Icons.Rounded.Search,
                        title = "No results",
                        subtitle = "Try a different song, album, or artist.",
                        modifier = Modifier.fillMaxSize(),
                    )
                } else {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(start = 4.dp, end = 4.dp, bottom = 30.dp),
                    ) {
                        if (found.songs.isNotEmpty()) {
                            item { SectionHeader("SONGS") }
                            items(found.songs, key = { it.id }) { song ->
                                TrackTile(
                                    track = song,
                                    active = playback.track?.id == song.id,
                                    onTap = { viewModel.player.play(found.songs, found.songs.indexOf(song)) },
                                    onMore = { viewModel.openActions(song) },
                                )
                            }
                        }
                        if (found.albums.isNotEmpty()) {
                            item { SectionHeader("ALBUMS") }
                            item {
                                LazyRow(
                                    contentPadding = PaddingValues(horizontal = 16.dp),
                                    horizontalArrangement = Arrangement.spacedBy(14.dp),
                                ) {
                                    items(found.albums, key = { it.id }) { album ->
                                        CatalogueCard(
                                            title = album.title,
                                            subtitle = album.year?.toString() ?: album.artist,
                                            coverUrl = album.cover,
                                            onClick = { viewModel.openAlbum(album.id) },
                                        )
                                    }
                                }
                            }
                        }
                        if (found.playlists.isNotEmpty()) {
                            item { SectionHeader("PLAYLISTS") }
                            item {
                                LazyRow(
                                    contentPadding = PaddingValues(horizontal = 16.dp),
                                    horizontalArrangement = Arrangement.spacedBy(14.dp),
                                ) {
                                    items(found.playlists, key = { it.id }) { playlist ->
                                        CatalogueCard(
                                            title = playlist.title,
                                            subtitle = playlist.author ?: "Playlist",
                                            coverUrl = playlist.cover,
                                            onClick = { viewModel.openPlaylist(playlist.id) },
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    val title = viewModel.detailTitle
    if (title != null) {
        ModalBottomSheet(
            onDismissRequest = { viewModel.closeDetail() },
            containerColor = VynlColors.surface,
        ) {
            Column(modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 30.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    ArtTile(coverUrl = viewModel.detailCover, size = 64.dp, radius = VynlRadius.art)
                    Spacer(Modifier.width(14.dp))
                    Column {
                        Text(
                            text = title,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                            color = VynlColors.text,
                            fontSize = 17.sp,
                            fontWeight = FontWeight.W800,
                            fontFamily = VynlFonts.display,
                        )
                        Spacer(Modifier.height(2.dp))
                        Text(
                            text = viewModel.detailSubtitle.orEmpty(),
                            color = VynlColors.dim,
                            fontSize = 13.sp,
                            fontFamily = VynlFonts.display,
                        )
                    }
                }
                Spacer(Modifier.height(10.dp))
                viewModel.detailTracks.forEach { track ->
                    TrackTile(
                        track = track,
                        active = playback.track?.id == track.id,
                        onTap = {
                            viewModel.player.play(viewModel.detailTracks, viewModel.detailTracks.indexOf(track))
                        },
                    )
                }
            }
        }
    }
}
