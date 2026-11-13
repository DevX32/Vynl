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
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
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
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Snackbar
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.LaunchedEffect
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
import com.vynl.vynl_mobile.ui.components.PlaylistPickerSheet
import com.vynl.vynl_mobile.ui.components.TrackActions
import com.vynl.vynl_mobile.ui.components.TrackActionsSheet
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.ui.screens.BrowseScreen
import com.vynl.vynl_mobile.ui.screens.HomeScreen
import com.vynl.vynl_mobile.ui.screens.LibraryScreen
import com.vynl.vynl_mobile.ui.screens.PlayerScreen
import com.vynl.vynl_mobile.ui.screens.PlaylistsScreen
import com.vynl.vynl_mobile.ui.screens.SettingsScreen
import com.vynl.vynl_mobile.ui.screens.PlaylistDetailScreen
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

private val tabTitles = listOf("Home", "Library", "Playlists", "Browse")

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeShell(viewModel: AppViewModel = viewModel()) {
    var playerOpen by remember { mutableStateOf(false) }
    var settingsOpen by remember { mutableStateOf(false) }
    val snackbar = remember { SnackbarHostState() }
    val playback by viewModel.player.state.collectAsStateWithLifecycle()
    val playlists by viewModel.playlists.collectAsStateWithLifecycle()

    LaunchedEffect(playback.track?.id) {
        playback.track?.let { viewModel.ensureLyrics(it) }
    }

    LaunchedEffect(viewModel.notice) {
        val message = viewModel.notice ?: return@LaunchedEffect
        snackbar.showSnackbar(message)
        viewModel.dismissNotice()
    }

    Scaffold(
        containerColor = VynlColors.bg,
        snackbarHost = {
            SnackbarHost(snackbar) { data ->
                Snackbar(
                    snackbarData = data,
                    containerColor = VynlColors.surfaceRaised,
                    contentColor = VynlColors.text,
                    shape = RoundedCornerShape(VynlRadius.tile),
                )
            }
        },
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
                VynlNavigationBar(
                    selectedIndex = viewModel.tab,
                    onSelect = viewModel::selectTab,
                )
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

    val lyrics by viewModel.lyrics.collectAsStateWithLifecycle()

    AnimatedVisibility(
        visible = playerOpen,
        enter = fadeIn() + scaleIn(initialScale = 0.94f),
        exit = fadeOut() + scaleOut(targetScale = 0.94f),
    ) {
        PlayerScreen(
            playback = playback,
            accent = VynlColors.accent,
            queueTracks = viewModel.player.queueTracks(),
            onClose = { playerOpen = false },
            onToggle = { viewModel.player.toggle() },
            onNext = { viewModel.player.next() },
            onPrevious = { viewModel.player.previous() },
            onSeek = { viewModel.player.seekTo(it) },
            onRetryLyrics = { viewModel.retryLyrics() },
            onReorder = { from, to -> viewModel.player.moveInQueue(from, to) },
            onPlayAt = { index -> viewModel.player.playAt(index) },
            onToggleShuffle = { viewModel.player.toggleShuffle() },
            onToggleRepeat = { viewModel.player.toggleRepeat() },
            lyrics = lyrics,
            lyricsStatus = viewModel.lyricsStatus,
            lyricsAvailable = viewModel.lyricsAvailable,
            lyricsOpen = viewModel.lyricsOpen,
            onToggleLyrics = { viewModel.toggleLyrics() },
        )
    }

    AnimatedVisibility(
        visible = settingsOpen,
        enter = fadeIn(),
        exit = fadeOut(),
    ) {
        SettingsScreen(viewModel = viewModel, onClose = { settingsOpen = false })
    }

    AnimatedVisibility(
        visible = viewModel.openPlaylistId != null,
        enter = fadeIn() + scaleIn(initialScale = 0.96f),
        exit = fadeOut() + scaleOut(targetScale = 0.96f),
    ) {
        PlaylistDetailScreen(viewModel = viewModel, onClose = { viewModel.clearLocalPlaylist() })
    }

    val actionsTrack = viewModel.actionsTrack
    if (actionsTrack != null) {
        TrackActionsSheet(
            track = actionsTrack,
            actions = TrackActions(
                onGoToAlbum = {
                    viewModel.closeActions()
                    viewModel.openAlbum(actionsTrack.albumId.orEmpty())
                },
                onGoToArtist = {
                    viewModel.closeActions()
                    viewModel.openArtist(actionsTrack.artistId.orEmpty())
                },
                onAddToPlaylist = { viewModel.openPlaylistPicker(actionsTrack) },
            ),
            onDismiss = { viewModel.closeActions() },
        )
    }

    val pickerFor = viewModel.playlistPickerFor
    if (pickerFor != null) {
        PlaylistPickerSheet(
            playlists = playlists,
            onPick = { viewModel.addToPlaylist(it.id, pickerFor) },
            onCreate = { viewModel.createPlaylist(it, pickerFor) },
            onDismiss = { viewModel.closePlaylistPicker() },
        )
    }
}

@Composable
private fun VynlNavigationBar(
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
) {
    val bottomInset = WindowInsets.navigationBars.asPaddingValues().calculateBottomPadding()
    val items = listOf(
        Triple("Home", Icons.Rounded.Home, Icons.Rounded.Home),
        Triple("Library", Icons.Rounded.LibraryMusic, Icons.Rounded.LibraryMusic),
        Triple("Playlists", Icons.Rounded.QueueMusic, Icons.Rounded.QueueMusic),
        Triple("Browse", Icons.Rounded.TravelExplore, Icons.Rounded.TravelExplore),
    )
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(VynlColors.surface)
            .padding(bottom = bottomInset),
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(1.dp)
                .background(VynlColors.line),
        )
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .height(66.dp),
        ) {
            items.forEachIndexed { index, item ->
                val selected = selectedIndex == index
                Column(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxHeight()
                        .clickable { onSelect(index) },
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Spacer(Modifier.height(8.dp))
                    Box(
                        modifier = Modifier
                            .size(width = 64.dp, height = 32.dp)
                            .clip(RoundedCornerShape(VynlRadius.control))
                            .background(
                                if (selected) VynlColors.accent.copy(alpha = 0.16f) else Color.Transparent,
                            ),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            item.second,
                            contentDescription = item.first,
                            tint = if (selected) VynlColors.accent else VynlColors.faint,
                            modifier = Modifier.size(24.dp),
                        )
                    }
                    Spacer(Modifier.height(2.dp))
                    Text(
                        text = item.first,
                        color = if (selected) VynlColors.text else VynlColors.faint,
                        fontSize = 10.5.sp,
                        lineHeight = 12.sp,
                        fontWeight = if (selected) FontWeight.W700 else FontWeight.W600,
                        letterSpacing = 0.1.sp,
                        fontFamily = VynlFonts.display,
                    )
                }
            }
        }
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
