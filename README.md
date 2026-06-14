# neko_rust

Neko desktop natif, en Rust. Un chat animé poursuit une **pelote de laine** qui
rebondit sur le bureau ; quand il l'attrape, il se repose puis une nouvelle
pelote réapparaît. En présence d'un canal **Twitch Heat**, il chasse en priorité
les clics des viewers sur le stream.

## Comportement (gameplay)

Trois **modes** (commutables à chaud via le tray, persistés) :

- **Pelote** (`src/toy.rs`) : une pelote rebondit en diagonale sur les bords de la
  zone globale (union des moniteurs). Le chat la poursuit, l'attrape par collision
  (< 24 px), elle se cache, le chat se repose (~2,5 s : alerte → toilette →
  fatigue → sommeil), puis une nouvelle pelote apparaît.
- **Autonome** : le chat erre vers des cibles aléatoires et fait la sieste en
  arrivant (le comportement « indépendant » de l'original).
- **Sommeil** : le chat reste sur place et dort.

**Twitch Heat** est prioritaire dans tous les modes : si `twitch_channel` est
configuré et qu'un clic de viewer arrive, le chat chasse cette cible.

## Pourquoi cette stack

Voir l'analyse complète, mais en bref : les tentatives précédentes (Tauri, PyQt,
Java) utilisaient une **petite fenêtre qui se déplace toute seule** — un pattern
**interdit par Wayland** (une fenêtre ne peut pas se repositionner en
coordonnées globales). Ici on inverse : **une seule fenêtre overlay fixe, plein
écran, click-through**, dans laquelle on *dessine* le chat. Seul le dessin bouge.

| Pièce | Rôle |
|---|---|
| **GTK4 + `gtk4-layer-shell`** | Overlay always-on-top, click-through, via le protocole `wlr-layer-shell` |
| **Cairo / `DrawingArea`** | Blit de la tuile 32×32 du sprite-sheet oneko (`oneko_neko.png`) |
| **`tokio-tungstenite`** | WebSocket Twitch Heat (`wss://heat-api.j38.net/...`) |

## Sprites

Skins issus de l'archive oneko : <https://bomvel.neocities.org/neko/>.
Format : grille 8×6, tuiles 32×32 avec **1 px de séparation** (origine d'une
tuile = `(col*33, row*33)`). Le fond plein est retiré au chargement en rendant
transparente la couleur du pixel (0,0) (`Pixbuf::add_alpha`).

Le mapping cellule → animation se fait via l'**éditeur natif GTK4** (`src/mapper.rs`),
ouvert plein écran depuis le tray (**« Configurer les sprites… »**, sur le skin
courant). On choisit une animation, on clique les cellules dans l'ordre des
frames, puis **« Enregistrer »** écrit `assets/pets/<skin>.json` et l'app
**recharge le mapping à chaud** (surveillance du mtime). Pour ajouter un skin :
dépose son `*.png` dans `assets/pets/` (il apparaît dans le menu **Chat**).

> Un éditeur web autonome subsiste dans `tools/sprite_mapper.html` (non relié à
> l'app), si tu préfères mapper dans un navigateur.

## Dépendances système (Gentoo)

```sh
sudo emerge -av gui-libs/gtk gui-libs/gtk4-layer-shell
```

Vérifier :
```sh
pkg-config --exists gtk4 && echo gtk4 OK
pkg-config --exists gtk4-layer-shell-0 && echo layer-shell OK
```

## Lancer

Depuis ce dossier (le sprite est chargé en chemin relatif) :
```sh
cargo run
```

Avec l'intégration Twitch Heat :
```sh
NEKO_TWITCH_CHANNEL=ton_user_id cargo run
```
(`ton_user_id` = l'ID numérique de la chaîne Twitch ; voir https://heat.j38.net)

## Installation & autostart

```sh
./install.sh
```
Compile en release, installe le binaire dans `~/.local/bin/neko-desktop`, copie
les assets dans `~/.local/share/neko-desktop/assets` (résolus automatiquement par
`assets_dir()`), et crée les entrées `.desktop` (applications + autostart XDG).

**Hyprland** ne lit pas l'autostart XDG par défaut — ajoute plutôt à
`~/.config/hypr/hyprland.conf` :
```
exec-once = ~/.local/bin/neko-desktop
```

## Compatibilité

| Compositeur | État |
|---|---|
| Sway / Hyprland / wlroots | ✅ supporté (layer-shell natif) |
| KDE Plasma (Wayland) | ✅ supporté (KWin gère layer-shell) |
| **GNOME (Wayland)** | ❌ Mutter ne supporte pas layer-shell — limite connue, inhérente à GNOME |
| X11 | ✅ layer-shell fonctionne aussi sous X11 |

## Tray icon

Icône de barre système via `ksni` (StatusNotifierItem). Menus :
- **Mode** — Pelote / Autonome / Sommeil
- **Chat** — choisir le skin (liste scannée dans `assets/pets/`)
- **Pelote** — choisir le jouet (`assets/toys/`)
- **Taille** — 1× / 1,5× / 2× / 3×
- **Quitter**

Chaque choix s'applique **à chaud** (sans relancer) et est **persisté** dans
`~/.config/neko-desktop/config.json`. Nécessite un hôte SNI (Waybar avec le
module `tray`, etc.).

## Multi-écran

Un overlay layer-shell est créé **par moniteur** (`set_monitor`). Le chat évolue
dans un espace de coordonnées global (union de tous les moniteurs) et chaque
overlay le dessine décalé de l'offset de son moniteur — il traverse donc les
écrans de façon continue.

## TODO / prochaines étapes

- [ ] Menu tray enrichi : changer de skin / régler la taille sans relancer.
- [ ] Mode « chasse le curseur local » — limité sous Wayland (pas de position
      globale du pointeur) ; faisable en restreignant l'input-region au chat.
- [ ] Taille réglable à l'exécution (`NEKO_SCALE`).
- [ ] Fallback Windows/macOS : `#[cfg]` vers une fenêtre normale always-on-top
      auto-positionnée (le pattern fenêtre-mobile y est autorisé).
