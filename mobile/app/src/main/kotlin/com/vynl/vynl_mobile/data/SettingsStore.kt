package com.vynl.vynl_mobile.data

import android.content.Context

class SettingsStore(context: Context) {
    private val prefs = context.getSharedPreferences("vynl", Context.MODE_PRIVATE)

    var accent: Int
        get() = prefs.getInt(KEY_ACCENT, DEFAULT_ACCENT)
        set(value) = prefs.edit().putInt(KEY_ACCENT, value).apply()

    var accentSet: Boolean
        get() = prefs.contains(KEY_ACCENT)
        set(value) = prefs.edit().putBoolean(KEY_ACCENT_SET, value).apply()

    var displayName: String
        get() = prefs.getString(KEY_DISPLAY_NAME, "") ?: ""
        set(value) = prefs.edit().putString(KEY_DISPLAY_NAME, value).apply()

    var nickname: String
        get() = prefs.getString(KEY_NICKNAME, "") ?: ""
        set(value) = prefs.edit().putString(KEY_NICKNAME, value).apply()

    var onboarded: Boolean
        get() = prefs.getBoolean(KEY_ONBOARDED, prefs.getString(KEY_NICKNAME, "")?.isNotBlank() == true)
        set(value) = prefs.edit().putBoolean(KEY_ONBOARDED, value).apply()

    companion object {
        const val KEY_ACCENT = "accent"
        const val KEY_ACCENT_SET = "accent_set"
        const val KEY_DISPLAY_NAME = "vynl.display_name"
        const val KEY_NICKNAME = "vynl.nickname"
        const val KEY_ONBOARDED = "vynl.onboarded"
        const val DEFAULT_ACCENT = 0xFFA894E8.toInt()
    }
}
