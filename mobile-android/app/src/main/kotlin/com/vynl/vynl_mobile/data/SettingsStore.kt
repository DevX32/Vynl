package com.vynl.vynl_mobile.data

import android.content.Context

/** The few settings the phone keeps between runs. */
class SettingsStore(context: Context) {
    private val prefs = context.getSharedPreferences("vynl", Context.MODE_PRIVATE)

    var accent: Int
        get() = prefs.getInt(KEY_ACCENT, DEFAULT_ACCENT)
        set(value) = prefs.edit().putInt(KEY_ACCENT, value).apply()

    var accentSet: Boolean
        get() = prefs.contains(KEY_ACCENT)
        set(value) = prefs.edit().putBoolean(KEY_ACCENT_SET, value).apply()

    companion object {
        const val KEY_ACCENT = "accent"
        const val KEY_ACCENT_SET = "accent_set"
        const val DEFAULT_ACCENT = 0xFFA894E8.toInt()
    }
}
