package com.indyz.ai.pos

import android.content.Intent
import android.os.Bundle
import android.util.Log
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    logAuthIntent("onCreate", intent)
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
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
