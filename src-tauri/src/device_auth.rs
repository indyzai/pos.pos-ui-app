//! device_auth.rs
//! 
//! OS keychain storage for device tokens and OS-level authentication.
//! Uses the `keyring` crate which backends to:
//!   - macOS: Keychain
//!   - Windows: Credential Manager
//!   - Linux: Secret Service / libsecret
//!
//! Authentication uses platform-native APIs:
//!   - macOS: LocalAuthentication (Touch ID + device password fallback)
//!   - Windows: Windows Hello
//!   - Android/iOS: Biometric + PIN via Tauri plugin

use keyring::Entry;

/// Store a token in the OS keychain under the given service name.
#[tauri::command]
pub fn store_device_token(service: String, token: String) -> Result<(), String> {
    let entry = Entry::new(&service, "indyzai-pos-device")
        .map_err(|e| e.to_string())?;
    entry.set_password(&token)
        .map_err(|e| format!("Keychain write failed: {}", e))
}

/// Retrieve a token from the OS keychain.
#[tauri::command]
pub fn get_device_token(service: String) -> Result<String, String> {
    let entry = Entry::new(&service, "indyzai-pos-device")
        .map_err(|e| e.to_string())?;
    entry.get_password()
        .map_err(|e| format!("Keychain read failed: {}", e))
}

/// Delete a token from the OS keychain.
#[tauri::command]
pub fn delete_device_token(service: String) -> Result<(), String> {
    let entry = Entry::new(&service, "indyzai-pos-device")
        .map_err(|e| e.to_string())?;
    entry.delete_credential()
        .map_err(|e| format!("Keychain delete failed: {}", e))
}

/// Prompt OS authentication (Touch ID / Windows Hello / device PIN).
/// On macOS uses LAPolicyDeviceOwnerAuthentication which covers biometrics
/// and falls back to the device password automatically.
#[tauri::command]
#[allow(unused_variables)]
pub async fn authenticate_device(reason: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let script = format!(
            "use framework \"LocalAuthentication\"\n\
             set ctx to current application's LAContext's new()\n\
             set {{res, err}} to ctx's evaluatePolicy:(current application's LAPolicyDeviceOwnerAuthentication) localizedReason:\"{}\" error:(reference)\n\
             if res is false then error \"Authentication failed\"",
            reason.replace('"', "\\\"")
        );
        let output = Command::new("osascript")
            .args(["-l", "AppleScript", "-e", &script])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            Err(if err.is_empty() { "Authentication cancelled".to_string() } else { err })
        }
    }
    #[cfg(target_os = "windows")]
    {
        // Windows Hello via PowerShell
        let script = "
            Add-Type -AssemblyName PresentationCore
            $dialog = [Windows.Security.Credentials.UI.UserConsentVerifier]
            $result = $dialog::RequestVerificationAsync('Verify your identity to access Indyz POS').GetAwaiter().GetResult()
            if ($result -ne 'Verified') { exit 1 }
        ";
        let output = std::process::Command::new("powershell")
            .args(["-Command", script])
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() { Ok(()) } else { Err("Windows Hello authentication failed".to_string()) }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Mobile (Android/iOS) — use Tauri biometric plugin or just return Ok
        // The actual biometric challenge happens via the frontend plugin on mobile
        Ok(())
    }
}

/// Check whether OS authentication (biometrics / device password) is available.
#[tauri::command]
pub fn check_device_auth_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("bioutil")
            .args(["--status"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows Hello and mobile biometrics are assumed available
        cfg!(any(target_os = "windows", target_os = "android", target_os = "ios"))
    }
}
