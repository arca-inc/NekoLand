# NekoLand

Native desktop companion written in Rust. An animated cat roams your screens, chases a **yarn ball** or your mouse pointer, poses on windows, and reacts to **Twitch Heat** stream clicks.

You can also **click and drag the cat** anywhere across your screens!

> 🇫🇷 [README_FR.md](README_FR.md) · 🇩🇪 [README_DE.md](README_DE.md)

---

## ✨ Features

- **Interactive Drag & Drop**: Click or touch the cat with your mouse to pick it up and place it anywhere.
- **Smart Multi-Monitor Tracking**: Strictly confined to real physical displays. The cat and the yarn ball will never fall into voids or wander off-screen.
- **Dynamic Click-Through**: Everything outside the cat remains 100% click-through, never interrupting your work, gaming, or browsing.
- **Clean Dashboard & Settings**: Configure skins, toys, modes, scale, Twitch channel, language, and custom sprite mappings in a clean GTK4 window.
- **Full CLI Support**: Run with `--help`, `--skin <name>`, `--mode <mode>`, `--scale <factor>`, `--list-skins`, etc.
- **Multiplatform Installers**: One-command installation for Linux, macOS, and Windows.

---

## 🎮 Gameplay Modes

- **Yarn Ball (Pelote)**: A bouncy yarn ball ricochets across your active screens. The cat chases it, catches it, takes a short rest, and plays again.
- **Autonomous (Autonome)**: The cat explores your monitors on its own and naps upon arrival.
  - *Dock Sub-mode*: If a window remains focused for over 1 minute, the cat strolls along its bottom edge.
- **Mouse Hunter (Souris)**: The cat actively tracks and chases your mouse cursor.
- **Sleep (Sommeil)**: The cat curls up and sleeps peacefully where it is.
- **Twitch Heat**: When a Twitch streamer channel is specified, real-time viewer stream clicks take immediate priority.

---

## 💻 CLI Usage

```sh
nekoland [OPTIONS]

Options:
  -h, --help               Show help message and exit
  -v, --version            Show version and exit
  -s, --skin <NAME>        Set initial skin (e.g. neko, black, calico, dog)
  -t, --toy <NAME>         Set initial toy (e.g. wool, darkWool, boomerang)
  -m, --mode <MODE>        Behavior mode: Pelote/Yarn, Autonome/Auto, Souris/Mouse, Sommeil/Sleep
  -z, --scale <FACTOR>     Scale factor (1.0, 1.5, 2.0, 3.0)
  -c, --twitch <CHANNEL>   Twitch channel name for stream click interaction
  -l, --lang <LANG>        Force language (fr, en, de)
      --list-skins         List all available skins and exit
      --list-toys          List all available toys and exit
  -d, --debug              Enable debug logging and on-screen visual overlays
```

---

## 📦 Installation

### 🐧 Linux (Wayland & X11)

```sh
git clone https://github.com/arca-inc/NekoLand.git
cd NekoLand
./install.sh
```

- Installs binary to `~/.local/bin/nekoland`
- Sets up `.desktop` menu launcher and autostart
- For **Hyprland**, add `exec-once = ~/.local/bin/nekoland` to `~/.config/hypr/hyprland.conf`
- To uninstall: `./install.sh --uninstall`

### 🍏 macOS (Apple Silicon & Intel)

```sh
git clone https://github.com/arca-inc/NekoLand.git
cd NekoLand
./install_macos.sh
```

- Installs `NekoLand.app` into `~/Applications`
- Sets up CLI symlink and background autostart
- To uninstall: `./install_macos.sh --uninstall`

### 🪟 Windows (10 & 11)

In PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File install.ps1
```

- Installs to `%LOCALAPPDATA%\NekoLand`
- Creates Desktop & Start Menu shortcuts
- Configures registry autostart (`HKCU\...\Run`)
- Adds directory to user `PATH`
- To uninstall: `powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall`

---

## 🎨 Sprites & Custom Skins

Format: 8×6 grid of 32×32 tiles (with 1 px stride). To add a custom skin, place its `.png` in the `assets/pets/` directory. Open the **Dashboard** from the system tray or menu to edit and save animation mappings.

## 📜 Credits

- The Neko Archive Project & original Neko software.
- Special thanks to m4keuse (twitch.tv/m4keuse) for the idea, inspired by NekoV2 by Aqueuse (github.com/Aqueuse/NekoV2).
