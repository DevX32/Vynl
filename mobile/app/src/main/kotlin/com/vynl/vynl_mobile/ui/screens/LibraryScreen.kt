package com.vynl.vynl_mobile.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.LibraryMusic
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material.icons.rounded.Shuffle
import androidx.compose.material3.Icon
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
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

@Composable
fun LibraryScreen(viewModel: AppViewModel) {
    val library by viewModel.library.collectAsStateWithLifecycle()
    val playback by viewModel.player.state.collectAsStateWithLifecycle()
    var query by remember { mutableStateOf("") }

    val needle = query.trim().lowercase()
    val tracks = library.filter { track ->
        needle.isEmpty() ||
            track.title.lowercase().contains(needle) ||
            track.artist.lowercase().contains(needle) ||
            track.album.orEmpty().lowercase().contains(needle)
    }

    Column(modifier = Modifier.fillMaxSize()) {
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
                    "Search songs, artists, albums",
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

        if (library.isEmpty()) {
            EmptyState(
                icon = Icons.Rounded.LibraryMusic,
                title = "Library",
                subtitle = "Songs you download land here, and they play without a connection.",
                modifier = Modifier.fillMaxSize(),
            )
            return@Column
        }

        SectionHeader(
            title = if (needle.isEmpty()) "TRACKS" else "RESULTS",
            trailing = if (needle.isEmpty() && tracks.isNotEmpty()) {
                {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .padding(start = 10.dp)
                            .then(
                                Modifier,
                            ),
                    ) {
                        Icon(
                            Icons.Rounded.Shuffle,
                            contentDescription = null,
                            tint = VynlColors.accent,
                            modifier = Modifier.size(16.dp),
                        )
                        Spacer(Modifier.size(6.dp))
                        Text(
                            text = "Shuffle all",
                            color = VynlColors.accent,
                            fontSize = 12.5.sp,
                            fontWeight = FontWeight.W700,
                            fontFamily = VynlFonts.display,
                        )
                    }
                }
            } else {
                null
            },
        )

        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(bottom = 30.dp),
        ) {
            items(tracks, key = { it.id }) { track ->
                TrackTile(
                    track = track.toCatalogue(),
                    coverPath = track.coverPath,
                    active = playback.track?.id == track.id,
                    subtitle = track.album,
                    onTap = {
                        val index = tracks.indexOf(track)
                        viewModel.playLocal(tracks, index)
                    },
                )
            }
        }
    }
}
