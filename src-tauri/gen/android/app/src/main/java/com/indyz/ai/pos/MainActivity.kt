package com.indyz.ai.pos

import android.content.Intent
import android.os.Bundle
import android.util.Log
import androidx.core.view.WindowCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    logAuthIntent("onCreate", intent)
    super.onCreate(savedInstanceState)
    // Keep the Tauri WebView inside Android's system-bar and cutout insets.
    // Android resolves these values for the actual device and orientation, so
    // the app avoids navigation/camera cutouts without adding browser padding.
    WindowCompat.setDecorFitsSystemWindows(window, true)
  }

  override fun onNewIntent(intent: Intent) {
    logAuthIntent("onNewIntent", intent)
    super.onNewIntent(intent)
    // Keep Activity.intent in sync for the deep-link plugin's getCurrent API.
    setIntent(intent)
  }

  private fun logAuthIntent(stage: String, intent: Intent?) {
    val data = intent?.data
    if (data?.scheme == "indyzai-pos" || data?.host == "auth.indyzai.com") {
      Log.i(
        "IndyzAuth",
        "$stage action=${intent.action} scheme=${data.scheme} host=${data.host} path=${data.path} hasCode=${data.getQueryParameter("code") != null}",
      )
    } else {
      Log.d("IndyzAuth", "$stage action=${intent?.action} (not an auth callback)")
    }
  }
}
