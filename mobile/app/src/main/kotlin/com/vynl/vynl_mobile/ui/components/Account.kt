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
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.automirrored.rounded.Login
import androidx.compose.material.icons.automirrored.rounded.Logout
import androidx.compose.material.icons.rounded.Refresh
import androidx.compose.material.icons.rounded.Tune
import androidx.compose.material.icons.rounded.Person
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.ui.window.Dialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius
import java.net.URLEncoder

internal fun blobatarUrl(seed: String, size: Int): String {
    val safe = URLEncoder.encode(seed.trim().ifBlank { "vynl" }, "UTF-8")
        .replace("+", "%20")
    return "https://blobatar.dev/avatar/$safe?size=$size&background=none&gen=2"
}

@Composable
internal fun AccountAvatar(
    seed: String,
    size: Dp,
    modifier: Modifier = Modifier,
) {
    Box(modifier = modifier.size(size)) {
        AsyncImage(
            model = blobatarUrl(seed, size = (size.value * 4).toInt().coerceAtLeast(96)),
            contentDescription = "Account avatar",
            modifier = Modifier
                .fillMaxSize()
                .clip(CircleShape),
        )
    }
}

@Composable
internal fun AccountMenu(
    expanded: Boolean,
    onDismiss: () -> Unit,
    displayName: String,
    email: String?,
    signedIn: Boolean,
    onAccount: () -> Unit,
    onSync: () -> Unit,
    onSettings: () -> Unit,
    onSignOut: () -> Unit,
) {
    DropdownMenu(
        expanded = expanded,
        onDismissRequest = onDismiss,
        containerColor = VynlColors.surface,
        shape = RoundedCornerShape(VynlRadius.control),
        tonalElevation = 0.dp,
        shadowElevation = 16.dp,
    ) {
        Column(
            modifier = Modifier
                .widthIn(min = 220.dp, max = 240.dp)
                .padding(vertical = 4.dp),
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
            ) {
                val seed = if (signedIn && !email.isNullOrBlank()) email else displayName
                AccountAvatar(seed = seed, size = 34.dp)
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = displayName,
                        color = VynlColors.text,
                        fontSize = 14.sp,
                        fontWeight = androidx.compose.ui.text.font.FontWeight.W600,
                        fontFamily = VynlFonts.display,
                        maxLines = 1,
                    )
                    Text(
                        text = if (signedIn) email.orEmpty() else "Not signed in",
                        color = VynlColors.faint,
                        fontSize = 10.sp,
                        fontFamily = VynlFonts.mono,
                        maxLines = 1,
                    )
                }
            }
            HorizontalDivider(color = VynlColors.line)
            if (signedIn) {
                AccountMenuItem(
                    label = "Account",
                    icon = Icons.Rounded.Person,
                    onClick = { onDismiss(); onAccount() },
                )
                AccountMenuItem(
                    label = "Sync now",
                    icon = Icons.Rounded.Refresh,
                    onClick = { onDismiss(); onSync() },
                )
            } else {
                AccountMenuItem(
                    label = "Sign in",
                    icon = Icons.AutoMirrored.Rounded.Login,
                    accent = true,
                    onClick = { onDismiss(); onAccount() },
                )
            }
            AccountMenuItem(
                label = "Settings",
                icon = Icons.Rounded.Tune,
                onClick = { onDismiss(); onSettings() },
            )
            if (signedIn) {
                AccountMenuItem(
                    label = "Sign out",
                    icon = Icons.AutoMirrored.Rounded.Logout,
                    danger = true,
                    onClick = { onDismiss(); onSignOut() },
                )
            }
        }
    }
}

@Composable
private fun AccountMenuItem(
    label: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    onClick: () -> Unit,
    accent: Boolean = false,
    danger: Boolean = false,
) {
    val color = when {
        danger -> VynlColors.danger
        accent -> VynlColors.accent
        else -> VynlColors.dim
    }
    DropdownMenuItem(
        modifier = Modifier.height(44.dp),
        text = {
            Text(
                text = label,
                color = color,
                fontSize = 12.5.sp,
                fontWeight = androidx.compose.ui.text.font.FontWeight.W500,
                fontFamily = VynlFonts.display,
            )
        },
        leadingIcon = { Icon(icon, contentDescription = null, tint = color, modifier = Modifier.size(16.dp)) },
        onClick = onClick,
        colors = androidx.compose.material3.MenuDefaults.itemColors(
            textColor = color,
            leadingIconColor = color,
        ),
    )
}

@Composable
internal fun AccountDialog(
    viewModel: AppViewModel,
    onDismiss: () -> Unit,
) {
    val cloud = viewModel.cloudStatus
    val seed = if (cloud.signedIn && !cloud.email.isNullOrBlank()) {
        cloud.email
    } else {
        viewModel.displayName.ifBlank { viewModel.nickname.ifBlank { "vynl" } }
    }

    Dialog(onDismissRequest = onDismiss) {
        Surface(
            modifier = Modifier
                .fillMaxWidth(0.92f)
                .padding(vertical = 24.dp),
            shape = RoundedCornerShape(VynlRadius.control),
            color = VynlColors.surface,
            border = androidx.compose.foundation.BorderStroke(1.dp, VynlColors.line),
        ) {
            Column(
                modifier = Modifier
                    .padding(16.dp)
                    .verticalScroll(rememberScrollState()),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = "VYNL",
                        modifier = Modifier.weight(1f),
                        color = VynlColors.faint,
                        fontSize = 9.5.sp,
                        fontFamily = VynlFonts.mono,
                        letterSpacing = 1.8.sp,
                    )
                    Box(
                        modifier = Modifier.size(36.dp),
                        contentAlignment = Alignment.Center,
                    ) {
                        Box(
                            modifier = Modifier
                                .size(24.dp)
                                .clip(RoundedCornerShape(VynlRadius.control))
                                .background(VynlColors.surfaceRaised)
                                .border(1.dp, VynlColors.line, RoundedCornerShape(VynlRadius.control))
                                .clickable { onDismiss() }
                                .rotate(45f),
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(
                                Icons.Rounded.Close,
                                contentDescription = "Close the account dialog",
                                tint = VynlColors.faint,
                                modifier = Modifier
                                    .rotate(-45f)
                                    .size(12.dp),
                            )
                        }
                    }
                }

                Spacer(Modifier.height(18.dp))

                if (cloud.signedIn) {
                    SignedInBody(viewModel, onDismiss)
                } else {
                    SignedOutBody(viewModel, onDismiss)
                }
            }
        }
    }
}

@Composable
private fun AccountBanner(seed: String, signedIn: Boolean, email: String?) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(VynlRadius.control))
            .background(VynlColors.bg)
            .padding(horizontal = 14.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        AccountAvatar(seed = seed, size = 46.dp)
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = "ACCOUNT",
                color = VynlColors.faint,
                fontSize = 9.5.sp,
                fontFamily = VynlFonts.mono,
                letterSpacing = 1.6.sp,
            )
            Text(
                text = if (signedIn) email.orEmpty() else "Not signed in",
                color = VynlColors.text,
                fontSize = 14.sp,
                fontWeight = androidx.compose.ui.text.font.FontWeight.W600,
                fontFamily = VynlFonts.display,
                maxLines = 1,
            )
            Text(
                text = if (signedIn) "Playlist changes mirror automatically." else "Keep everything on this device.",
                color = VynlColors.faint,
                fontSize = 10.5.sp,
                lineHeight = 15.sp,
                fontFamily = VynlFonts.display,
            )
        }
    }
}

@Composable
private fun ActionRow(
    label: String,
    hint: String,
    control: @Composable () -> Unit,
) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(VynlRadius.control))
            .border(1.dp, VynlColors.line, RoundedCornerShape(VynlRadius.control))
            .padding(horizontal = 12.dp, vertical = 10.dp),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = label,
                color = VynlColors.text,
                fontSize = 12.5.sp,
                fontWeight = androidx.compose.ui.text.font.FontWeight.W500,
                fontFamily = VynlFonts.display,
            )
            Text(
                text = hint,
                color = VynlColors.faint,
                fontSize = 10.5.sp,
                lineHeight = 14.sp,
                fontFamily = VynlFonts.display,
            )
        }
        control()
    }
}

@Composable
private fun ActionButton(
    label: String,
    onClick: () -> Unit,
    accent: Boolean = false,
    danger: Boolean = false,
    enabled: Boolean = true,
    busy: Boolean = false,
) {
    val color = when {
        danger -> VynlColors.danger
        accent -> VynlColors.accent
        else -> VynlColors.dim
    }
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(7.dp),
        modifier = Modifier
            .clip(RoundedCornerShape(VynlRadius.control))
            .border(1.dp, color.copy(alpha = if (enabled || busy) 0.55f else 0.2f), RoundedCornerShape(VynlRadius.control))
            .background(color.copy(alpha = if (enabled || busy) 0.12f else 0.05f))
            .clickable(enabled = enabled && !busy, onClick = onClick)
            .padding(horizontal = 13.dp, vertical = 8.dp),
    ) {
        if (busy) {
            CircularProgressIndicator(modifier = Modifier.size(12.dp), color = color, strokeWidth = 1.6.dp)
        }
        Text(
            text = label,
            color = color,
            fontSize = 12.5.sp,
            fontWeight = androidx.compose.ui.text.font.FontWeight.W600,
            fontFamily = VynlFonts.display,
        )
    }
}

@Composable
private fun SignedInBody(viewModel: AppViewModel, onDismiss: () -> Unit) {
    val cloud = viewModel.cloudStatus
    val seed = cloud.email ?: viewModel.displayName.ifBlank { viewModel.nickname }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        AccountBanner(seed = seed, signedIn = true, email = cloud.email)

        ActionRow(
            label = "Playlist mirror",
            hint = "Audio files and cover art stay on this device.",
        ) {
            ActionButton(
                label = if (viewModel.cloudSyncing) "Syncing" else "Sync now",
                onClick = { viewModel.syncCloud() },
                accent = true,
                busy = viewModel.cloudSyncing,
            )
        }

        ActionRow(
            label = "Sign out",
            hint = "Stop mirroring playlists on this device.",
        ) {
            ActionButton(
                label = "Sign out",
                onClick = { viewModel.cloudSignOut(); onDismiss() },
                danger = true,
            )
        }

        Text(
            text = "Stay signed out and Vynl works exactly as it does today - everything stays on this device.",
            color = VynlColors.faint,
            fontSize = 9.sp,
            lineHeight = 13.sp,
            fontFamily = VynlFonts.mono,
        )
    }
}

@Composable
private fun SignedOutBody(viewModel: AppViewModel, onDismiss: () -> Unit) {
    var signUp by remember { mutableStateOf(false) }
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    val canSubmit = email.isNotBlank() && password.isNotEmpty()
    val seed = viewModel.displayName.ifBlank { viewModel.nickname }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        AccountBanner(seed = seed, signedIn = false, email = null)

        Column {
            Text(
                text = "EMAIL",
                color = VynlColors.faint,
                fontSize = 9.5.sp,
                letterSpacing = 1.4.sp,
                fontFamily = VynlFonts.mono,
                modifier = Modifier.padding(start = 2.dp, bottom = 6.dp),
            )
            OutlinedTextField(
                value = email,
                onValueChange = { email = it },
                placeholder = { Text("you@example.com") },
                singleLine = true,
                enabled = !viewModel.cloudBusy,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next, keyboardType = KeyboardType.Email),
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(VynlRadius.control),
                colors = accountFieldColors(),
            )
        }

        Column {
            Text(
                text = "PASSWORD",
                color = VynlColors.faint,
                fontSize = 9.5.sp,
                letterSpacing = 1.4.sp,
                fontFamily = VynlFonts.mono,
                modifier = Modifier.padding(start = 2.dp, bottom = 6.dp),
            )
            OutlinedTextField(
                value = password,
                onValueChange = { password = it },
                placeholder = { Text("Your password") },
                singleLine = true,
                enabled = !viewModel.cloudBusy,
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done, keyboardType = KeyboardType.Password),
                keyboardActions = KeyboardActions(onDone = { submitCloud(viewModel, signUp, email.trim(), password) }),
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(VynlRadius.control),
                colors = accountFieldColors(),
            )
        }

        if (viewModel.cloudError != null) {
            Text(
                text = viewModel.cloudError.orEmpty(),
                color = VynlColors.danger,
                fontSize = 11.sp,
                lineHeight = 16.sp,
                fontFamily = VynlFonts.display,
            )
        } else if (viewModel.cloudNotice != null) {
            Text(
                text = viewModel.cloudNotice.orEmpty(),
                color = VynlColors.success,
                fontSize = 11.sp,
                lineHeight = 16.sp,
                fontFamily = VynlFonts.display,
            )
        }

        ActionRow(
            label = if (signUp) "Create account" else "Sign in",
            hint = if (signUp) "Create a playlist mirror for this device" else "Mirror playlists already saved to your account",
        ) {
            ActionButton(
                label = if (viewModel.cloudBusy) (if (signUp) "Creating" else "Signing in") else (if (signUp) "Create account" else "Sign in"),
                onClick = { submitCloud(viewModel, signUp, email.trim(), password) },
                accent = true,
                enabled = canSubmit,
                busy = viewModel.cloudBusy,
            )
        }

        Text(
            text = if (signUp) "Already have an account? Sign in" else "New here? Create an account",
            color = VynlColors.dim,
            fontSize = 10.5.sp,
            fontFamily = VynlFonts.mono,
            modifier = Modifier.clickable(enabled = !viewModel.cloudBusy) { signUp = !signUp },
        )

        Text(
            text = "Stay signed out and Vynl works exactly as it does today - everything stays on this device.",
            color = VynlColors.faint,
            fontSize = 9.sp,
            lineHeight = 13.sp,
            fontFamily = VynlFonts.mono,
        )
    }
}

private fun submitCloud(viewModel: AppViewModel, signUp: Boolean, email: String, password: String) {
    if (signUp) {
        viewModel.cloudSignUp(email, password)
    } else {
        viewModel.cloudSignIn(email, password)
    }
}

@Composable
private fun accountFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedContainerColor = VynlColors.bg,
    unfocusedContainerColor = VynlColors.bg,
    disabledContainerColor = VynlColors.bg,
    focusedBorderColor = VynlColors.line,
    unfocusedBorderColor = VynlColors.line,
    disabledBorderColor = VynlColors.line,
    cursorColor = VynlColors.accent,
    focusedTextColor = VynlColors.text,
    unfocusedTextColor = VynlColors.text,
    disabledTextColor = VynlColors.dim,
)
