package com.indyz.ai.pos

import android.app.Activity
import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.nio.charset.StandardCharsets
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Credentials are AES-GCM encrypted with a non-exportable Android Keystore key. */
@TauriPlugin
class SecureStoragePlugin(private val activity: Activity) : Plugin(activity) {
  private val prefs = activity.getSharedPreferences("indyz_secure_storage", Context.MODE_PRIVATE)
  private val alias = "indyz_pos_device_secret"

  private fun key(): SecretKey {
    val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    (store.getKey(alias, null) as? SecretKey)?.let { return it }
    val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
    generator.init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
      .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
    return generator.generateKey()
  }

  @Command fun set(invoke: Invoke) { val service = invoke.parseArgs(Args::class.java).service; val value = invoke.parseArgs(Args::class.java).value; val cipher=Cipher.getInstance("AES/GCM/NoPadding"); cipher.init(Cipher.ENCRYPT_MODE,key()); prefs.edit().putString(service, Base64.encodeToString(cipher.iv,Base64.NO_WRAP)+":"+Base64.encodeToString(cipher.doFinal(value.toByteArray(StandardCharsets.UTF_8)),Base64.NO_WRAP)).apply(); invoke.resolve() }
  @Command fun get(invoke: Invoke) { val service=invoke.parseArgs(Args::class.java).service; val encoded=prefs.getString(service,null) ?: return invoke.reject("secret not found"); val parts=encoded.split(":",limit=2); val cipher=Cipher.getInstance("AES/GCM/NoPadding"); cipher.init(Cipher.DECRYPT_MODE,key(),GCMParameterSpec(128,Base64.decode(parts[0],Base64.NO_WRAP))); invoke.resolve(JSObject().put("value",String(cipher.doFinal(Base64.decode(parts[1],Base64.NO_WRAP)),StandardCharsets.UTF_8))) }
  @Command fun delete(invoke: Invoke) { prefs.edit().remove(invoke.parseArgs(Args::class.java).service).apply(); invoke.resolve() }
  data class Args(val service:String,val value:String="")
}
