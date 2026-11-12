package com.vynl.vynl_mobile.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.material.icons.rounded.MoreHoriz
import androidx.compose.material.icons.rounded.MusicNote
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.vynl.vynl_mobile.data.CatalogueTrack
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@Composable
fun ArtTile(
    coverUrl: String?,
    size: Dp = 56.dp,
    radius: Dp = VynlRadius.thumb,
    icon: androidx.compose.ui.graphics.vector.ImageVector = Icons.Rounded.MusicNote,
    accent: Color = VynlColors.accent,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .size(size)
            .clip(RoundedCornerShape(radius)),
    ) {
        if (coverUrl.isNullOrEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(
                        Brush.linearGradient(
                            listOf(accent.copy(alpha = 0.20f), VynlColors.surfaceRaised),
                        ),
                    ),
                contentAlignment = Alignment.Center,
            ) {
                Icon(icon, contentDescription = null, tint = VynlColors.faint, modifier = Modifier.size(size * 0.42f))
            }
        } else {
            AsyncImage(
                model = coverUrl,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}

@Composable
fun TrackTile(
    track: CatalogueTrack,
    onTap: (() -> Unit)? = null,
    onMore: (() -> Unit)? = null,
    active: Boolean = false,
    subtitle: String? = null,
    durationLabel: String? = null,
    showStatus: Boolean = true,
    downloaded: Boolean = false,
    modifier: Modifier = Modifier,
) {
    val row = Row(
        modifier = modifier
            .fillMaxWidth()
            .then(
                if (onTap == null) {
                    Modifier
                } else {
                    Modifier
                        .clip(RoundedCornerShape(VynlRadius.tile))
                        .clickable { onTap() }
                        .background(
                            if (active) VynlColors.accent.copy(alpha = 0.07f) else Color.Transparent
                        )
                },
            )
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box {
            ArtTile(coverUrl = track.cover, size = 56.dp)
            if (active) {
                Box(
                    modifier = Modifier
                        .size(56.dp)
                        .clip(RoundedCornerShape(VynlRadius.thumb))
                        .border(1.5.dp, VynlColors.accent.copy(alpha = 0.55f), RoundedCornerShape(VynlRadius.thumb)),
                )
            }
        }
        Spacer(Modifier.width(14.dp))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = track.title,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = if (active) VynlColors.accent else VynlColors.text,
                fontWeight = FontWeight.W700,
                fontSize = 14.5.sp,
                letterSpacing = (-0.1).sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(2.dp))
            Text(
                text = track.artist,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = VynlColors.dim,
                fontSize = 12.5.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(1.dp))
            Text(
                text = subtitle ?: track.album.orEmpty(),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = VynlColors.faint,
                fontSize = 11.5.sp,
                fontFamily = VynlFonts.display,
            )
        }
        Spacer(Modifier.width(10.dp))
        Column(horizontalAlignment = Alignment.End) {
            Text(
                text = durationLabel ?: formatDuration(track.duration),
                color = VynlColors.faint,
                fontSize = 11.5.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(6.dp))
            if (onMore != null) {
                Box(
                    modifier = Modifier
                        .size(width = 30.dp, height = 22.dp)
                        .clickable { onMore() },
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Rounded.MoreHoriz,
                        contentDescription = "Track options",
                        tint = VynlColors.faint,
                        modifier = Modifier.size(18.dp),
                    )
                }
            } else if (showStatus) {
                Icon(
                    Icons.Rounded.MusicNote,
                    contentDescription = null,
                    tint = VynlColors.faint,
                    modifier = Modifier.size(15.dp),
                )
            }
        }
    }
}

@Composable
fun SectionHeader(
    title: String,
    modifier: Modifier = Modifier,
    trailing: @Composable (() -> Unit)? = null,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(start = 20.dp, end = 16.dp, top = 18.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = title,
            modifier = Modifier.weight(1f),
            color = VynlColors.faint,
            fontSize = 11.sp,
            fontWeight = FontWeight.W800,
            letterSpacing = 1.3.sp,
            fontFamily = VynlFonts.display,
        )
        trailing?.invoke()
    }
}

enum class VynlControlStyle { accent, raised, plain }

@Composable
fun VynlControl(
    onTap: () -> Unit,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    modifier: Modifier = Modifier,
    style: VynlControlStyle = VynlControlStyle.plain,
    size: Dp = 40.dp,
    iconSize: Dp = 20.dp,
    color: Color? = null,
) {
    val background = when (style) {
        VynlControlStyle.accent -> color ?: VynlColors.accent
        VynlControlStyle.raised -> color ?: VynlColors.surfaceRaised
        VynlControlStyle.plain -> color ?: Color.Transparent
    }
    val foreground = when (style) {
        VynlControlStyle.accent -> VynlColors.accentOn
        else -> VynlColors.dim
    }
    Box(
        modifier = modifier
            .graphicsLayer { rotationZ = 45f }
            .size(size)
            .clip(RoundedCornerShape(VynlRadius.control))
            .background(background)
            .clickable { onTap() },
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = foreground,
            modifier = Modifier
                .size(iconSize)
                .graphicsLayer { rotationZ = -45f },
        )
    }
}

@Composable
fun VynlSwitch(
    value: Boolean,
    onChanged: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    accent: Color = VynlColors.accent,
) {
    Box(
        modifier = modifier
            .size(width = 48.dp, height = 44.dp)
            .clickable { onChanged(!value) },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier = Modifier
                .size(width = 36.dp, height = 20.dp)
                .clip(RoundedCornerShape(VynlRadius.control))
                .background(
                    if (value) accent.copy(alpha = 0.11f) else VynlColors.text.copy(alpha = 0.20f),
                ),
            contentAlignment = if (value) Alignment.CenterEnd else Alignment.CenterStart,
        ) {
            Box(
                modifier = Modifier
                    .padding(horizontal = 3.dp)
                    .size(14.dp)
                    .clip(RoundedCornerShape(3.dp))
                    .background(if (value) accent else VynlColors.faint),
            )
        }
    }
}

@Composable
fun EmptyState(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    title: String,
    subtitle: String,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 44.dp, vertical = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Box(
            modifier = Modifier
                .size(84.dp)
                .clip(RoundedCornerShape(VynlRadius.control))
                .background(VynlColors.surfaceRaised),
            contentAlignment = Alignment.Center,
        ) {
            Icon(icon, contentDescription = null, tint = VynlColors.faint, modifier = Modifier.size(34.dp))
        }
        Spacer(Modifier.height(22.dp))
        Text(
            text = title,
            color = VynlColors.text,
            fontSize = 17.sp,
            fontWeight = FontWeight.W800,
            letterSpacing = (-0.2).sp,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            text = subtitle,
            color = VynlColors.dim,
            fontSize = 13.5.sp,
            lineHeight = 21.sp,
            fontFamily = VynlFonts.display,
        )
    }
}

@Composable
fun CatalogueCard(
    title: String,
    subtitle: String,
    coverUrl: String?,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .width(140.dp)
            .clip(RoundedCornerShape(VynlRadius.card))
            .clickable { onClick() },
    ) {
        ArtTile(
            coverUrl = coverUrl,
            size = 140.dp,
            radius = VynlRadius.card,
            modifier = Modifier.clip(RoundedCornerShape(VynlRadius.card)),
        )
        Spacer(Modifier.height(10.dp))
        Text(
            text = title,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            color = VynlColors.text,
            fontSize = 13.sp,
            fontWeight = FontWeight.W700,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(2.dp))
        Text(
            text = subtitle,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            color = VynlColors.dim,
            fontSize = 11.5.sp,
            fontFamily = VynlFonts.display,
        )
    }
}

fun formatDuration(seconds: Double): String {
    val total = seconds.toInt().coerceAtLeast(0)
    val minutes = (total / 60) % 60
    val rest = total % 60
    val hours = total / 3600
    return if (hours > 0) {
        "$hours:${minutes.toString().padStart(2, '0')}:${rest.toString().padStart(2, '0')}"
    } else {
        "$minutes:${rest.toString().padStart(2, '0')}"
    }
}
