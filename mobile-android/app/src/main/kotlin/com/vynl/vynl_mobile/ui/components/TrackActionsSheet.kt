package com.vynl.vynl_mobile.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Album
import androidx.compose.material.icons.rounded.Download
import androidx.compose.material.icons.rounded.Person
import androidx.compose.material.icons.rounded.PlaylistAdd
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.vynl.vynl_mobile.data.CatalogueTrack
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

/** The things a row can do. */
data class TrackActions(
    val onGoToAlbum: () -> Unit,
    val onGoToArtist: () -> Unit,
    val onAddToPlaylist: () -> Unit,
    val onDownload: () -> Unit,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrackActionsSheet(
    track: CatalogueTrack,
    actions: TrackActions,
    downloading: Boolean,
    progress: Float,
    onDismiss: () -> Unit,
) {
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        containerColor = VynlColors.surface,
    ) {
        Column(modifier = Modifier.padding(start = 8.dp, end = 8.dp, bottom = 30.dp)) {
            Column(modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp)) {
                Text(
                    text = track.title,
                    color = VynlColors.text,
                    fontSize = 15.sp,
                    fontWeight = FontWeight.W800,
                    fontFamily = VynlFonts.display,
                )
                Spacer(Modifier.height(2.dp))
                Text(
                    text = track.artist,
                    color = VynlColors.dim,
                    fontSize = 12.5.sp,
                    fontFamily = VynlFonts.display,
                )
            }
            Spacer(Modifier.height(10.dp))
            ActionRow(
                icon = Icons.Rounded.Album,
                title = "Go to album",
                enabled = !track.albumId.isNullOrEmpty(),
                onClick = actions.onGoToAlbum,
            )
            ActionRow(
                icon = Icons.Rounded.Person,
                title = "Go to artist",
                enabled = !track.artistId.isNullOrEmpty(),
                onClick = actions.onGoToArtist,
            )
            ActionRow(
                icon = Icons.Rounded.PlaylistAdd,
                title = "Add to playlist",
                onClick = actions.onAddToPlaylist,
            )
            ActionRow(
                icon = Icons.Rounded.Download,
                title = if (downloading) "Downloading" else "Download",
                enabled = !downloading,
                onClick = actions.onDownload,
            )
            if (downloading) {
                LinearProgressIndicator(
                    progress = { progress },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp)
                        .height(4.dp),
                    color = VynlColors.accent,
                    trackColor = VynlColors.line,
                    drawStopIndicator = {},
                )
            }
        }
    }
}

@Composable
private fun ActionRow(
    icon: ImageVector,
    title: String,
    onClick: () -> Unit,
    enabled: Boolean = true,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(VynlRadius.control))
            .clickable(enabled = enabled) { onClick() }
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Start,
    ) {
        Box(
            modifier = Modifier
                .size(34.dp)
                .clip(RoundedCornerShape(VynlRadius.control))
                .background(if (enabled) VynlColors.surfaceRaised else VynlColors.surface),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                icon,
                contentDescription = null,
                tint = if (enabled) VynlColors.dim else VynlColors.faint,
                modifier = Modifier.size(18.dp),
            )
        }
        Spacer(Modifier.width(14.dp))
        Text(
            text = title,
            color = if (enabled) VynlColors.text else VynlColors.faint,
            fontSize = 14.sp,
            fontWeight = FontWeight.W600,
            fontFamily = VynlFonts.display,
        )
    }
}
