/**
 * pos-ui-app — App.tsx
 * 
 * Tauri host app entry point.
 * Responsibilities:
 *  1. Initialize the Tauri SQLite DB adapter
 *  2. Configure pos-api library (API URLs, storage, Tauri env flag)
 *  3. Start the offline sync manager
 *  4. Render TauriLoginPage until authenticated, then render the main POS UI
 * 
 * The actual POS UI components are imported from pos-ui (packages/pos-web)
 * or can be built inline here for the Tauri-specific shell.
 */
import { useEffect, useState } from "react";
import { initPosLib, setDbAdapter, initLocalDb, authService } from "@indyzai/pos-api";
import { TauriDbAdapter, TauriLoginPage } from "@indyzai/pos-tauri";
import { offlineSyncManager } from "@indyzai/pos-offline";
import { refreshDeviceSession } from "@indyzai/pos-tauri";
import "./App.css";

type AppState = "initializing" | "login" | "app";

async function initApp() {
  // 1. Register the Tauri SQLite adapter
  const dbAdapter = new TauriDbAdapter();
  setDbAdapter(dbAdapter);

  // 2. Configure pos-api
  initPosLib({
    posGqlUrl: import.meta.env.VITE_POS_GQL_URL || "https://api.indyzai.com/pos/api/graphql",
    authApiUrl: import.meta.env.VITE_AUTH_API_URL || "https://api.indyzai.com",
    authUiUrl: import.meta.env.VITE_AUTH_UI_URL || "https://auth.indyzai.com",
    storage: localStorage,
    isTauriEnv: true,
    deepLinkScheme: import.meta.env.VITE_DEEP_LINK_SCHEME || "indyzai-pos",
    useReleaseUrls: import.meta.env.PROD,
  });

  // 3. Initialize local DB schema
  await initLocalDb();
}

function App() {
  const [appState, setAppState] = useState<AppState>("initializing");

  useEffect(() => {
    initApp()
      .then(() => {
        // Check if already authenticated
        const token = authService.storage.getToken();
        setAppState(token ? "login" : "login"); // always show login — TauriLoginPage handles the lock/unlock
      })
      .catch((err) => {
        console.error("App init failed:", err);
        setAppState("login"); // degrade gracefully
      });
  }, []);

  const handleLoginSuccess = async () => {
    // Start background sync when user is authenticated
    const deviceId = localStorage.getItem("indyz_device_id");
    offlineSyncManager.start(
      deviceId ? () => refreshDeviceSession() : null,
      deviceId
    );
    setAppState("app");
  };

  if (appState === "initializing") {
    return (
      <div className="app-loading">
        <div className="app-loading-spinner" />
        <p>Starting Indyz POS…</p>
      </div>
    );
  }

  if (appState === "login") {
    return (
      <TauriLoginPage
        onLoginSuccess={handleLoginSuccess}
      />
    );
  }

  // Main POS shell — in a full implementation this would render the POS router
  // For now, renders a placeholder until the full web app is embedded
  return (
    <div className="app-shell">
      <p>POS App Loaded — replace this with the main POS router.</p>
      <button
        onClick={() => {
          offlineSyncManager.stop();
          authService.storage.clearAuth();
          setAppState("login");
        }}
      >
        Sign Out
      </button>
    </div>
  );
}

export default App;
