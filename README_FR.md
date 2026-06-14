# neko_rust

Neko desktop natif, en Rust. Un chat animé poursuit une **pelote de laine** qui rebondit sur le bureau ; quand il l'attrape, il se repose puis une nouvelle pelote réapparaît. En présence d'un canal **Twitch Heat**, il chasse en priorité les clics des viewers sur le stream.

## Comportement (gameplay)

Trois **modes** (commutables à chaud via le tray, persistés) :

- **Pelote** : une pelote rebondit en diagonale sur les bords de la zone globale (union des moniteurs). Le chat la poursuit, l'attrape par collision (< 24 px), elle se cache, le chat se repose (~2,5 s), puis une nouvelle pelote apparaît.
- **Autonome** : le chat erre vers des cibles aléatoires et fait la sieste en arrivant (le comportement « indépendant » de l'original). **Sous-mode « dock »** (Hyprland) : si une fenêtre reste focus > 1 min, le chat va se promener sous son bord inférieur. Dès que le focus change, il reprend son errance libre.
- **Sommeil** : le chat reste sur place et dort.

**Twitch Heat** est prioritaire dans tous les modes : si `twitch_channel` est configuré et qu'un clic de viewer arrive, le chat chasse cette cible.

## Sprites

Skins issus de l'archive oneko : <https://bomvel.neocities.org/neko/>.
Format : grille 8×6, tuiles 32×32 avec **1 px de séparation**.

Le mapping cellule → animation se fait via l'**éditeur natif GTK4** (`src/mapper.rs`), ouvert plein écran depuis le tray (**"Options / Dashboard"**). On choisit une animation, on clique les cellules dans l'ordre des frames, puis **"Enregistrer"**. L'app **recharge le mapping à chaud**.

## Crédits
Crédits : The Neko Archive Project & Neko (software).

## Installation & autostart

```sh
./install.sh
```
Compile en release, installe le binaire dans `~/.local/bin/neko-desktop`, copie les assets, et crée les entrées `.desktop` (applications + autostart XDG).
