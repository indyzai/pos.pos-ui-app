# Indyz POS — Native Desktop & Mobile App (`pos.pos-ui-app`)

Cross-platform native desktop and mobile container application for **Indyz POS**, built using **[Tauri v2](https://v2.tauri.app/)** and **Rust**.

This repository hosts the native client shell that packages and embeds the frontend web application bundle produced by [`pos.pos-ui`](../pos.pos-ui), extending it with native system integrations (hardware ESC/POS printing, Bluetooth, deep linking, offline SQLite, and secure OS keychain storage).

---

update

## 🎯 Supported Platforms

| Platform | Output Artifacts | Status |
| :--- | :--- | :--- |
| **macOS** | `.dmg`, `.app` (Universal Apple Silicon & Intel) | ✅ Supported |
| **Windows** | `.msi`, `.exe` (x64) | ✅ Supported |
| **Linux** | `.deb`, `.AppImage` (x64) | ✅ Supported |
| **Android** | `.apk`, `.aab` (ARM64 / x86_64) | ✅ Supported |
| **iOS** | `.ipa` / Xcode Archive | ✅ Supported |

---

## 🏗️ Architecture & Integrations

```
┌────────────────────────────────────────────────────────┐
│               Frontend Web Application                  │
│       (Bundled from `pos.pos-ui` into `bundle/`)       │
└──────────────────────────┬─────────────────────────────┘
                           │ IPC Bridge (Tauri v2)
┌──────────────────────────▼─────────────────────────────┐
│                 Tauri Native Rust Core                 │
│                                                        │
│  ├─ 🔐 OS Keychain (`keyring`) — Token Storage        │
│  ├─ 🔗 Deep Links (`indyzai-pos://`) — Auth Callback   │
│  ├─ 💾 Offline Storage (`rusqlite` / SQL Plugin)       │
│  ├─ 🖨️ Native Printing (Bluetooth LE, Network, Spool) │
│  └─ 🌐 Native HTTP Client (`reqwest` with Rustls)      │
└────────────────────────────────────────────────────────┘
```

### Key Native Capabilities:
- **Deep Linking**: Registers custom URI schemes (`indyzai-pos://auth/callback`) and HTTPS domain routes for authentication code exchanges directly inside the app.
- **Hardware Printing**: Direct ESC/POS printing support across Network (TCP 9100), Bluetooth Low Energy (via `btleplug`), and Windows/macOS native spoolers.
- **Offline SQLite**: Local embedded SQLite database via `@tauri-apps/plugin-sql` / `rusqlite` for robust offline point-of-sale resilience.
- **Secure Keychain**: Encrypted credential and refresh token management via macOS Keychain, Windows Credential Manager, and Linux Secret Service.

---

## 🛠️ Prerequisites & Setup

### 1. System Requirements
- **Node.js**: LTS (v20+) & **Yarn**
- **Rust Toolchain**: Stable (`rustup default stable`)
- **Platform-Specific Dependencies**:
  - **macOS**: Xcode 15+ and Command Line Tools (`xcode-select --install`)
  - **Windows**: Microsoft C++ Build Tools or Visual Studio
  - **Linux (Ubuntu/Debian)**:
    ```bash
    sudo apt-get update
    sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
    ```
  - **Android**: Android Studio with SDK (API 34+), NDK (`27.0.12077973`), and JDK 17 (`JAVA_HOME` set)
  - **iOS**: macOS with Xcode 15+ and CocoaPods

### 2. Installation
Clone the repository and install frontend dependencies:
```bash
yarn install
```

---

## 💻 Development Workflow

The desktop app loads the static web bundle compiled from `pos.pos-ui`.

### Step 1: Prepare the Web Bundle
Generate the static frontend bundle from `pos.pos-ui` or provide an archive:
```bash
# Option A: From adjacent repository
cd ../pos.pos-ui
yarn bundle

# Option B: Extract to pos-ui-app bundle directory
cd ../pos.pos-ui-app
yarn prepare:bundle
```

### Step 2: Run in Development Mode
Start the local bundle static server and launch Tauri Dev:
```bash
yarn dev
```

---

## 📦 Build Commands

| Command | Description |
| :--- | :--- |
| `yarn build` | Prepares bundle and builds default desktop installer for current OS |
| `yarn build:macos` | Builds macOS DMG and `.app` bundles |
| `yarn build:windows` | Builds Windows x64 binaries using `cargo-xwin` |
| `yarn build:android` | Compiles Android release `.apk` and `.aab` bundles |
| `yarn build:ios` | Compiles iOS Xcode project / archive |
| `yarn prepare:bundle` | Unpacks web frontend ZIP into `bundle/` for Tauri embedding |

---

## 🚀 Release & Tagging

Automated builds and releases are managed via GitHub Actions:

1. Ensure changes are committed and pushed to `main`.
2. Generate and push a new release tag:
   ```bash
   ./gen-tag.sh
   ```
3. GitHub Actions (`.github/workflows/tauri-build.yml`) will automatically:
   - Download the corresponding `web-dist.zip` release bundle from `indyzai/pos.pos-ui`.
   - Compile desktop binaries for macOS, Windows, and Linux.
   - Build Android APK/AAB and iOS packages.
   - Publish all compiled release artifacts under the created tag.
