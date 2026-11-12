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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.SystemUpdateAlt
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.components.AccentPicker
import com.vynl.vynl_mobile.ui.components.HueSlider
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@Composable
fun SettingsScreen(viewModel: AppViewModel, onClose: () -> Unit) {
    val update by viewModel.update.collectAsStateWithLifecycle()
    val version by viewModel.installedVersion.collectAsStateWithLifecycle()
    var address by remember { mutableStateOf(viewModel.desktop.address) }
    var pin by remember { mutableStateOf("") }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg),
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
                    contentDescription = "Close the settings",
                    tint = VynlColors.dim,
                    modifier = Modifier.size(26.dp),
                )
            }
            Text(
                text = "Settings",
                modifier = Modifier.weight(1f),
                color = VynlColors.text,
                fontSize = 21.sp,
                fontWeight = FontWeight.W600,
                letterSpacing = (-0.4).sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.width(48.dp))
        }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp),
        ) {
            SettingsGroup("APPEARANCE")
            Text(
                text = "Accent",
                color = VynlColors.dim,
                fontSize = 13.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(10.dp))
            AccentPicker(
                accent = VynlColors.accent,
                onPick = { viewModel.setAccent(it) },
            )
            Spacer(Modifier.height(12.dp))
            HueSlider(
                hue = hueOf(VynlColors.accent),
                onHue = { viewModel.setAccent(Color.hsv(it * 360f, 0.62f, 0.91f)) },
            )

            SettingsGroup("DESKTOP")
            Text(
                text = "Pair with the desktop to see what it plays and to bring its library over.",
                color = VynlColors.dim,
                fontSize = 12.5.sp,
                lineHeight = 19.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(12.dp))
            Field(
                value = address,
                onChange = { address = it },
                hint = "Address of the desktop",
            )
            Spacer(Modifier.height(10.dp))
            Field(
                value = pin,
                onChange = { pin = it },
                hint = "Pin from the desktop",
            )
            Spacer(Modifier.height(12.dp))
            ActionButton(
                title = if (viewModel.desktop.paired) "Pair again" else "Pair with the desktop",
                onClick = { viewModel.pairDesktop(address, pin) },
            )
            if (viewModel.desktop.paired) {
                Spacer(Modifier.height(10.dp))
                ActionButton(
                    title = "Bring the library over",
                    onClick = { viewModel.syncLibrary() },
                )
            }

            SettingsGroup("UPDATES")
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Version $version",
                        color = VynlColors.text,
                        fontSize = 13.5.sp,
                        fontWeight = FontWeight.W600,
                        fontFamily = VynlFonts.display,
                    )
                    Spacer(Modifier.height(2.dp))
                    Text(
                        text = update?.let { "Version ${it.short} is ready" } ?: "Check for a newer build",
                        color = VynlColors.faint,
                        fontSize = 12.sp,
                        fontFamily = VynlFonts.display,
                    )
                }
                Box(
                    modifier = Modifier
                        .size(40.dp)
                        .clip(RoundedCornerShape(VynlRadius.control))
                        .background(
                            if (viewModel.updateBusy) {
                                VynlColors.surfaceRaised
                            } else {
                                VynlColors.accent.copy(alpha = 0.16f)
                            },
                        )
                        .clickable(enabled = !viewModel.updateBusy) { viewModel.checkForUpdate() },
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        Icons.Rounded.SystemUpdateAlt,
                        contentDescription = "Check for updates",
                        tint = VynlColors.accent,
                        modifier = Modifier.size(20.dp),
                    )
                }
            }
            if (viewModel.updateBusy) {
                Spacer(Modifier.height(10.dp))
                LinearProgressIndicator(
                    progress = { viewModel.updateProgress },
                    modifier = Modifier.fillMaxWidth(),
                    color = VynlColors.accent,
                    trackColor = VynlColors.line,
                    drawStopIndicator = {},
                )
            }
            update?.let { ready ->
                if (ready.notes.isNotEmpty()) {
                    Spacer(Modifier.height(12.dp))
                    Text(
                        text = ready.notes,
                        color = VynlColors.dim,
                        fontSize = 12.5.sp,
                        lineHeight = 19.sp,
                        fontFamily = VynlFonts.display,
                    )
                }
                Spacer(Modifier.height(12.dp))
                ActionButton(
                    title = if (viewModel.updateDownloaded) "Install the update" else "Download the update",
                    onClick = {
                        if (viewModel.updateDownloaded) viewModel.installUpdate() else viewModel.downloadUpdate()
                    },
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
                Text(
                    text = "Clear",
                    color = VynlColors.accent,
                    fontSize = 12.5.sp,
                    fontWeight = FontWeight.W700,
                    fontFamily = VynlFonts.display,
                    modifier = Modifier
                        .clip(RoundedCornerShape(VynlRadius.control))
                        .clickable { viewModel.clearDownloads() }
                        .padding(horizontal = 12.dp, vertical = 8.dp),
                )
            }

            SettingsGroup("ABOUT")
            Text(
                text = "Vynl, a minimal music player.",
                color = VynlColors.dim,
                fontSize = 13.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = "github.com/DevX32/Vynl",
                color = VynlColors.faint,
                fontSize = 12.sp,
                fontFamily = VynlFonts.display,
            )
            Spacer(Modifier.height(40.dp))
        }
    }
}

@Composable
private fun Field(
    value: String,
    onChange: (String) -> Unit,
    hint: String,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        modifier = Modifier.fillMaxWidth(),
        singleLine = true,
        placeholder = {
            Text(
                hint,
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
    )
}

@Composable
private fun SettingsGroup(title: String) {
    Spacer(Modifier.height(26.dp))
    Text(
        text = title,
        color = VynlColors.faint,
        fontSize = 11.sp,
        fontWeight = FontWeight.W800,
        letterSpacing = 1.3.sp,
        fontFamily = VynlFonts.display,
    )
    Spacer(Modifier.height(12.dp))
}

@Composable
private fun ActionButton(title: String, onClick: () -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(46.dp)
            .clip(RoundedCornerShape(VynlRadius.control))
            .background(VynlColors.accent)
            .clickable { onClick() },
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = title,
            color = VynlColors.accentOn,
            fontSize = 14.sp,
            fontWeight = FontWeight.W700,
            fontFamily = VynlFonts.display,
        )
    }
}

private fun hueOf(color: Color): Float {
    val red = color.red
    val green = color.green
    val blue = color.blue
    val max = maxOf(red, green, blue)
    val min = minOf(red, green, blue)
    val delta = max - min
    if (delta <= 0f) return 0f
    val hue = when (max) {
        red -> ((green - blue) / delta) % 6f
        green -> (blue - red) / delta + 2f
        else -> (red - green) / delta + 4f
    }
    return ((hue * 60f) + 360f) % 360f / 360f
}
