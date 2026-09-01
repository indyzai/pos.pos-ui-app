mod device_auth;
mod printer;
mod bluetooth;
mod offline_store;

use tauri::{AppHandle, Emitter, Listener, Manager, State};
use serde::Serialize;
use offline_store::{OfflineStatus, OfflineStore};
#[cfg(desktop)]
use tauri::menu::Menu;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthDebugEvent {
    stage: String,
    detail: String,
}

fn emit_auth_debug(handle: &AppHandle, stage: impl Into<String>, detail: impl Into<String>) {
    let event = AuthDebugEvent {
        stage: stage.into(),
        detail: detail.into(),
    };
    println!("[IndyzAuth] native stage={} detail={}", event.stage, event.detail);
    let _ = handle.emit("auth-debug", event);
}

/// Return useful API error context without ever writing credentials, OAuth
/// codes, or complete response payloads to Android logs.
fn safe_api_error_detail(body: &str) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|value| value.get("message").or_else(|| value.get("error")))
        .and_then(|value| value.as_str())
        .unwrap_or("no-message");
    message.chars().take(240).collect()
}

fn response_keys(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|item| item.as_object())
        .map(|object| object.keys().cloned().collect::<Vec<_>>().join(","))
        .unwrap_or_else(|| "none".into())
}

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
        emit_auth_debug(&handle, "browser-open-native", "opening-external-browser");
        handle
            .opener()
            .open_url(url, None::<String>)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Emits safe login diagnostics to the native application log. Never include
/// OAuth codes, access tokens, refresh tokens, or full callback URLs here.
#[tauri::command]
fn auth_debug_log(stage: String, detail: String) {
    println!("[IndyzAuth] ui stage={stage} detail={detail}");
}

/// Exchanges an OAuth callback code without going through the WebView network
/// stack. This avoids Android WebView CORS preflights while keeping the code
/// and tokens out of diagnostic output.
#[tauri::command]
async fn exchange_auth_code(handle: AppHandle, code: String, auth_api_url: String) -> Result<serde_json::Value, String> {
    if code.trim().is_empty() {
        emit_auth_debug(&handle, "token-exchange-rejected", "empty-code");
        return Err("Authentication callback did not include a code".into());
    }

    let base = url::Url::parse(&auth_api_url)
        .map_err(|_| {
            emit_auth_debug(&handle, "token-exchange-rejected", "invalid-auth-api-url");
            "Invalid authentication API URL".to_string()
        })?;
    let is_local_http = base.scheme() == "http"
        && matches!(base.host_str(), Some("localhost") | Some("127.0.0.1") | Some("::1"));
    if base.scheme() != "https" && !is_local_http {
        emit_auth_debug(&handle, "token-exchange-rejected", "auth-api-must-use-https");
        return Err("Authentication API must use HTTPS".into());
    }

    let endpoint = format!("{}/auth/api/v1/auth/app/success", auth_api_url.trim_end_matches('/'));
    emit_auth_debug(&handle, "token-exchange-start", format!(
        "host={} scheme={} endpoint={}",
        base.host_str().unwrap_or(""),
        base.scheme(),
        endpoint,
    ));

    emit_auth_debug(&handle, "token-exchange-building-client", "begin");

    // Building a Rustls verifier may synchronously touch Android platform
    // state. Do it off the Tauri command runtime and bound the work: a stalled
    // native client must not block the WebView-first authentication flow.
    let client = match tokio::time::timeout(
        std::time::Duration::from_secs(8),
        tokio::task::spawn_blocking(|| {
            println!("[IndyzAuth] native token-exchange client-init root-store");
            let mut root_store = rustls::RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            println!("[IndyzAuth] native token-exchange client-init tls-config");
            let tls_config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            println!("[IndyzAuth] native token-exchange client-init reqwest-builder");
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .connect_timeout(std::time::Duration::from_secs(8))
                .user_agent("Indyz-POS-Tauri/1.0")
                .use_preconfigured_tls(tls_config)
                .pool_max_idle_per_host(0)
                .build()
                .map_err(|error| error.to_string())
        }),
    )
    .await
    {
        Err(_) => {
            emit_auth_debug(&handle, "token-exchange-failed", "client-init-timeout=8s");
            return Err("Native authentication client initialization timed out".into());
        }
        Ok(Err(error)) => {
            emit_auth_debug(&handle, "token-exchange-failed", format!("client-init-task={error}"));
            return Err("Could not initialize the authentication client".into());
        }
        Ok(Ok(Err(error))) => {
            emit_auth_debug(&handle, "token-exchange-failed", format!("client-init={error}"));
            return Err("Could not initialize the authentication client".into());
        }
        Ok(Ok(Ok(client))) => client,
    };
    emit_auth_debug(&handle, "token-exchange-client-ready", "ok");


    // Probe raw TCP reachability before the full TLS+HTTP round-trip.
    // This lets us distinguish "server unreachable/DNS failure" from a
    // TLS-layer hang in the debug log.
    let tcp_probe_host = format!(
        "{}:{}",
        base.host_str().unwrap_or("localhost"),
        base.port_or_known_default().unwrap_or(443),
    );
    emit_auth_debug(&handle, "token-exchange-tcp-probe", format!("addr={tcp_probe_host}"));
    match tokio::time::timeout(
        std::time::Duration::from_secs(6),
        tokio::net::TcpStream::connect(&tcp_probe_host),
    ).await {
        Ok(Ok(_))   => emit_auth_debug(&handle, "token-exchange-tcp-probe", "reachable"),
        Ok(Err(e))  => emit_auth_debug(&handle, "token-exchange-tcp-probe", format!("tcp-error={e}")),
        Err(_)      => emit_auth_debug(&handle, "token-exchange-tcp-probe", "timeout-6s"),
    }

    emit_auth_debug(&handle, "token-exchange-sending-request", format!(
        "method=POST url={endpoint}",
    ));
    let request = client
        .post(&endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, "application/json")
        .json(&serde_json::json!({ "code": code }));
    emit_auth_debug(&handle, "token-exchange-request-built", "awaiting-send");

    // Spawn onto a fresh Tokio task so that any OS-level blocking inside
    // Android's TLS stack (native-tls → conscrypt) does not starve the shared
    // Tauri command-executor threads.
    let request_task = tokio::spawn(request.send());
    let response = tokio::time::timeout(std::time::Duration::from_secs(15), request_task)
        .await
        .map_err(|_| {
            emit_auth_debug(&handle, "token-exchange-failed", "native-request-timeout=15s");
            "Authentication service request timed out".to_string()
        })?
        .map_err(|join_err| {
            emit_auth_debug(&handle, "token-exchange-failed", format!("task-panic={join_err}"));
            "Authentication request task panicked".to_string()
        })?
        .map_err(|error| {
            let kind = if error.is_timeout() {
                "timeout"
            } else if error.is_connect() {
                "connect"
            } else if error.is_request() {
                "request"
            } else if error.is_body() {
                "body"
            } else {
                "unknown"
            };
            emit_auth_debug(
                &handle,
                "token-exchange-failed",
                format!("network-kind={kind} network-detail={error}"),
            );
            "Could not reach the authentication service".to_string()
        })?;
    emit_auth_debug(&handle, "token-exchange-response-received", "reading-headers");


    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown");
    let content_length = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown");
    emit_auth_debug(
        &handle,
        "token-exchange-http-response",
        format!("status={status} content-type={content_type} content-length={content_length}"),
    );
    let body = response.text().await.map_err(|error| {
        emit_auth_debug(&handle, "token-exchange-failed", format!("response-read={error}"));
        "Could not read the authentication service response".to_string()
    })?;

    if !status.is_success() {
        emit_auth_debug(
            &handle,
            "token-exchange-failed",
            format!("http-status={status} api-message={}", safe_api_error_detail(&body)),
        );
        return Err(format!("Authentication service returned {status}"));
    }

    let payload = serde_json::from_str::<serde_json::Value>(&body).map_err(|error| {
            emit_auth_debug(&handle, "token-exchange-failed", "invalid-json");
            println!("[IndyzAuth] native token-exchange invalid-json error={error}");
            "Authentication service returned an invalid response".to_string()
        })?;
    let has_access_token = payload.pointer("/tokens/accessToken").and_then(|value| value.as_str()).is_some();
    let has_refresh_token = payload.pointer("/tokens/refreshToken").and_then(|value| value.as_str()).is_some();
    let has_user = payload.get("user").is_some();
    emit_auth_debug(
        &handle,
        "token-exchange-success",
        format!(
            "access={has_access_token} refresh={has_refresh_token} user={has_user} tokens-keys={} user-keys={}",
            response_keys(&payload, "tokens"),
            response_keys(&payload, "user"),
        ),
    );
    Ok(payload)
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

#[derive(Serialize)] struct OfflineServiceConfig { base_url: String, api_key: String }
#[tauri::command]
fn offline_service_config() -> Result<OfflineServiceConfig, String> {
    let output = std::process::Command::new(std::env::var("POS_OFFLINE_SERVICE_BIN").unwrap_or_else(|_| "pos-offline-svc".into())).arg("key").output().map_err(|e| format!("POS offline service is not installed: {e}"))?;
    if !output.status.success() { return Err("could not read POS offline service credential".into()); }
    Ok(OfflineServiceConfig { base_url: "http://127.0.0.1:8765".into(), api_key: String::from_utf8(output.stdout).map_err(|_| "invalid offline service credential")?.trim().into() })
}
#[tauri::command] fn mobile_offline_enqueue(store: State<'_, OfflineStore>, idempotency_key: String, action: String, payload: serde_json::Value) -> Result<(), String> { store.enqueue(&idempotency_key, &action, &payload.to_string()) }
#[tauri::command] fn mobile_offline_status(store: State<'_, OfflineStore>) -> Result<OfflineStatus, String> { store.status() }
#[tauri::command] fn mobile_offline_pending(store: State<'_, OfflineStore>) -> Result<Vec<offline_store::OfflineCommand>, String> { store.pending() }
#[tauri::command] fn mobile_offline_mark(store: State<'_, OfflineStore>, idempotency_key: String, status: String, conflict: Option<String>) -> Result<(), String> { store.mark(&idempotency_key, &status, conflict.as_deref()) }
#[tauri::command] fn mobile_offline_catalog_replace(store: State<'_, OfflineStore>, snapshot: serde_json::Value) -> Result<(), String> { store.replace_catalog(&snapshot.to_string()) }
#[tauri::command] fn mobile_offline_catalog(store: State<'_, OfflineStore>) -> Result<Option<serde_json::Value>, String> { store.catalog() }

// ─── App entry point ──────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(device_auth::init_secure_storage())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(|app| {
            let path = app.path().app_data_dir().map_err(|e| e.to_string())?.join("offline-mobile.db");
            if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
            app.manage(OfflineStore::open(&path)?);
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
                emit_auth_debug(&handle, "deep-link-event", format!("payload-bytes={}", data.len()));

                // Parse as JSON array first, fall back to single quoted string
                let urls: Vec<String> = serde_json::from_str(data).unwrap_or_else(|error| {
                        emit_auth_debug(&handle, "deep-link-payload-fallback", format!("json-error={error}"));
                        // Legacy / single-string fallback
                        let s = data.trim_matches('"').to_string();
                        vec![s]
                    });

                emit_auth_debug(&handle, "deep-link-urls", format!("count={}", urls.len()));

                for url_str in urls {
                    if let Ok(url) = url_str.parse::<tauri::Url>() {
                        emit_auth_debug(
                            &handle,
                            "deep-link-received",
                            format!(
                                "scheme={} host={} path={} has-code={}",
                                url.scheme(),
                                url.host_str().unwrap_or(""),
                                url.path(),
                                url.query_pairs().any(|(key, _)| key == "code"),
                            ),
                        );
                        // indyzai-pos://auth/callback is parsed as host `auth`
                        // and path `/callback`; only checking the path drops the
                        // real Android browser callback.
                        if url.scheme() == "indyzai-pos"
                            && ((url.host_str() == Some("auth")
                                && url.path().starts_with("/callback"))
                                || url.path().contains("auth/callback"))
                        {
                            if let Some(code) = url
                                .query_pairs()
                                .find(|(key, _)| key == "code")
                                .map(|(_, value)| value.to_string())
                            {
                                emit_auth_debug(&handle, "deep-link-accepted", "custom-callback");
                                match handle.emit("auth-code", code) {
                                    Ok(()) => emit_auth_debug(&handle, "auth-code-event", "custom-emitted"),
                                    Err(error) => emit_auth_debug(&handle, "auth-code-event-failed", error.to_string()),
                                }
                            } else {
                                emit_auth_debug(&handle, "deep-link-rejected", "custom-callback-missing-code");
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
                                emit_auth_debug(&handle, "deep-link-accepted", "https-callback");
                                match handle.emit("auth-code", code) {
                                    Ok(()) => emit_auth_debug(&handle, "auth-code-event", "https-emitted"),
                                    Err(error) => emit_auth_debug(&handle, "auth-code-event-failed", error.to_string()),
                                }
                            } else {
                                emit_auth_debug(&handle, "deep-link-rejected", "https-callback-missing-code");
                            }
                            return;
                        }
                        emit_auth_debug(&handle, "deep-link-ignored", "callback-host-or-path-did-not-match");
                    } else {
                        emit_auth_debug(&handle, "deep-link-rejected", "invalid-url");
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth window
            open_auth_window,
            close_auth_window,
            auth_debug_log,
            exchange_auth_code,
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
            offline_service_config,
            mobile_offline_enqueue,
            mobile_offline_status,
            mobile_offline_pending,
            mobile_offline_mark,
            mobile_offline_catalog_replace,
            mobile_offline_catalog,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
