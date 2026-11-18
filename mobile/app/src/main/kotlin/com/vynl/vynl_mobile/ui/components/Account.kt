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

internal fun avatarSeed(email: String): String {
    val digest = java.security.MessageDigest.getInstance("SHA-256")
        .digest(email.trim().lowercase().toByteArray(Charsets.UTF_8))
    return digest.joinToString("") { b -> "%02x".format(b) }.take(24)
}

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
                val seed = if (signedIn && !email.isNullOrBlank()) avatarSeed(email) else displayName
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
    val session = viewModel.syncStatus
    val seed = if (session.signedIn && !session.email.isNullOrBlank()) {
        avatarSeed(session.email)
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

                if (session.signedIn) {
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
                text = if (signedIn) "Playlist changes sync automatically." else "Keep everything on this device.",
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
    val session = viewModel.syncStatus
    val seed = session.email?.let { avatarSeed(it) } ?: viewModel.displayName.ifBlank { viewModel.nickname }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        AccountBanner(seed = seed, signedIn = true, email = session.email)

        ActionRow(
            label = "Playlist sync",
            hint = "Audio files and cover art stay on this device.",
        ) {
            ActionButton(
                label = if (viewModel.syncBusy) "Syncing" else "Sync now",
                onClick = { viewModel.syncNow() },
                accent = true,
                busy = viewModel.syncBusy,
            )
        }

        ActionRow(
            label = "Sign out",
            hint = "Stop syncing playlists on this device.",
        ) {
            ActionButton(
                label = "Sign out",
                onClick = { viewModel.signOut(); onDismiss() },
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
                enabled = !viewModel.authBusy,
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
                enabled = !viewModel.authBusy,
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done, keyboardType = KeyboardType.Password),
                keyboardActions = KeyboardActions(onDone = { submitAuth(viewModel, signUp, email.trim(), password) }),
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(VynlRadius.control),
                colors = accountFieldColors(),
            )
        }

        if (viewModel.authError != null) {
            Text(
                text = viewModel.authError.orEmpty(),
                color = VynlColors.danger,
                fontSize = 11.sp,
                lineHeight = 16.sp,
                fontFamily = VynlFonts.display,
            )
        } else if (viewModel.authNotice != null) {
            Text(
                text = viewModel.authNotice.orEmpty(),
                color = VynlColors.success,
                fontSize = 11.sp,
                lineHeight = 16.sp,
                fontFamily = VynlFonts.display,
            )
        }

        ActionRow(
            label = if (signUp) "Create account" else "Sign in",
            hint = if (signUp) "Create an account to sync playlists from this device" else "Sync playlists already saved to your account",
        ) {
            ActionButton(
                label = if (viewModel.authBusy) (if (signUp) "Creating" else "Signing in") else (if (signUp) "Create account" else "Sign in"),
                onClick = { submitAuth(viewModel, signUp, email.trim(), password) },
                accent = true,
                enabled = canSubmit,
                busy = viewModel.authBusy,
            )
        }

        Text(
            text = if (signUp) "Already have an account? Sign in" else "New here? Create an account",
            color = VynlColors.dim,
            fontSize = 10.5.sp,
            fontFamily = VynlFonts.mono,
            modifier = Modifier.clickable(enabled = !viewModel.authBusy) { signUp = !signUp },
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

private fun submitAuth(viewModel: AppViewModel, signUp: Boolean, email: String, password: String) {
    if (signUp) {
        viewModel.signUp(email, password)
    } else {
        viewModel.signIn(email, password)
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
