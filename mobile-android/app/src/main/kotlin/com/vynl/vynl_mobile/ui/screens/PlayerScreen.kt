package com.vynl.vynl_mobile.ui.screens

import android.os.Build
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.Lyrics
import androidx.compose.material.icons.rounded.Pause
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.SkipNext
import androidx.compose.material.icons.rounded.SkipPrevious
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.vynl.vynl_mobile.playback.PlaybackState
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.ui.components.VynlSeekBar
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

/**
 * The player: the art, the song, the seek bar and the transport, over the
 * cover as a backdrop.
 */
@Composable
fun PlayerScreen(
    playback: PlaybackState,
    accent: Color,
    onClose: () -> Unit,
    onToggle: () -> Unit,
    onNext: () -> Unit,
    onPrevious: () -> Unit,
    onSeek: (Long) -> Unit,
    onOpenLyrics: () -> Unit,
) {
    val track = playback.track

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg),
    ) {
        ArtBackdrop(
            coverUrl = track?.cover,
            modifier = Modifier.fillMaxSize(),
        )

        Column(
            modifier = Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding()
                .padding(horizontal = 26.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(onClick = onClose) {
                    Icon(
                        Icons.Rounded.KeyboardArrowDown,
                        contentDescription = "Close the player",
                        tint = VynlColors.dim,
                        modifier = Modifier.size(26.dp),
                    )
                }
                Text(
                    text = "NOW PLAYING",
                    modifier = Modifier.weight(1f),
                    textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                    color = VynlColors.faint,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.W800,
                    letterSpacing = 1.3.sp,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.width(48.dp))
            }

            Spacer(Modifier.weight(0.7f))

            AsyncImage(
                model = track?.cover,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier
                    .fillMaxWidth()
                    .aspectRatio(1f)
                    .clip(RoundedCornerShape(VynlRadius.hero))
                    .graphicsLayer { shadowElevation = 24.dp.toPx() },
            )

            Spacer(Modifier.height(28.dp))

            Text(
                text = track?.title ?: "Nothing playing",
                modifier = Modifier.fillMaxWidth(),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = VynlColors.text,
                fontSize = 20.sp,
                fontWeight = FontWeight.W800,
                letterSpacing = (-0.4).sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = track?.artist.orEmpty(),
                modifier = Modifier.fillMaxWidth(),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = VynlColors.dim,
                fontSize = 13.5.sp,
                fontFamily = VynlFonts.display,
            )

            Spacer(Modifier.height(18.dp))

            VynlSeekBar(
                positionMs = playback.positionMs,
                durationMs = playback.durationMs,
                bufferedMs = playback.bufferedMs,
                accent = accent,
                onSeek = onSeek,
            )

            Spacer(Modifier.height(16.dp))

            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(22.dp),
            ) {
                VynlControl(
                    onTap = onPrevious,
                    icon = Icons.Rounded.SkipPrevious,
                    style = VynlControlStyle.plain,
                    size = 44.dp,
                    iconSize = 32.dp,
                )
                VynlControl(
                    onTap = onToggle,
                    icon = if (playback.playing) Icons.Rounded.Pause else Icons.Rounded.PlayArrow,
                    style = VynlControlStyle.accent,
                    color = accent,
                    size = 62.dp,
                    iconSize = 34.dp,
                )
                VynlControl(
                    onTap = onNext,
                    icon = Icons.Rounded.SkipNext,
                    style = VynlControlStyle.plain,
                    size = 44.dp,
                    iconSize = 32.dp,
                )
            }

            Spacer(Modifier.weight(0.7f))

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(bottom = 24.dp),
                horizontalArrangement = Arrangement.Center,
            ) {
                IconButton(onClick = onOpenLyrics) {
                    Icon(
                        Icons.Rounded.Lyrics,
                        contentDescription = "Lyrics",
                        tint = VynlColors.dim,
                        modifier = Modifier.size(22.dp),
                    )
                }
            }
        }
    }
}

/** The cover, blown up and dimmed, behind the player. */
@Composable
private fun ArtBackdrop(coverUrl: String?, modifier: Modifier = Modifier) {
    Box(modifier = modifier) {
        if (!coverUrl.isNullOrEmpty()) {
            AsyncImage(
                model = coverUrl,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier
                    .fillMaxSize()
                    .graphicsLayer { scaleX = 1.35f; scaleY = 1.35f }
                    .then(
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                            Modifier.blur(72.dp)
                        } else {
                            Modifier
                        },
                    )
                    .drawWithContent {
                        drawContent()
                        drawRect(Color(0xFF0E0E10).copy(alpha = 0.72f))
                    },
            )
        }
    }
}
