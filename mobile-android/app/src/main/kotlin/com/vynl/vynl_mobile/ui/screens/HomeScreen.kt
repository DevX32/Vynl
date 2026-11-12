package com.vynl.vynl_mobile.ui.screens

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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.LibraryMusic
import androidx.compose.material.icons.rounded.Pause
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.SkipNext
import androidx.compose.material.icons.rounded.SkipPrevious
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
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
import com.vynl.vynl_mobile.ui.components.EmptyState
import com.vynl.vynl_mobile.ui.components.SectionHeader
import com.vynl.vynl_mobile.ui.components.TrackTile
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius
import kotlinx.coroutines.delay

private val quotes = listOf(
    "where words fail, music speaks",
    "music is the shorthand of emotion",
    "life is a song. love is the music",
    "without music, life would be a mistake",
    "music is the universal language of mankind",
    "one good thing about music, when it hits you, you feel no pain",
    "music expresses that which cannot be put into words",
)

private fun greetingFor(hour: Int): String = when {
    hour < 6 -> "Good Night"
    hour < 12 -> "Good Morning"
    hour < 18 -> "Good Afternoon"
    else -> "Good Evening"
}

@Composable
fun HomeScreen(viewModel: AppViewModel) {
    val mix by viewModel.mix.collectAsStateWithLifecycle()
    val playback by viewModel.player.state.collectAsStateWithLifecycle()
    val remote by viewModel.remote.collectAsStateWithLifecycle()
    val now = java.util.Calendar.getInstance()

    LaunchedEffect(Unit) {
        while (true) {
            viewModel.refreshRemote()
            delay(5000)
        }
    }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = androidx.compose.foundation.layout.PaddingValues(bottom = 30.dp),
    ) {
        item {
            Column(modifier = Modifier.padding(start = 20.dp, end = 20.dp, top = 4.dp)) {
                Text(
                    text = greetingFor(now.get(java.util.Calendar.HOUR_OF_DAY)),
                    color = VynlColors.text,
                    fontSize = 26.sp,
                    fontWeight = FontWeight.W800,
                    letterSpacing = (-0.6).sp,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.height(6.dp))
                Text(
                    text = quotes[(now.get(java.util.Calendar.DAY_OF_MONTH) - 1) % quotes.size],
                    color = VynlColors.dim,
                    fontSize = 13.sp,
                    fontFamily = VynlFonts.display,
                )
            }
        }

        val desktop = remote?.track
        if (desktop != null) {
            item {
                RemoteCard(
                    title = desktop.title,
                    artist = desktop.artist,
                    cover = desktop.cover,
                    playing = remote?.playing == true,
                    onToggle = { viewModel.remoteAction("toggle") },
                    onNext = { viewModel.remoteAction("next") },
                    onPrevious = { viewModel.remoteAction("previous") },
                )
            }
        }

        if (viewModel.mixLoading) {
            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 48.dp),
                    horizontalArrangement = Arrangement.Center,
                ) {
                    CircularProgressIndicator(
                        modifier = Modifier.height(22.dp),
                        color = VynlColors.accent,
                        strokeWidth = 2.dp,
                    )
                }
            }
        }

        viewModel.mixError?.let { message ->
            item {
                EmptyState(
                    icon = Icons.Rounded.LibraryMusic,
                    title = "No mix yet",
                    subtitle = message,
                )
            }
        }

        if (mix.isNotEmpty()) {
            item { SectionHeader("MADE FOR YOU") }
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
}

/** What the desktop is playing, with the controls for it. */
@Composable
private fun RemoteCard(
    title: String,
    artist: String,
    cover: String?,
    playing: Boolean,
    onToggle: () -> Unit,
    onNext: () -> Unit,
    onPrevious: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 16.dp)
            .clip(RoundedCornerShape(VynlRadius.card))
            .background(VynlColors.surfaceRaised)
            .padding(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        ArtTile(coverUrl = cover, size = 46.dp, radius = VynlRadius.thumb)
        Spacer(Modifier.width(12.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = VynlColors.text,
                fontSize = 13.5.sp,
                fontWeight = FontWeight.W700,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(2.dp))
            Text(
                text = "On the desktop · $artist",
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = VynlColors.dim,
                fontSize = 12.sp,
                fontFamily = VynlFonts.display,
            )
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            VynlControl(
                onTap = onPrevious,
                icon = Icons.Rounded.SkipPrevious,
                style = VynlControlStyle.plain,
                size = 28.dp,
                iconSize = 20.dp,
            )
            Spacer(Modifier.width(8.dp))
            VynlControl(
                onTap = onToggle,
                icon = if (playing) Icons.Rounded.Pause else Icons.Rounded.PlayArrow,
                style = VynlControlStyle.accent,
                color = VynlColors.accent,
                size = 34.dp,
                iconSize = 20.dp,
            )
            Spacer(Modifier.width(8.dp))
            VynlControl(
                onTap = onNext,
                icon = Icons.Rounded.SkipNext,
                style = VynlControlStyle.plain,
                size = 28.dp,
                iconSize = 20.dp,
            )
        }
    }
}
