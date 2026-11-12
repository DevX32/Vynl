package com.vynl.vynl_mobile.ui.shell

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Home
import androidx.compose.material.icons.rounded.LibraryMusic
import androidx.compose.material.icons.rounded.Pause
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.QueueMusic
import androidx.compose.material.icons.rounded.SkipNext
import androidx.compose.material.icons.rounded.SkipPrevious
import androidx.compose.material.icons.rounded.TravelExplore
import androidx.compose.material.icons.rounded.Tune
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.ArtTile
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.ui.screens.BrowseScreen
import com.vynl.vynl_mobile.ui.screens.HomeScreen
import com.vynl.vynl_mobile.ui.screens.LibraryScreen
import com.vynl.vynl_mobile.ui.screens.PlayerScreen
import com.vynl.vynl_mobile.ui.screens.PlaylistsScreen
import com.vynl.vynl_mobile.ui.screens.SettingsScreen
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

private val tabTitles = listOf("Home", "Library", "Playlists", "Browse")

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeShell(viewModel: AppViewModel = viewModel()) {
    var playerOpen by remember { mutableStateOf(false) }
    var settingsOpen by remember { mutableStateOf(false) }
    val playback by viewModel.player.state.collectAsStateWithLifecycle()

    Scaffold(
        containerColor = VynlColors.bg,
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = tabTitles[viewModel.tab.coerceIn(0, tabTitles.lastIndex)],
                        color = VynlColors.text,
                        fontSize = 21.sp,
                        fontWeight = FontWeight.W600,
                        letterSpacing = (-0.4).sp,
                        fontFamily = VynlFonts.display,
                    )
                },
                actions = {
                    IconButton(onClick = { settingsOpen = true }) {
                        Icon(
                            Icons.Rounded.Tune,
                            contentDescription = "Settings",
                            tint = VynlColors.dim,
                            modifier = Modifier.size(22.dp),
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = Color.Transparent,
                    titleContentColor = VynlColors.text,
                ),
            )
        },
        bottomBar = {
            Column {
                MiniPlayer(
                    playback = playback,
                    onOpen = { playerOpen = true },
                    onToggle = { viewModel.player.toggle() },
                    onNext = { viewModel.player.next() },
                    onPrevious = { viewModel.player.previous() },
                )
                NavigationBar(
                    containerColor = VynlColors.surface,
                    tonalElevation = 0.dp,
                ) {
                    val items = listOf(
                        Triple("Home", Icons.Rounded.Home, Icons.Rounded.Home),
                        Triple("Library", Icons.Rounded.LibraryMusic, Icons.Rounded.LibraryMusic),
                        Triple("Playlists", Icons.Rounded.QueueMusic, Icons.Rounded.QueueMusic),
                        Triple("Browse", Icons.Rounded.TravelExplore, Icons.Rounded.TravelExplore),
                    )
                    items.forEachIndexed { index, item ->
                        val selected = viewModel.tab == index
                        NavigationBarItem(
                            selected = selected,
                            onClick = { viewModel.selectTab(index) },
                            icon = {
                                Icon(
                                    item.second,
                                    contentDescription = item.first,
                                    modifier = Modifier.size(22.dp),
                                )
                            },
                            label = {
                                Text(
                                    item.first,
                                    fontSize = 10.5.sp,
                                    lineHeight = 12.sp,
                                    fontWeight = if (selected) FontWeight.W700 else FontWeight.W600,
                                    letterSpacing = 0.1.sp,
                                    fontFamily = VynlFonts.display,
                                )
                            },
                            colors = NavigationBarItemDefaults.colors(
                                selectedIconColor = VynlColors.accent,
                                unselectedIconColor = VynlColors.faint,
                                selectedTextColor = VynlColors.text,
                                unselectedTextColor = VynlColors.faint,
                                indicatorColor = VynlColors.accent.copy(alpha = 0.16f),
                            ),
                        )
                    }
                }
            }
        },
    ) { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            when (viewModel.tab) {
                0 -> HomeScreen(viewModel)
                1 -> LibraryScreen(viewModel)
                2 -> PlaylistsScreen(viewModel)
                else -> BrowseScreen(viewModel)
            }
        }
    }

    AnimatedVisibility(
        visible = playerOpen,
        enter = fadeIn() + scaleIn(initialScale = 0.94f),
        exit = fadeOut() + scaleOut(targetScale = 0.94f),
    ) {
        PlayerScreen(
            playback = playback,
            onClose = { playerOpen = false },
            onToggle = { viewModel.player.toggle() },
            onNext = { viewModel.player.next() },
            onPrevious = { viewModel.player.previous() },
            onSeek = { viewModel.player.seekTo(it) },
        )
    }

    AnimatedVisibility(
        visible = settingsOpen,
        enter = fadeIn(),
        exit = fadeOut(),
    ) {
        SettingsScreen(onClose = { settingsOpen = false })
    }
}

@Composable
private fun MiniPlayer(
    playback: com.vynl.vynl_mobile.playback.PlaybackState,
    onOpen: () -> Unit,
    onToggle: () -> Unit,
    onNext: () -> Unit,
    onPrevious: () -> Unit,
) {
    val track = playback.track ?: return
    val progress = if (playback.durationMs > 0) {
        (playback.positionMs.toFloat() / playback.durationMs.toFloat()).coerceIn(0f, 1f)
    } else {
        0f
    }
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 10.dp, end = 10.dp, bottom = 6.dp)
            .clip(RoundedCornerShape(VynlRadius.card))
            .background(VynlColors.surfaceRaised)
            .clickable { onOpen() },
    ) {
        Column {
            LinearProgressIndicator(
                progress = { progress },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(2.dp),
                color = VynlColors.accent,
                trackColor = androidx.compose.ui.graphics.Color.Transparent,
                drawStopIndicator = {},
            )
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(66.dp)
                    .padding(start = 8.dp, end = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                ArtTile(coverUrl = track.cover, size = 46.dp, radius = VynlRadius.thumb)
                Spacer(Modifier.width(12.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = track.title,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        color = VynlColors.text,
                        fontSize = 13.5.sp,
                        fontWeight = FontWeight.W700,
                        letterSpacing = (-0.1).sp,
                        fontFamily = VynlFonts.display,
                    )
                    Spacer(Modifier.height(2.dp))
                    Text(
                        text = track.artist,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        color = VynlColors.dim,
                        fontSize = 12.sp,
                        fontFamily = VynlFonts.display,
                    )
                }
                Row(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    VynlControl(
                        onTap = onPrevious,
                        icon = Icons.Rounded.SkipPrevious,
                        style = VynlControlStyle.plain,
                        size = 30.dp,
                        iconSize = 23.dp,
                    )
                    VynlControl(
                        onTap = onToggle,
                        icon = if (playback.playing) Icons.Rounded.Pause else Icons.Rounded.PlayArrow,
                        style = VynlControlStyle.accent,
                        color = VynlColors.accent,
                        size = 36.dp,
                        iconSize = 22.dp,
                    )
                    VynlControl(
                        onTap = onNext,
                        icon = Icons.Rounded.SkipNext,
                        style = VynlControlStyle.plain,
                        size = 30.dp,
                        iconSize = 23.dp,
                    )
                }
            }
        }
    }
}
