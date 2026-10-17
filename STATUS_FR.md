# Statut des Plateformes

Statut actuel du support par plateforme.

> 🇬🇧 [STATUS.md](STATUS.md) · 🇩🇪 [STATUS_DE.md](STATUS_DE.md)

| Plateforme | Statut | Fonctionnalités |
|---|---|---|
| **Linux (Wayland & X11)** | ✅ **Testé & Supporté** | Overlay transparent, click-through, confinement multi-écrans, drag & drop du chat, icône de barre (ksni), dashboard, autostart (`install.sh`). |
| **Windows (10 & 11)** | ✅ **Testé & Supporté** | Bundle GTK4 autonome, mise à l'échelle HiDPI, overlay plein écran, drag & drop à la souris, tray système, autostart registre (`install.ps1`). |
| **macOS (Apple Silicon & Intel)** | ✅ **Supporté** | Runtime GTK4 natif, overlay flottant, tray, support complet CLI, bundle `NekoLand.app` et autostart LaunchAgent (`install_macos.sh`). |

---

Voir `.github/workflows/release.yml` pour les builds automatisés et la publication des paquets de release.
