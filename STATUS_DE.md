# Plattform-Status

Aktueller Status je Plattform. NekoLand ist jung: der Kern funktioniert unter
Linux/Wayland, der Rest ist in Arbeit.

> 🇬🇧 [STATUS.md](STATUS.md) · 🇫🇷 [STATUS_FR.md](STATUS_FR.md)

| Plattform | Status | Details |
|---|---|---|
| **Linux — Wayland / Hyprland** | ✅ **Getestet & funktionsfähig** | Transparentes Always-on-top-Overlay, Click-through, Multi-Monitor, Tray, Dashboard, Modi (Wollknäuel / Autonom / Maus / Schlaf), Twitch-Streamer-Modus. |
| Linux — X11 / andere Compositoren | ⚠️ Ungetestet | Sollte über GTK4 laufen; die Modi „Dock" und „Maus" benötigen `hyprctl` (sonst sanfter Fallback). |
| **Windows** | 🚧 **In Arbeit — noch nicht funktionsfähig** | Das Binary kompiliert (MinGW/GTK4); das Bündeln der GTK4-Runtime-DLLs wird gerade stabilisiert. Noch nicht auf einer echten Maschine validiert. |
| **macOS** | 🚧 **In Arbeit — noch nicht funktionsfähig** | Universal-Binary (Intel + Apple Silicon) wird in der CI eingerichtet; die GTK4-Dylibs sind noch nicht eingebettet (`brew install gtk4` nötig). Noch nicht validiert. |

## Zusammenfassung

- **Linux Wayland/Hyprland** ist die Referenzplattform: getestet und
  funktionsfähig.
- **Windows** und **macOS** sind **experimentell**: das Kompilieren erfolgt in
  der CI, aber eigenständiges Packaging und Validierung auf echter Hardware
  sind **nicht abgeschlossen**. Noch nicht als nutzbar betrachten.

Siehe `.github/workflows/release.yml` für den CI-Build-Status je Plattform.
