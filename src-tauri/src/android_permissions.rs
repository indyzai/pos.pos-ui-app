//! Just-in-time Android runtime permission bridge.
//!
//! Android 12+ requires nearby-device permissions at runtime and Android 13+
//! requires notification permission at runtime. Keeping the prompt here lets
//! callers request access immediately before the feature that needs it.

#[cfg(target_os = "android")]
use serde::Deserialize;
#[cfg(target_os = "android")]
use tauri::Manager;

#[cfg(target_os = "android")]
pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("android-permissions")
        .setup(|app, api| {
            let handle = api.register_android_plugin(
                "com.indyz.ai.pos",
                "AndroidPermissionsPlugin",
            )?;
            app.manage(AndroidPermissionsPlugin(handle));
            Ok(())
        })
        .build()
}

#[cfg(not(target_os = "android"))]
pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("android-permissions").build()
}

#[cfg(target_os = "android")]
pub struct AndroidPermissionsPlugin<R: tauri::Runtime>(tauri::plugin::PluginHandle<R>);

#[cfg(target_os = "android")]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PermissionResult {
    granted: bool,
}

/// Requests nearby-device access only when the user begins Bluetooth printer
/// discovery, pairing, or printing.
#[tauri::command]
#[cfg(target_os = "android")]
pub fn request_android_bluetooth_permissions(
    plugin: tauri::State<'_, AndroidPermissionsPlugin<tauri::Wry>>,
) -> Result<(), String> {
    let result = plugin
        .0
        .run_mobile_plugin::<PermissionResult>("requestBluetoothPermissions", ())
        .map_err(|error| format!("Unable to request Bluetooth permission: {error}"))?;

    if result.granted {
        Ok(())
    } else {
        Err("Bluetooth permission was denied. Allow Nearby devices in Android Settings to scan or print.".into())
    }
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn request_android_bluetooth_permissions() -> Result<(), String> {
    Ok(())
}

/// Requests notification access only when the app is about to use notification
/// alerts, rather than showing an unrelated prompt on startup.
#[tauri::command]
#[cfg(target_os = "android")]
pub fn request_android_notification_permission(
    plugin: tauri::State<'_, AndroidPermissionsPlugin<tauri::Wry>>,
) -> Result<(), String> {
    let result = plugin
        .0
        .run_mobile_plugin::<PermissionResult>("requestNotificationPermission", ())
        .map_err(|error| format!("Unable to request notification permission: {error}"))?;

    if result.granted {
        Ok(())
    } else {
        Err("Notification permission was denied. You can enable it later in Android Settings.".into())
    }
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn request_android_notification_permission() -> Result<(), String> {
    Ok(())
}

/// Requests camera access only when the user opens a barcode or QR scanner.
#[tauri::command]
#[cfg(target_os = "android")]
pub fn request_android_camera_permission(
    plugin: tauri::State<'_, AndroidPermissionsPlugin<tauri::Wry>>,
) -> Result<(), String> {
    let result = plugin
        .0
        .run_mobile_plugin::<PermissionResult>("requestCameraPermission", ())
        .map_err(|error| format!("Unable to request camera permission: {error}"))?;

    if result.granted {
        Ok(())
    } else {
        Err("Camera permission was denied. Allow Camera access in Android Settings to scan barcodes.".into())
    }
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn request_android_camera_permission() -> Result<(), String> {
    Ok(())
}
