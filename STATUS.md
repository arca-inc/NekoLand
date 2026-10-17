# Platform Status

Current per-platform support status.

> 🇫🇷 [STATUS_FR.md](STATUS_FR.md) · 🇩🇪 [STATUS_DE.md](STATUS_DE.md)

| Platform | Status | Features |
|---|---|---|
| **Linux (Wayland & X11)** | ✅ **Tested & Supported** | Transparent overlay, click-through, multi-screen bounds, cat drag & drop, tray (ksni), dashboard, autostart (`install.sh`). |
| **Windows (10 & 11)** | ✅ **Tested & Supported** | Standalone GTK4 runtime bundle, HiDPI scaling, full-screen overlay, mouse drag & drop, system tray, registry autostart (`install.ps1`). |
| **macOS (Apple Silicon & Intel)** | ✅ **Supported** | GTK4 native runtime, floating overlay, tray, full CLI support, `NekoLand.app` bundle and LaunchAgent autostart (`install_macos.sh`). |

---

See `.github/workflows/release.yml` for automated multi-platform builds and artifact releases.
