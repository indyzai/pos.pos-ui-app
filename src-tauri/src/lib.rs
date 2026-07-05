mod device_auth;
mod printer;
mod bluetooth;

use tauri::{AppHandle, Emitter, Listener};
#[cfg(desktop)]
use tauri::menu::Menu;

// ─── Auth window commands ─────────────────────────────────────────────────────

#[tauri::command]
async fn open_auth_window(handle: AppHandle, url: String) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri::Manager;
        if let Some(window) = handle.get_webview_window("auth") {
            let _ = window.set_focus();
            return Ok(());
        }

        tauri::WebviewWindowBuilder::new(
            &handle,
            "auth",
            tauri::WebviewUrl::External(url.parse().unwrap()),
        )
        .title("Login")
        .inner_size(800.0, 600.0)
        .center()
        .on_navigation({
            let handle_clone = handle.clone();
            move |url| {
                // Accept navigation if it looks like an auth callback
                let is_callback = url.scheme() == "indyzai-pos"
                    || (url.host_str() == Some("auth.indyzai.com")
                        && url.path().contains("callback"));
                if is_callback {
                    let url_str = url.to_string();
                    let _ = handle_clone.emit("deep-link://new-url", vec![url_str]);
                    if let Some(w) = handle_clone.get_webview_window("auth") {
                        let _ = w.close();
                    }
                    return false;
                }
                true
            }
        })
        .build()
        .map_err(|e| e.to_string())?;
    }

    #[cfg(mobile)]
    {
        use tauri_plugin_opener::OpenerExt;
        handle
            .opener()
            .open_url(url, None::<String>)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn close_auth_window(handle: AppHandle) {
    #[cfg(desktop)]
    {
        use tauri::Manager;
        if let Some(window) = handle.get_webview_window("auth") {
            let _ = window.close();
        }
    }
}

// ─── Legacy biometric commands (kept for compatibility) ────────────────────

#[tauri::command]
fn check_biometric_available() -> bool {
    device_auth::check_device_auth_available()
}

#[tauri::command]
async fn authenticate_biometric(reason: String) -> Result<(), String> {
    device_auth::authenticate_device(reason).await
}

// ─── App entry point ──────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(|app| {
            let handle = app.handle().clone();

            #[cfg(desktop)]
            {
                if let Ok(menu) = Menu::default(&handle) {
                    if let Ok(settings_item) = tauri::menu::MenuItem::with_id(
                        &handle, "pos_settings", "Settings", true, None::<&str>,
                    ) {
                        if let Ok(offline_item) = tauri::menu::MenuItem::with_id(
                            &handle, "pos_offline", "Toggle Online/Offline Mode", true, None::<&str>,
                        ) {
                            if let Ok(admin_item) = tauri::menu::MenuItem::with_id(
                                &handle, "pos_admin", "Toggle Admin/Cashier Mode", true, None::<&str>,
                            ) {
                                if let Ok(pos_submenu) = tauri::menu::Submenu::with_items(
                                    &handle, "POS", true,
                                    &[&settings_item, &offline_item, &admin_item],
                                ) {
                                    #[cfg(target_os = "macos")]
                                    let _ = menu.insert(&pos_submenu, 1);
                                    #[cfg(not(target_os = "macos"))]
                                    let _ = menu.insert(&pos_submenu, 0);
                                }
                            }
                        }
                    }
                    let _ = app.set_menu(menu);
                }

                app.on_menu_event(move |app_handle, event| {
                    let id = event.id().as_ref();
                    if id == "pos_settings" || id == "pos_offline" || id == "pos_admin" {
                        let _ = app_handle.emit("pos-menu-action", id);
                    }
                });
            }

            // Deep-link callback listener
            // tauri-plugin-deep-link emits the payload as a JSON array of URL strings.
            app.listen_any("deep-link://new-url", move |event| {
                let data = event.payload();

                // Parse as JSON array first, fall back to single quoted string
                let urls: Vec<String> = serde_json::from_str(data)
                    .unwrap_or_else(|_| {
                        // Legacy / single-string fallback
                        let s = data.trim_matches('"').to_string();
                        vec![s]
                    });

                for url_str in urls {
                    if let Ok(url) = url_str.parse::<tauri::Url>() {
                        if url.scheme() == "indyzai-pos" && url.path().contains("auth/callback") {
                            if let Some(code) = url
                                .query_pairs()
                                .find(|(key, _)| key == "code")
                                .map(|(_, value)| value.to_string())
                            {
                                let _ = handle.emit("auth-code", code);
                            }
                            return;
                        }
                        // Also handle HTTPS callback (production)
                        if url.host_str() == Some("auth.indyzai.com")
                            && url.path().contains("callback")
                        {
                            if let Some(code) = url
                                .query_pairs()
                                .find(|(key, _)| key == "code")
                                .map(|(_, value)| value.to_string())
                            {
                                let _ = handle.emit("auth-code", code);
                            }
                            return;
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth window
            open_auth_window,
            close_auth_window,
            // Device OS auth
            device_auth::authenticate_device,
            device_auth::check_device_auth_available,
            // Keychain
            device_auth::store_device_token,
            device_auth::get_device_token,
            device_auth::delete_device_token,
            // Legacy biometric aliases
            check_biometric_available,
            authenticate_biometric,
            // Printer
            printer::get_system_printers,
            printer::print_raw_payload,
            // Bluetooth
            bluetooth::scan_bluetooth_printers,
            bluetooth::pair_bluetooth_printer,
            bluetooth::print_bluetooth_payload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
