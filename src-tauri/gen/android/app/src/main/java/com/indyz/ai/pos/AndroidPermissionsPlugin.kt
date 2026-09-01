package com.indyz.ai.pos

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.ActivityCompat
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Runtime permissions are intentionally requested at the point of use. The
 * Bluetooth prompt appears only when a user scans, pairs, or prints; Android
 * notification access is requested only when alerts are enabled or used.
 */
@TauriPlugin(
  permissions = [
    Permission(
      strings = [Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT],
      alias = "bluetooth",
    ),
    Permission(
      strings = [Manifest.permission.POST_NOTIFICATIONS],
      alias = "notifications",
    ),
    Permission(
      strings = [Manifest.permission.CAMERA],
      alias = "camera",
    ),
  ],
)
class AndroidPermissionsPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun requestBluetoothPermissions(invoke: Invoke) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S || bluetoothGranted()) {
      invoke.resolve(grantedResult())
      return
    }
    requestPermissionForAlias("bluetooth", invoke, "onBluetoothPermissionResult")
  }

  @PermissionCallback
  fun onBluetoothPermissionResult(invoke: Invoke) {
    invoke.resolve(grantedResult(bluetoothGranted()))
  }

  @Command
  fun requestNotificationPermission(invoke: Invoke) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU || notificationsGranted()) {
      invoke.resolve(grantedResult())
      return
    }
    requestPermissionForAlias("notifications", invoke, "onNotificationPermissionResult")
  }

  @PermissionCallback
  fun onNotificationPermissionResult(invoke: Invoke) {
    invoke.resolve(grantedResult(notificationsGranted()))
  }

  @Command
  fun requestCameraPermission(invoke: Invoke) {
    if (cameraGranted()) {
      invoke.resolve(grantedResult())
      return
    }
    requestPermissionForAlias("camera", invoke, "onCameraPermissionResult")
  }

  @PermissionCallback
  fun onCameraPermissionResult(invoke: Invoke) {
    invoke.resolve(grantedResult(cameraGranted()))
  }

  private fun bluetoothGranted(): Boolean =
    ActivityCompat.checkSelfPermission(activity, Manifest.permission.BLUETOOTH_SCAN) == PackageManager.PERMISSION_GRANTED &&
      ActivityCompat.checkSelfPermission(activity, Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED

  private fun notificationsGranted(): Boolean =
    ActivityCompat.checkSelfPermission(activity, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED

  private fun cameraGranted(): Boolean =
    ActivityCompat.checkSelfPermission(activity, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED

  private fun grantedResult(granted: Boolean = true): JSObject = JSObject().put("granted", granted)
}
