package com.vynl.vynl_mobile.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.vynl.vynl_mobile.data.CatalogueTrack
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts

data class LocalTrackAction(
    val icon: ImageVector,
    val title: String,
    val onClick: () -> Unit,
    val enabled: Boolean = true,
    val destructive: Boolean = false,
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LocalTrackActionsSheet(
    track: CatalogueTrack,
    actions: List<LocalTrackAction>,
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
            actions.forEach { action ->
                ActionRow(
                    icon = action.icon,
                    title = action.title,
                    enabled = action.enabled,
                    destructive = action.destructive,
                    onClick = {
                        action.onClick()
                        onDismiss()
                    },
                )
            }
        }
    }
}
