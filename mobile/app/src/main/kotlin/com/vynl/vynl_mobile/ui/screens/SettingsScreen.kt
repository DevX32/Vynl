package com.vynl.vynl_mobile.ui.screens

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.DeleteOutline
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import com.vynl.vynl_mobile.R
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.material.icons.rounded.Coffee
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.AccentSheet
import com.vynl.vynl_mobile.ui.components.AccentSwatch
import com.vynl.vynl_mobile.ui.components.hexOf
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@Composable
fun SettingsScreen(viewModel: AppViewModel, onClose: () -> Unit) {
    val update by viewModel.update.collectAsStateWithLifecycle()
    val version by viewModel.installedVersion.collectAsStateWithLifecycle()
    var accentOpen by remember { mutableStateOf(false) }
    var pairOpen by remember { mutableStateOf(false) }
    var clearConfirm by remember { mutableStateOf(false) }
    var unpairConfirm by remember { mutableStateOf(false) }
    val paired = viewModel.desktop.paired
    val context = LocalContext.current

    fun openKofi() {
        runCatching {
            context.startActivity(
                Intent(Intent.ACTION_VIEW, Uri.parse("https://ko-fi.com/devx32"))
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
        }
    }

    BackHandler(enabled = !pairOpen && !accentOpen && !clearConfirm && !unpairConfirm) { onClose() }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg)
            .statusBarsPadding(),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(top = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "Settings",
                modifier = Modifier
                    .weight(1f)
                    .padding(start = 20.dp),
                color = VynlColors.text,
                fontSize = 21.sp,
                fontWeight = FontWeight.W600,
                letterSpacing = (-0.4).sp,
                fontFamily = VynlFonts.display,
            )
            IconButton(onClick = onClose) {
                Icon(
                    Icons.Rounded.KeyboardArrowDown,
                    contentDescription = "Close the settings",
                    tint = VynlColors.dim,
                    modifier = Modifier.size(26.dp),
                )
            }
        }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .navigationBarsPadding()
                .padding(horizontal = 20.dp),
        ) {
            SettingsGroup("PROFILE")
            NicknameField(
                value = viewModel.displayName,
                nickname = viewModel.nickname,
                onChange = viewModel::saveDisplayName,
                onRoll = viewModel::rollNickname,
            )

            SettingsGroup("APPEARANCE")
            SettingsRow(
                label = "Accent colour",
                hint = if (VynlColors.presets.any { it == VynlColors.accent }) {
                    "Preset"
                } else {
                    "Custom #${hexOf(VynlColors.accent)}"
                },
                onClick = { accentOpen = true },
                trailing = {
                    AccentSwatch(
                        color = VynlColors.accent,
                        selected = false,
                        onClick = { accentOpen = true },
                        size = 26.dp,
                    )
                },
            )

            SettingsGroup("DESKTOP")
            SettingsRow(
                label = "Connection",
                hint = if (paired) "Synced over your local network" else "Not paired yet",
                trailing = { StatusChip(text = if (paired) "PAIRED" else "OFF", on = paired) },
            )
            if (paired) {
                SettingsRow(
                    label = "Sync library",
                    hint = if (viewModel.syncing) {
                        viewModel.syncStage
                    } else {
                        "Sync songs, covers, lyrics and playlists"
                    },
                    onClick = { viewModel.syncLibrary() },
                    trailing = {
                        if (viewModel.syncing) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(17.dp),
                                color = VynlColors.accent,
                                strokeWidth = 2.dp,
                            )
                        } else {
                            Chip("Sync")
                        }
                    },
                )
                if (viewModel.syncing) {
                    LinearProgressIndicator(
                        progress = { viewModel.syncProgressState },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(3.dp)
                            .clip(RoundedCornerShape(2.dp)),
                        color = VynlColors.accent,
                        trackColor = VynlColors.surfaceRaised,
                    )
                }
                SettingsRow(
                    label = "Unpair",
                    hint = "Forget this desktop and its library",
                    labelColor = VynlColors.danger,
                    onClick = { unpairConfirm = true },
                    trailing = { Chip("Unpair", danger = true) },
                )
            } else {
                SettingsRow(
                    label = "Pair with desktop",
                    hint = "Scan the QR code shown in Vynl GǦ Settings",
                    onClick = { pairOpen = true },
                    trailing = { Chip("Scan", accent = true) },
                )
            }

            SettingsGroup("STORAGE")
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Downloads",
                        color = VynlColors.text,
                        fontSize = 13.5.sp,
                        fontWeight = FontWeight.W600,
                        fontFamily = VynlFonts.display,
                    )
                    Spacer(Modifier.height(2.dp))
                    Text(
                        text = "${viewModel.downloadedSize.value} on the phone",
                        color = VynlColors.faint,
                        fontSize = 12.sp,
                        fontFamily = VynlFonts.display,
                    )
                }
                Row(
                    modifier = Modifier
                        .clip(RoundedCornerShape(VynlRadius.control))
                        .background(VynlColors.danger.copy(alpha = 0.09f))
                        .border(1.dp, VynlColors.danger.copy(alpha = 0.38f), RoundedCornerShape(VynlRadius.control))
                        .clickable { clearConfirm = true }
                        .padding(horizontal = 14.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        Icons.Rounded.DeleteOutline,
                        contentDescription = null,
                        tint = VynlColors.danger,
                        modifier = Modifier.size(16.dp),
                    )
                    Spacer(Modifier.width(7.dp))
                    Text(
                        text = "Clear",
                        color = VynlColors.danger,
                        fontSize = 12.5.sp,
                        fontWeight = FontWeight.W700,
                        fontFamily = VynlFonts.display,
                    )
                }
            }

            SettingsGroup("ABOUT")
            Row(verticalAlignment = Alignment.CenterVertically) {
                Image(
                    painter = painterResource(R.mipmap.ic_launcher_foreground),
                    contentDescription = null,
                    modifier = Modifier.size(76.dp).clip(RoundedCornerShape(18.dp)),
                )
                Spacer(Modifier.width(14.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Vynl",
                        color = VynlColors.text,
                        fontSize = 18.sp,
                        fontWeight = FontWeight.W800,
                        letterSpacing = (-0.3).sp,
                        fontFamily = VynlFonts.display,
                    )
                    Text(
                        text = "Version $version",
                        color = VynlColors.faint,
                        fontSize = 12.sp,
                        lineHeight = 14.sp,
                        fontFamily = VynlFonts.mono,
                    )
                    Text(
                        text = "A minimal music player",
                        color = VynlColors.dim,
                        fontSize = 13.sp,
                        lineHeight = 16.sp,
                        fontFamily = VynlFonts.display,
                    )
                }
            }
            Spacer(Modifier.height(8.dp))
            SettingsRow(
                label = when {
                    update == null -> "Check for updates"
                    viewModel.updateDownloaded -> "Install update ${update?.short}"
                    else -> "Update ${update?.short} is ready"
                },
                hint = when {
                    update == null -> "Builds ship whenever the desktop ships"
                    viewModel.updateDownloaded -> "Ready to install"
                    else -> "A new build is ready to download"
                },
                onClick = {
                    when {
                        update == null -> viewModel.checkForUpdate()
                        viewModel.updateDownloaded -> viewModel.installUpdate()
                        else -> viewModel.downloadUpdate()
                    }
                },
                trailing = if (viewModel.updateBusy) {
                    {
                        CircularProgressIndicator(
                            modifier = Modifier.size(17.dp),
                            color = VynlColors.accent,
                            strokeWidth = 2.dp,
                        )
                    }
                } else {
                    {
                        Chip(
                            label = when {
                                update == null -> "Check"
                                viewModel.updateDownloaded -> "Install"
                                else -> "Download"
                            },
                            accent = true,
                        )
                    }
                },
            )
            SettingsRow(
                label = "Support Vynl",
                hint = "If you like the app, a coffee helps keep it going.",
                trailing = {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .clip(RoundedCornerShape(VynlRadius.control))
                            .background(VynlColors.accent.copy(alpha = 0.16f))
                            .clickable { openKofi() }
                            .padding(horizontal = 14.dp, vertical = 8.dp),
                    ) {
                        Icon(
                            Icons.Rounded.Coffee,
                            contentDescription = "Support on Ko-fi",
                            tint = VynlColors.accent,
                            modifier = Modifier.size(16.dp),
                        )
                        Spacer(Modifier.width(6.dp))
                        Text(
                            text = "Ko-fi",
                            color = VynlColors.accent,
                            fontSize = 12.sp,
                            fontWeight = FontWeight.W700,
                            fontFamily = VynlFonts.display,
                        )
                    }
                },
            )
            Spacer(Modifier.height(40.dp))
        }
    }

    if (accentOpen) {
        AccentSheet(
            accent = VynlColors.accent,
            onChanged = viewModel::setAccent,
            onDismiss = { accentOpen = false },
        )
    }

    if (clearConfirm) {
        AlertDialog(
            onDismissRequest = { clearConfirm = false },
            shape = RoundedCornerShape(VynlRadius.control),
            title = {
                Text(
                    text = "Remove all downloaded songs?",
                    color = VynlColors.text,
                    fontFamily = VynlFonts.display,
                    fontWeight = FontWeight.W800,
                )
            },
            text = {
                Text(
                    text = "Everything you have downloaded will be freed (${viewModel.downloadedSize.value} on the phone). The songs stay playable by streaming from the desktop.",
                    color = VynlColors.dim,
                    fontFamily = VynlFonts.display,
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        clearConfirm = false
                        viewModel.clearDownloads()
                    },
                    shape = RoundedCornerShape(VynlRadius.control),
                    colors = ButtonDefaults.buttonColors(containerColor = VynlColors.danger),
                ) {
                    Text(
                        text = "Remove all",
                        color = Color.White,
                        fontWeight = FontWeight.W700,
                        fontFamily = VynlFonts.display,
                    )
                }
            },
            dismissButton = {
                TextButton(
                    onClick = { clearConfirm = false },
                    shape = RoundedCornerShape(VynlRadius.control),
                ) {
                    Text(
                        text = "Cancel",
                        color = VynlColors.dim,
                        fontFamily = VynlFonts.display,
                    )
                }
            },
            containerColor = VynlColors.surface,
        )
    }
    if (unpairConfirm) {
        AlertDialog(
            onDismissRequest = { unpairConfirm = false },
            shape = RoundedCornerShape(VynlRadius.control),
            title = {
                Text(
                    text = "Forget this desktop?",
                    color = VynlColors.text,
                    fontFamily = VynlFonts.display,
                    fontWeight = FontWeight.W800,
                )
            },
            text = {
                Text(
                    text = "The desktop will be forgotten — no more sync. The library you synced stays on the phone.",
                    color = VynlColors.dim,
                    fontFamily = VynlFonts.display,
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        unpairConfirm = false
                        viewModel.unpairDesktop()
                    },
                    shape = RoundedCornerShape(VynlRadius.control),
                    colors = ButtonDefaults.buttonColors(containerColor = VynlColors.danger),
                ) {
                    Text(
                        text = "Forget",
                        color = Color.White,
                        fontWeight = FontWeight.W700,
                        fontFamily = VynlFonts.display,
                    )
                }
            },
            dismissButton = {
                TextButton(
                    onClick = { unpairConfirm = false },
                    shape = RoundedCornerShape(VynlRadius.control),
                ) {
                    Text(
                        text = "Cancel",
                        color = VynlColors.dim,
                        fontFamily = VynlFonts.display,
                    )
                }
            },
            containerColor = VynlColors.surface,
        )
    }

    if (pairOpen) {
        PairScreen(
            viewModel = viewModel,
            onClose = { pairOpen = false },
            onPaired = {
                pairOpen = false
                viewModel.selectTab(1)
                onClose()
            },
        )
    }
}

@Composable
private fun NicknameField(
    value: String,
    nickname: String,
    onChange: (String) -> Unit,
    onRoll: () -> Unit,
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = "NICKNAME",
            color = VynlColors.faint,
            fontSize = 11.sp,
            fontWeight = FontWeight.W800,
            letterSpacing = 1.3.sp,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(10.dp))
        OutlinedTextField(
            value = value,
            onValueChange = { onChange(it.take(32)) },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            placeholder = {
                Text(
                    nickname,
                    color = VynlColors.faint,
                    fontSize = 14.sp,
                    fontFamily = VynlFonts.display,
                )
            },
            textStyle = TextStyle(
                fontSize = 14.sp,
                color = VynlColors.text,
                fontFamily = VynlFonts.display,
            ),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = {}),
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
        )
        Spacer(Modifier.height(8.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = "Your name across Vynl. Leave blank for a random nickname.",
                modifier = Modifier.weight(1f),
                color = VynlColors.faint,
                fontSize = 12.sp,
                lineHeight = 17.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.width(12.dp))
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(VynlRadius.control))
                    .clickable { onRoll() }
                    .padding(horizontal = 6.dp, vertical = 4.dp),
            ) {
                Chip("Roll")
            }
        }
    }
}

@Composable
private fun SettingsRow(
    label: String,
    hint: String,
    onClick: (() -> Unit)? = null,
    labelColor: Color = VynlColors.text,
    trailing: @Composable (() -> Unit)? = null,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable { onClick() } else Modifier)
            .padding(vertical = 13.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = label,
                color = labelColor,
                fontSize = 13.5.sp,
                fontWeight = FontWeight.W600,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(2.dp))
            Text(
                text = hint,
                color = VynlColors.faint,
                fontSize = 12.sp,
                lineHeight = 17.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                fontFamily = VynlFonts.display,
            )
        }
        if (trailing != null) {
            Spacer(Modifier.width(12.dp))
            trailing()
        }
    }
}

@Composable
private fun Chip(
    label: String,
    accent: Boolean = false,
    danger: Boolean = false,
) {
    val foreground = when {
        danger -> VynlColors.danger
        accent -> VynlColors.accent
        else -> VynlColors.dim
    }
    val border = when {
        danger -> VynlColors.danger.copy(alpha = 0.5f)
        accent -> VynlColors.accent.copy(alpha = 0.5f)
        else -> VynlColors.line
    }
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(VynlRadius.control))
            .background(
                when {
                    danger -> VynlColors.danger.copy(alpha = 0.08f)
                    accent -> VynlColors.accent.copy(alpha = 0.10f)
                    else -> Color.Transparent
                },
            )
            .border(1.dp, border, RoundedCornerShape(VynlRadius.control))
            .padding(horizontal = 10.dp, vertical = 6.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            color = foreground,
            fontSize = 11.5.sp,
            fontWeight = FontWeight.W700,
            fontFamily = VynlFonts.display,
        )
    }
}

@Composable
private fun StatusChip(text: String, on: Boolean) {
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(VynlRadius.control))
            .background(if (on) VynlColors.accent.copy(alpha = 0.12f) else VynlColors.text.copy(alpha = 0.06f))
            .padding(horizontal = 9.dp, vertical = 5.dp),
    ) {
        Text(
            text = text,
            color = if (on) VynlColors.accent else VynlColors.faint,
            fontSize = 10.sp,
            fontWeight = FontWeight.W800,
            letterSpacing = 0.8.sp,
            fontFamily = VynlFonts.display,
        )
    }
}

@Composable
private fun SettingsGroup(title: String) {
    Spacer(Modifier.height(24.dp))
    Text(
        text = title,
        color = VynlColors.faint,
        fontSize = 11.sp,
        fontWeight = FontWeight.W800,
        letterSpacing = 1.3.sp,
        fontFamily = VynlFonts.display,
    )
    Spacer(Modifier.height(6.dp))
}

