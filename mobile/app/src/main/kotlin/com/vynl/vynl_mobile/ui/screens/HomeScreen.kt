package com.vynl.vynl_mobile.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.CloudSync
import androidx.compose.material.icons.rounded.LibraryMusic
import androidx.compose.material.icons.rounded.MusicNote
import androidx.compose.material.icons.rounded.MusicOff
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.Shuffle
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import coil3.compose.AsyncImage
import com.vynl.vynl_mobile.data.LocalTrack
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.ArtTile
import com.vynl.vynl_mobile.ui.components.SectionHeader
import com.vynl.vynl_mobile.ui.components.VynlControl
import com.vynl.vynl_mobile.ui.components.VynlControlStyle
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius
import java.io.File
import java.util.Calendar
import kotlin.math.sqrt
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
    val library by viewModel.library.collectAsStateWithLifecycle()
    val playback by viewModel.player.state.collectAsStateWithLifecycle()
    var clock by remember { mutableStateOf(System.currentTimeMillis()) }

    LaunchedEffect(Unit) {
        while (true) {
            delay(60_000)
            clock = System.currentTimeMillis()
        }
    }

    val now = remember(clock) { Calendar.getInstance().apply { timeInMillis = clock } }
    val downloaded = remember(library) { library.filter { it.isDownloaded } }
    val recent = remember(downloaded) { downloaded.take(6) }
    val featured = remember(downloaded) {
        if (downloaded.isEmpty()) {
            null
        } else {
            downloaded[(now.get(Calendar.DAY_OF_MONTH) * 31 + now.get(Calendar.MONTH)) % downloaded.size]
        }
    }
    val greeting = greetingFor(now.get(Calendar.HOUR_OF_DAY))
    val quote = quotes[(now.get(Calendar.DAY_OF_MONTH) - 1).coerceAtLeast(0) % quotes.size]

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(bottom = 28.dp),
    ) {
        item {
            HomeHeader(
                greeting = greeting,
                name = viewModel.greetingName,
                quote = quote,
            )
        }

        if (featured == null) {
            item {
                Spacer(Modifier.height(26.dp))
                NothingYet(
                    hasLibrary = library.isNotEmpty(),
                    onSync = { viewModel.syncLibrary() },
                    onBrowse = { viewModel.selectTab(3) },
                )
            }
        } else {
            item {
                Spacer(Modifier.height(18.dp))
                FeaturedCard(
                    track = featured,
                    onTap = {
                        viewModel.playLocal(downloaded, downloaded.indexOf(featured))
                    },
                )
            }
            item {
                Spacer(Modifier.height(18.dp))
                Row(
                    modifier = Modifier.padding(horizontal = 20.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    ActionPill(
                        icon = Icons.Rounded.Shuffle,
                        label = "Shuffle All",
                        onClick = {
                            viewModel.playLocal(downloaded.shuffled(), 0)
                        },
                    )
                    ActionPill(
                        icon = Icons.Rounded.CloudSync,
                        label = if (viewModel.syncing) "Syncing" else "Sync",
                        enabled = !viewModel.syncing,
                        loading = viewModel.syncing,
                        onClick = { viewModel.syncLibrary() },
                    )
                    ActionPill(
                        icon = Icons.Rounded.LibraryMusic,
                        label = "Library",
                        onClick = { viewModel.selectTab(1) },
                    )
                }
            }
            if (recent.isNotEmpty()) {
                item {
                    SectionHeader(
                        title = "RECENTLY ADDED",
                        trailing = {
                            Text(
                                text = "View all",
                                color = VynlColors.faint,
                                fontSize = 11.5.sp,
                                fontFamily = VynlFonts.display,
                                modifier = Modifier
                                    .clip(RoundedCornerShape(VynlRadius.control))
                                    .clickable { viewModel.selectTab(1) }
                                    .padding(horizontal = 8.dp, vertical = 6.dp),
                            )
                        },
                    )
                }
                item {
                    RecentGrid(
                        tracks = recent,
                        activeId = playback.track?.id,
                        onTap = { track ->
                            viewModel.playLocal(downloaded, downloaded.indexOf(track))
                        },
                    )
                }
            }
        }

    }
}

@Composable
private fun HomeHeader(greeting: String, name: String, quote: String) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 20.dp, end = 20.dp, top = 4.dp),
    ) {
        Text(
            text = buildAnnotatedString {
                append(greeting)
                val display = name.trim()
                if (display.isNotEmpty()) {
                    append(", ")
                    withStyle(
                        SpanStyle(
                            fontFamily = VynlFonts.serif,
                            fontStyle = FontStyle.Italic,
                            fontWeight = FontWeight.W700,
                            letterSpacing = 0.sp,
                            color = VynlColors.accent,
                        ),
                    ) {
                        append(display)
                    }
                }
            },
            color = VynlColors.text,
            fontSize = 27.sp,
            fontWeight = FontWeight.W600,
            letterSpacing = (-0.6).sp,
            lineHeight = 33.sp,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(8.dp))
        Row(modifier = Modifier.height(IntrinsicSize.Min)) {
            Box(
                modifier = Modifier
                    .width(2.dp)
                    .fillMaxHeight()
                    .background(VynlColors.lineStrong),
            )
            Spacer(Modifier.width(10.dp))
            Text(
                text = quote,
                color = VynlColors.dim,
                fontSize = 12.5.sp,
                fontStyle = FontStyle.Italic,
                lineHeight = 17.sp,
                fontFamily = VynlFonts.display,
            )
        }
    }
}

@Composable
private fun NothingYet(
    hasLibrary: Boolean,
    onSync: () -> Unit,
    onBrowse: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            modifier = Modifier
                .size(84.dp)
                .clip(RoundedCornerShape(VynlRadius.control))
                .background(VynlColors.surfaceRaised),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                if (hasLibrary) Icons.Rounded.MusicOff else Icons.Rounded.CloudSync,
                contentDescription = null,
                tint = VynlColors.faint,
                modifier = Modifier.size(34.dp),
            )
        }
        Spacer(Modifier.height(22.dp))
        Text(
            text = if (hasLibrary) "Nothing downloaded yet" else "Nothing synced yet",
            color = VynlColors.text,
            fontSize = 17.sp,
            fontWeight = FontWeight.W800,
            letterSpacing = (-0.2).sp,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            text = if (hasLibrary) {
                "Download a few tracks on Wi-Fi so they can play offline."
            } else {
                "Pair with your desktop under Settings, then pull down to sync."
            },
            color = VynlColors.dim,
            fontSize = 13.5.sp,
            lineHeight = 21.sp,
            textAlign = TextAlign.Center,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(18.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            ActionPill(
                icon = Icons.Rounded.CloudSync,
                label = if (hasLibrary) "Sync now" else "Sync now",
                onClick = onSync,
            )
            ActionPill(
                icon = Icons.Rounded.MusicNote,
                label = "Browse",
                onClick = onBrowse,
            )
        }
    }
}

@Composable
private fun ActionPill(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    label: String,
    onClick: () -> Unit,
    enabled: Boolean = true,
    loading: Boolean = false,
) {
    Row(
        modifier = Modifier
            .clip(RoundedCornerShape(VynlRadius.control))
            .background(if (enabled) VynlColors.surfaceRaised else VynlColors.surfaceRaised.copy(alpha = 0.45f))
            .border(1.dp, VynlColors.line, RoundedCornerShape(VynlRadius.control))
            .then(if (enabled) Modifier.clickable { onClick() } else Modifier)
            .padding(horizontal = 14.dp, vertical = 9.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (loading) {
            CircularProgressIndicator(
                modifier = Modifier.size(16.dp),
                strokeWidth = 2.dp,
                color = VynlColors.dim,
            )
        } else {
            Icon(
                icon,
                contentDescription = null,
                tint = if (enabled) VynlColors.dim else VynlColors.faint,
                modifier = Modifier.size(16.dp),
            )
        }
        Spacer(Modifier.width(8.dp))
        Text(
            text = label,
            color = if (enabled) VynlColors.dim else VynlColors.faint,
            fontSize = 12.sp,
            fontFamily = VynlFonts.display,
        )
    }
}

private val featuredScrim = listOf(
    0.0f to Color(0x0008080C),
    0.28f to Color(0x0008080C),
    0.55f to Color(0x4708080C),
    0.82f to Color(0xB808080C),
    1.0f to Color(0xE608080C),
)

@Composable
private fun FeaturedCard(track: LocalTrack, onTap: () -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp)
            .height(152.dp)
            .clip(RoundedCornerShape(VynlRadius.card))
            .background(VynlColors.surfaceRaised)
            .clickable { onTap() },
    ) {
        val local = track.coverPath?.takeIf { it.isNotEmpty() && File(it).exists() }?.let { File(it) }
        val model: Any? = local ?: track.cover?.takeIf { it.isNotEmpty() }
        if (model != null) {
            AsyncImage(
                model = model,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )
        } else {
            Box(modifier = Modifier.fillMaxSize()) {
                Canvas(modifier = Modifier.fillMaxSize()) {
                    val center = Offset(size.width / 2f, size.height / 2f)
                    val reach = sqrt(size.width * size.width + size.height * size.height) / 2f
                    var radius = 0f
                    while (radius < reach) {
                        drawCircle(
                            color = VynlColors.bgTop,
                            radius = radius,
                            center = center,
                            style = Stroke(width = 2.dp.toPx()),
                        )
                        radius += 4.dp.toPx()
                    }
                }
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(
                            Brush.radialGradient(
                                colors = listOf(
                                    VynlColors.accent.copy(alpha = 0.08f),
                                    VynlColors.accent.copy(alpha = 0f),
                                ),
                                center = Offset(0f, 0f),
                                radius = 420f,
                            ),
                        ),
                )
                Icon(
                    Icons.Rounded.MusicNote,
                    contentDescription = null,
                    tint = VynlColors.faint,
                    modifier = Modifier
                        .size(40.dp)
                        .align(Alignment.Center),
                )
            }
        }
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Brush.verticalGradient(colorStops = *featuredScrim.toTypedArray())),
        )
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(
                    Brush.radialGradient(
                        colors = listOf(Color(0x73000000), Color.Transparent),
                        center = Offset(0f, 0f),
                        radius = 520f,
                    ),
                ),
        )
        Column(
            modifier = Modifier
                .align(Alignment.BottomStart)
                .padding(start = 20.dp, end = 76.dp, bottom = 18.dp),
        ) {
            Text(
                text = "Jump Back In",
                color = Color(0xBFFFFFFF),
                fontSize = 11.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(6.dp))
            Text(
                text = track.title,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = Color.White,
                fontSize = 18.sp,
                fontWeight = FontWeight.W600,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(2.dp))
            Text(
                text = track.artist,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                color = Color(0xB3FFFFFF),
                fontSize = 13.sp,
                fontFamily = VynlFonts.display,
            )
        }
        Box(
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(end = 18.dp, bottom = 18.dp),
        ) {
            VynlControl(
                onTap = onTap,
                icon = Icons.Rounded.PlayArrow,
                style = VynlControlStyle.accent,
                size = 40.dp,
                iconSize = 24.dp,
            )
        }
    }
}

@Composable
private fun RecentGrid(
    tracks: List<LocalTrack>,
    activeId: String?,
    onTap: (LocalTrack) -> Unit,
) {
    BoxWithConstraints(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp),
    ) {
        val gap = 12.dp
        val columns = (maxWidth / 148.dp).toInt().coerceIn(2, 4)
        val cardWidth = (maxWidth - gap * (columns - 1)) / columns
        Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
            tracks.chunked(columns).forEach { row ->
                Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
                    row.forEach { track ->
                        RecentCard(
                            track = track,
                            width = cardWidth,
                            active = track.id == activeId,
                            onTap = { onTap(track) },
                        )
                    }
                    repeat(columns - row.size) {
                        Spacer(Modifier.width(cardWidth))
                    }
                }
            }
        }
    }
}

@Composable
private fun RecentCard(
    track: LocalTrack,
    width: Dp,
    active: Boolean,
    onTap: () -> Unit,
) {
    Column(
        modifier = Modifier
            .width(width)
            .clip(RoundedCornerShape(VynlRadius.card))
            .clickable { onTap() }
            .padding(4.dp),
    ) {
        ArtTile(
            coverUrl = track.cover,
            coverPath = track.coverPath,
            size = width - 8.dp,
            radius = VynlRadius.card,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            text = track.title,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            color = if (active) VynlColors.accent else VynlColors.text,
            fontSize = 12.5.sp,
            fontWeight = FontWeight.W700,
            letterSpacing = (-0.1).sp,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(2.dp))
        Text(
            text = track.artist,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            color = VynlColors.faint,
            fontSize = 11.sp,
            fontFamily = VynlFonts.display,
        )
    }
}
