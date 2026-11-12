package com.vynl.vynl_mobile.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.vynl.vynl_mobile.R

object VynlColors {
    val bg = Color(0xFF0E0E10)
    val bgTop = Color(0xFF151519)
    val surface = Color(0xFF131317)
    val surfaceRaised = Color(0xFF151519)
    val line = Color(0x17F0F0EC)
    val lineStrong = Color(0x33F0F0EC)
    val text = Color(0xFFF0F0EC)
    val dim = Color(0xFFA3A39B)
    val faint = Color(0xFF7A7A74)
    val warm = Color(0xFFD9A15C)
    val danger = Color(0xFFFF6B61)
    val warning = Color(0xFFD9A15C)
    val success = Color(0xFF8FD694)

    val accent = Color(0xFFA894E8)
    val accentOn = Color(0xFF161128)
    val accentSoft = Color(0x1CB9A7FF)
}

object VynlRadius {
    val control = 4.dp
    val card = 4.dp
    val tile = 4.dp
    val thumb = 4.dp
    val art = 4.dp
    val hero = 10.dp
}

object VynlMotion {
    const val fast = 170
    const val normal = 320
    const val slow = 520
}

object VynlFonts {
    private fun variable(resId: Int, weight: Int): Font = Font(
        resId = resId,
        weight = FontWeight(weight),
        variationSettings = FontVariation.Settings(FontVariation.weight(weight)),
    )

    val display = FontFamily(
        variable(R.font.geist_variable, 400),
        variable(R.font.geist_variable, 500),
        variable(R.font.geist_variable, 600),
        variable(R.font.geist_variable, 700),
    )

    val mono = FontFamily(
        variable(R.font.geistmono_variable, 400),
        variable(R.font.geistmono_variable, 500),
        variable(R.font.geistmono_variable, 600),
        variable(R.font.geistmono_variable, 700),
    )

    val lyrics = FontFamily(
        variable(R.font.sora_variable, 300),
        variable(R.font.sora_variable, 400),
        variable(R.font.sora_variable, 600),
    )

    val serif = FontFamily(
        variable(R.font.fraunces_variable, 400),
        variable(R.font.fraunces_variable, 600),
    )
}

private val VynlColorScheme = darkColorScheme(
    primary = VynlColors.accent,
    onPrimary = VynlColors.accentOn,
    secondary = VynlColors.warm,
    onSecondary = Color(0xFF161128),
    background = VynlColors.bg,
    onBackground = VynlColors.text,
    surface = VynlColors.bg,
    onSurface = VynlColors.text,
    surfaceVariant = VynlColors.surfaceRaised,
    onSurfaceVariant = VynlColors.dim,
    surfaceContainer = VynlColors.surface,
    surfaceContainerHigh = VynlColors.surfaceRaised,
    outline = VynlColors.line,
    outlineVariant = VynlColors.line,
    error = VynlColors.danger,
)

@Composable
fun VynlTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = VynlColorScheme,
        content = content,
    )
}
