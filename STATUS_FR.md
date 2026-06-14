# État des plateformes

État par plateforme au moment présent. NekoLand est jeune : le cœur est
fonctionnel sous Linux/Wayland, le reste est en cours.

> 🇬🇧 [STATUS.md](STATUS.md) · 🇩🇪 [STATUS_DE.md](STATUS_DE.md)

| Plateforme | État | Détails |
|---|---|---|
| **Linux — Wayland / Hyprland** | ✅ **Testé & fonctionnel** | Overlay transparent always-on-top, click-through, multi-moniteur, tray, dashboard, modes (Pelote / Autonome / Souris / Sommeil), mode streamer Twitch. |
| Linux — X11 / autres compositeurs | ⚠️ Non testé | Devrait marcher via GTK4 ; les modes « dock » et « Souris » dépendent de `hyprctl` (dégradation gracieuse sinon). |
| **Windows** | 🚧 **En cours — pas encore fonctionnel** | Le binaire se compile (MinGW/GTK4) ; le bundling des DLL du runtime GTK4 est en cours de fiabilisation. Pas encore validé sur une machine réelle. |
| **macOS** | 🚧 **En cours — pas encore fonctionnel** | Le binaire universel (Intel + Apple Silicon) est en cours de mise au point dans la CI ; les dylibs GTK4 ne sont pas encore embarquées (nécessite `brew install gtk4`). Pas encore validé. |

## Résumé

- **Linux Wayland/Hyprland** est la cible de référence : testé et fonctionnel.
- **Windows** et **macOS** sont **expérimentaux** : la compilation est traitée
  côté CI, mais le packaging autonome et la validation sur machine réelle ne
  sont **pas terminés**. Ne pas considérer comme utilisables pour l'instant.

Voir `.github/workflows/release.yml` pour l'état des builds CI par plateforme.
