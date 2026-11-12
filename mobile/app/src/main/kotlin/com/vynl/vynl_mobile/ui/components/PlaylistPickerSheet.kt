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
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.ModalBottomSheet
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.vynl.vynl_mobile.data.LocalPlaylist
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PlaylistPickerSheet(
    playlists: List<LocalPlaylist>,
    onPick: (LocalPlaylist) -> Unit,
    onCreate: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    var name by remember { mutableStateOf("") }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        containerColor = VynlColors.surface,
    ) {
        Column(modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 30.dp)) {
            SectionHeader("ADD TO PLAYLIST")
            playlists.forEach { playlist ->
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(VynlRadius.tile))
                        .clickable { onPick(playlist) }
                        .padding(horizontal = 8.dp, vertical = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Box(
                        modifier = Modifier
                            .size(40.dp)
                            .clip(RoundedCornerShape(VynlRadius.control))
                            .background(VynlColors.surfaceRaised),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            Icons.Rounded.Add,
                            contentDescription = null,
                            tint = VynlColors.faint,
                            modifier = Modifier.size(18.dp),
                        )
                    }
                    Spacer(Modifier.width(12.dp))
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = playlist.name,
                            color = VynlColors.text,
                            fontSize = 14.sp,
                            fontWeight = FontWeight.W700,
                            fontFamily = VynlFonts.display,
                        )
                        Text(
                            text = if (playlist.trackCount == 1) "1 track" else "${playlist.trackCount} tracks",
                            color = VynlColors.faint,
                            fontSize = 11.5.sp,
                            fontFamily = VynlFonts.display,
                        )
                    }
                }
            }
            Spacer(Modifier.height(10.dp))
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                placeholder = {
                    Text(
                        "New playlist",
                        color = VynlColors.faint,
                        fontSize = 14.sp,
                        fontFamily = VynlFonts.display,
                    )
                },
                textStyle = androidx.compose.ui.text.TextStyle(
                    fontSize = 14.sp,
                    color = VynlColors.text,
                    fontFamily = VynlFonts.display,
                ),
                shape = RoundedCornerShape(VynlRadius.control),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedContainerColor = VynlColors.bg,
                    unfocusedContainerColor = VynlColors.bg,
                    focusedBorderColor = VynlColors.line,
                    unfocusedBorderColor = VynlColors.line,
                    cursorColor = VynlColors.accent,
                    focusedTextColor = VynlColors.text,
                    unfocusedTextColor = VynlColors.text,
                ),
                trailingIcon = {
                    if (name.isNotBlank()) {
                        Box(
                            modifier = Modifier
                                .size(40.dp)
                                .clickable { onCreate(name) },
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(
                                Icons.Rounded.Add,
                                contentDescription = "Create the playlist",
                                tint = VynlColors.accent,
                                modifier = Modifier.size(20.dp),
                            )
                        }
                    }
                },
            )
        }
    }
}
