package com.vynl.vynl_mobile.remote

import android.content.SharedPreferences
import android.util.Base64
import android.util.Log
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

object SecurePrefs {
    private const val PREFIX = "enc1:"
    private const val KEYSTORE = "AndroidKeyStore"
    private const val ALIAS = "vynl_token_key_v1"
    private const val GCM_TAG_BITS = 128
    private const val IV_BYTES = 12

    private fun secretKey(): SecretKey? = try {
        val keyStore = KeyStore.getInstance(KEYSTORE).apply { load(null) }
        (keyStore.getEntry(ALIAS, null) as? KeyStore.SecretKeyEntry)?.secretKey
            ?: KeyGenerator.getInstance(
                android.security.keystore.KeyProperties.KEY_ALGORITHM_AES,
                KEYSTORE,
            ).apply {
                init(
                    android.security.keystore.KeyGenParameterSpec.Builder(
                        ALIAS,
                        android.security.keystore.KeyProperties.PURPOSE_ENCRYPT or
                            android.security.keystore.KeyProperties.PURPOSE_DECRYPT,
                    )
                        .setBlockModes(android.security.keystore.KeyProperties.BLOCK_MODE_GCM)
                        .setEncryptionPaddings(android.security.keystore.KeyProperties.ENCRYPTION_PADDING_NONE)
                        .setKeySize(256)
                        .build(),
                )
            }.generateKey()
    } catch (error: Exception) {
        Log.w("Vynl", "keystore unavailable; tokens stay unencrypted", error)
        null
    }

    private fun encrypt(plain: String): String? = try {
        val key = secretKey() ?: return null
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key)
        val iv = cipher.iv
        val body = cipher.doFinal(plain.toByteArray(Charsets.UTF_8))
        PREFIX + Base64.encodeToString(iv + body, Base64.NO_WRAP)
    } catch (error: Exception) {
        Log.w("Vynl", "could not encrypt token", error)
        null
    }

    private fun decrypt(payload: String): String? = try {
        val key = secretKey() ?: return null
        val bytes = Base64.decode(payload, Base64.NO_WRAP)
        if (bytes.size <= IV_BYTES + 16) {
            null
        } else {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(
                Cipher.DECRYPT_MODE,
                key,
                GCMParameterSpec(GCM_TAG_BITS, bytes, 0, IV_BYTES),
            )
            String(cipher.doFinal(bytes, IV_BYTES, bytes.size - IV_BYTES), Charsets.UTF_8)
        }
    } catch (_: Exception) {
        null
    }

    fun getString(prefs: SharedPreferences, key: String, default: String?): String? {
        val raw = prefs.getString(key, null) ?: return default
        if (!raw.startsWith(PREFIX)) {
            encrypt(raw)?.let { encrypted ->
                prefs.edit().putString(key, encrypted).apply()
            }
            return raw
        }
        return decrypt(raw.removePrefix(PREFIX)) ?: default
    }

    fun putString(editor: SharedPreferences.Editor, key: String, value: String) {
        val encrypted = encrypt(value)
            ?: throw IllegalStateException("could not encrypt the token — try again")
        editor.putString(key, encrypted)
    }
}
