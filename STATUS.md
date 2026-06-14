# Platform status

Current per-platform status. NekoLand is young: the core is functional on
Linux/Wayland, the rest is in progress.

> 🇫🇷 [STATUS_FR.md](STATUS_FR.md) · 🇩🇪 [STATUS_DE.md](STATUS_DE.md)

| Platform | Status | Details |
|---|---|---|
| **Linux — Wayland / Hyprland** | ✅ **Tested & working** | Transparent always-on-top overlay, click-through, multi-monitor, tray, dashboard, modes (Yarn / Autonomous / Mouse / Sleep), Twitch streamer mode. |
| Linux — X11 / other compositors | ⚠️ Untested | Should work via GTK4; the "dock" and "Mouse" modes rely on `hyprctl` (graceful fallback otherwise). |
| **Windows** | 🚧 **In progress — not yet working** | The binary compiles (MinGW/GTK4); bundling the GTK4 runtime DLLs is being stabilized. Not yet validated on a real machine. |
| **macOS** | 🚧 **In progress — not yet working** | Universal binary (Intel + Apple Silicon) is being sorted out in CI; the GTK4 dylibs are not bundled yet (requires `brew install gtk4`). Not yet validated. |

## Summary

- **Linux Wayland/Hyprland** is the reference target: tested and working.
- **Windows** and **macOS** are **experimental**: compilation is handled in CI,
  but self-contained packaging and real-machine validation are **not done**.
  Do not consider them usable yet.

See `.github/workflows/release.yml` for the CI build status per platform.
