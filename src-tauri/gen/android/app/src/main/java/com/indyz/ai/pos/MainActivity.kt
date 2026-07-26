package com.indyz.ai.pos

import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    registerPlugin(SecureStoragePlugin(this))
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }
}
