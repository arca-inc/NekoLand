# NekoLand

Compagnon félin animé pour votre bureau, écrit nativement en Rust. Un chat parcourt vos écrans, poursuit une **pelote de laine** ou votre curseur de souris, se pose sur les fenêtres actives et réagit aux clics en direct des spectateurs via **Twitch Heat**.

Vous pouvez également **attraper et déplacer le chat à la souris ou au tactile** n'importe où sur votre écran !

> 🇬🇧 [README.md](README.md) · 🇩🇪 [README_DE.md](README_DE.md)

---

## ✨ Fonctionnalités

- **Drag & Drop interactif** : Attrapez le chat au clic gauche (souris ou écran tactile) pour le déplacer à volonté.
- **Confinement multi-écrans intelligent** : Le chat et la pelote restent strictement dans les zones réelles de vos écrans et ne tombent jamais dans le vide inter-écrans.
- **Click-Through dynamique** : Tout l'écran en dehors du chat reste 100 % traversable (aucun blocage de vos fenêtres, jeux ou navigateurs).
- **Dashboard & Paramètres épurés** : Configurez facilement les skins, pelotes, modes, taille, canal Twitch, langue et éditez les mappings de sprites dans une interface GTK4 moderne.
- **CLI complète** : Lancez avec `--help`, `--skin <nom>`, `--mode <mode>`, `--scale <taille>`, `--list-skins`, etc.
- **Installateurs multiplateformes** : Installation en une seule commande pour Linux, macOS et Windows.

---

## 🎮 Modes de jeu

- **Pelote (Jeu)** : Une pelote rebondit sur les bords de vos écrans visibles. Le chat la pourchasse, l'attrape, se repose un instant puis recommence.
- **Autonome (Errance)** : Le chat se promène librement sur vos écrans et fait la sieste en arrivant.
  - *Sous-mode Dock* : Si une fenêtre reste au premier plan plus d'1 minute, le chat vient marcher le long de son bord inférieur.
- **Souris (Chasseur)** : Le chat suit et pourchasse activement votre pointeur de souris.
- **Sommeil (Sieste)** : Le chat reste sur place et dort paisiblement.
- **Twitch Heat** : Si une chaîne de stream est configurée, les clics en direct des spectateurs deviennent prioritaires.

---

## 💻 Utilisation en ligne de commande (CLI)

```sh
nekoland [OPTIONS]

Options:
  -h, --help               Affiche ce message d'aide et quitte
  -v, --version            Affiche la version et quitte
  -s, --skin <NOM>         Définit le skin de départ (ex: neko, black, calico, dog)
  -t, --toy <NOM>          Définit le jouet (ex: wool, darkWool, boomerang)
  -m, --mode <MODE>        Mode de comportement : Pelote, Autonome, Souris, Sommeil
  -z, --scale <FACTEUR>    Facteur d'échelle (1.0, 1.5, 2.0, 3.0)
  -c, --twitch <CHAINE>    Canal Twitch pour interagir avec les clics du stream
  -l, --lang <LANGUE>      Force la langue (fr, en, de)
      --list-skins         Liste tous les skins disponibles et quitte
      --list-toys          Liste tous les jouets disponibles et quitte
  -d, --debug              Active le mode débogage avec repères visuels à l'écran
```

---

## 📦 Installation

### 🐧 Linux (Wayland & X11)

```sh
git clone https://github.com/arca-inc/NekoLand.git
cd NekoLand
./install.sh
```

- Installe le binaire dans `~/.local/bin/nekoland`
- Configure le raccourci `.desktop` et le lancement automatique
- Pour **Hyprland**, ajoutez `exec-once = ~/.local/bin/nekoland` dans `~/.config/hypr/hyprland.conf`
- Pour désinstaller : `./install.sh --uninstall`

### 🍏 macOS (Apple Silicon & Intel)

```sh
git clone https://github.com/arca-inc/NekoLand.git
cd NekoLand
./install_macos.sh
```

- Installe l'application `NekoLand.app` dans `~/Applications`
- Configure le symlink CLI et le démarrage automatique
- Pour désinstaller : `./install_macos.sh --uninstall`

### 🪟 Windows (10 & 11)

Dans PowerShell :

```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```

- Installe dans `%LOCALAPPDATA%\NekoLand`
- Crée les raccourcis sur le Bureau et dans le Menu Démarrer
- Configure l'autostart Windows (`HKCU\...\Run`)
- Ajoute le dossier au `PATH` utilisateur
- Pour désinstaller : `powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall`

---

## 🎨 Sprites & Skins personnalisés

Format : grille de 8×6 tuiles en 32×32 pixels (avec marge de 1 px). Pour ajouter un nouveau skin, déposez simplement son image `.png` dans le dossier `assets/pets/`. Ouvrez le **Dashboard** depuis l'icône de barre des tâches pour ajuster et sauvegarder le mapping d'animation.

## 📜 Crédits

- The Neko Archive Project & le logiciel original Neko.
- Merci à m4keuse (twitch.tv/m4keuse) pour l'idée, inspirée de NekoV2 d'Aqueuse (github.com/Aqueuse/NekoV2).
