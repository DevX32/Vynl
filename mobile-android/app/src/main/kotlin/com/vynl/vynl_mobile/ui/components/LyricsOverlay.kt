package com.vynl.vynl_mobile.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.vynl.vynl_mobile.data.LyricLine
import com.vynl.vynl_mobile.data.Lyrics
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts

/**
 * The lyrics of the song that is playing. A synced line lights up with the
 * music, and the words inside it light up one after another.
 */
@Composable
fun LyricsOverlay(
    lyrics: Lyrics,
    positionMs: Long,
    accent: Color,
    onClose: () -> Unit,
) {
    val listState = rememberLazyListState()
    val active = remember(lyrics, positionMs) { lyrics.indexAt(positionMs) }

    LaunchedEffect(active) {
        if (active >= 0) {
            val target = (active - 1).coerceAtLeast(0)
            if (target < listState.layoutInfo.totalItemsCount) {
                listState.animateScrollToItem(target)
            }
        }
    }

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
                .padding(top = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onClose) {
                Icon(
                    Icons.Rounded.KeyboardArrowDown,
                    contentDescription = "Close the lyrics",
                    tint = VynlColors.dim,
                    modifier = Modifier.size(26.dp),
                )
            }
            Text(
                text = "LYRICS",
                modifier = Modifier.weight(1f),
                textAlign = TextAlign.Center,
                color = VynlColors.faint,
                fontSize = 11.sp,
                fontWeight = FontWeight.W800,
                letterSpacing = 1.3.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.size(48.dp))
        }

        if (!lyrics.isSynced || lyrics.lines.isEmpty()) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(horizontal = 28.dp),
                verticalArrangement = Arrangement.Center,
            ) {
                Text(
                    text = lyrics.text.ifBlank { "No lyrics for this song." },
                    color = VynlColors.text,
                    fontSize = 16.sp,
                    lineHeight = 27.sp,
                    fontFamily = VynlFonts.lyrics,
                )
            }
            return@Column
        }

        LazyColumn(
            state = listState,
            modifier = Modifier.fillMaxSize(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                start = 28.dp,
                end = 28.dp,
                top = 20.dp,
                bottom = 60.dp,
            ),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            itemsIndexed(lyrics.lines, key = { index, line -> "${index}-${line.timeMs}" }) { index, line ->
                val isActive = index == active
                if (isActive) {
                    Text(
                        text = karaoke(line, positionMs, accent),
                        color = VynlColors.text,
                        fontSize = 17.sp,
                        lineHeight = 26.sp,
                        fontWeight = FontWeight.W700,
                        fontFamily = VynlFonts.lyrics,
                    )
                } else {
                    Text(
                        text = line.text,
                        color = VynlColors.faint,
                        fontSize = 16.sp,
                        lineHeight = 25.sp,
                        fontFamily = VynlFonts.lyrics,
                    )
                }
            }
        }
    }
}

/** The words of the line that are being sung right now. */
private fun karaoke(line: LyricLine, positionMs: Long, accent: Color): AnnotatedString {
    if (line.words.isEmpty()) {
        return AnnotatedString(line.text)
    }
    return buildAnnotatedString {
        var cursor = 0
        for (word in line.words) {
            val start = line.text.indexOf(word.text, cursor)
            if (start < 0) continue
            val end = start + word.text.length
            cursor = end
            val sung = positionMs >= word.timeMs
            pushStyle(
                SpanStyle(
                    color = if (sung) accent else VynlColors.faint,
                    fontWeight = if (sung) FontWeight.W800 else FontWeight.W600,
                ),
            )
            append(line.text.substring(start, end))
            pop()
            if (end < line.text.length) {
                append(line.text.substring(end, minOf(end + 1, line.text.length)))
                cursor = end + 1
            }
        }
        if (cursor < line.text.length) {
            pushStyle(SpanStyle(color = VynlColors.faint))
            append(line.text.substring(cursor))
            pop()
        }
    }
}
