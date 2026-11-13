package com.vynl.vynl_mobile.ui.screens

import androidx.activity.compose.BackHandler
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
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.text.TextStyle
import androidx.compose.foundation.border
import com.vynl.vynl_mobile.data.randomNickname
import com.vynl.vynl_mobile.ui.AppViewModel
import com.vynl.vynl_mobile.ui.theme.VynlColors
import com.vynl.vynl_mobile.ui.theme.VynlFonts
import com.vynl.vynl_mobile.ui.theme.VynlRadius

@Composable
fun OnboardingScreen(viewModel: AppViewModel) {
    var name by remember { mutableStateOf("") }
    val placeholder = remember { viewModel.nickname }

    BackHandler { }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(VynlColors.bg)
            .statusBarsPadding()
            .navigationBarsPadding()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 28.dp),
    ) {
        Spacer(Modifier.height(48.dp))
        Text(
            text = "GET STARTED",
            color = VynlColors.faint,
            fontSize = 11.sp,
            fontWeight = FontWeight.W800,
            letterSpacing = 1.8.sp,
            fontFamily = VynlFonts.display,
        )

        Spacer(Modifier.height(26.dp))
        Text(
            text = "What should\nwe call you?",
            color = VynlColors.text,
            fontSize = 31.sp,
            lineHeight = 37.sp,
            fontWeight = FontWeight.W700,
            letterSpacing = (-0.8).sp,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(12.dp))
        Text(
            text = "Your name across Vynl.",
            color = VynlColors.dim,
            fontSize = 14.sp,
            lineHeight = 21.sp,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(30.dp))

        Text(
            text = "YOUR NAME",
            color = VynlColors.faint,
            fontSize = 11.sp,
            fontWeight = FontWeight.W800,
            letterSpacing = 1.3.sp,
            fontFamily = VynlFonts.display,
        )
        Spacer(Modifier.height(10.dp))
        OutlinedTextField(
            value = name,
            onValueChange = { name = it.take(32) },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            placeholder = {
                Text(
                    placeholder,
                    color = VynlColors.faint,
                    fontSize = 15.sp,
                    fontFamily = VynlFonts.display,
                )
            },
            textStyle = TextStyle(
                fontSize = 15.sp,
                color = VynlColors.text,
                fontFamily = VynlFonts.display,
            ),
            keyboardOptions = KeyboardOptions(
                imeAction = ImeAction.Done,
            ),
            keyboardActions = KeyboardActions(
                onDone = { viewModel.completeOnboarding(name) },
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
        Spacer(Modifier.height(10.dp))
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "Leave blank for a random nickname",
                color = VynlColors.faint,
                fontSize = 12.sp,
                fontFamily = VynlFonts.display,
            )
            Text(
                text = "Roll",
                color = VynlColors.accent,
                fontSize = 13.sp,
                fontWeight = FontWeight.W700,
                fontFamily = VynlFonts.display,
                modifier = Modifier
                    .clip(RoundedCornerShape(VynlRadius.control))
                    .clickable { name = randomNickname() }
                    .padding(horizontal = 8.dp, vertical = 6.dp),
            )
        }

        Spacer(Modifier.height(30.dp))
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(52.dp)
                .clip(RoundedCornerShape(VynlRadius.control))
                .border(1.dp, VynlColors.accent.copy(alpha = 0.3f), RoundedCornerShape(VynlRadius.control))
                .background(VynlColors.accent.copy(alpha = 0.2f))
                .clickable { viewModel.completeOnboarding(name) },
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "START LISTENING",
                color = VynlColors.accent,
                fontSize = 14.sp,
                fontWeight = FontWeight.W600,
                letterSpacing = 1.4.sp,
                fontFamily = VynlFonts.display,
            )
        }
        Spacer(Modifier.height(40.dp))
    }
}
