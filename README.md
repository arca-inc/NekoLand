# NekoLand

Native desktop NekoLand, written in Rust. An animated cat chases a **yarn ball** bouncing across the desktop; when caught, the cat rests before a new ball appears. If a **Twitch Heat** channel is active, it prioritizes chasing viewers' clicks on the stream.

## Behavior (Gameplay)

Three **modes** (hot-swappable via the tray menu, persisted):

- **Toy**: A yarn ball bounces diagonally along the edges of the global area (union of all monitors). The cat chases it, catches it by collision (< 24 px), the ball hides, the cat rests (~2.5s: alert → wash → tired → sleep), and then a new ball appears.
- **Autonomous**: The cat wanders toward random targets and naps upon arrival (the "independent" behavior of the original). **Dock sub-mode** (Hyprland): if a window remains focused > 1 min, the cat will stroll horizontally along its bottom edge—"posing" on the window. As soon as focus changes, it resumes free roaming.
- **Sleep**: The cat stays in place and sleeps.

**Twitch Heat** takes priority in all modes: if `twitch_channel` is configured and a viewer click arrives, the cat chases that target.

## Why this stack?

Previous attempts (Tauri, PyQt, Java) used a **small self-moving window**—a pattern **forbidden by Wayland** (a window cannot reposition itself in global coordinates). Here we reverse the approach: **a single fixed fullscreen overlay, click-through**, in which we *draw* the cat. Only the drawing moves.

| Component | Role |
|---|---|
| **GTK4 + `gtk4-layer-shell`** | Always-on-top, click-through overlay via the `wlr-layer-shell` protocol |
| **Cairo / `DrawingArea`** | Blitting the 32x32 tile of the oneko sprite-sheet |
| **`tokio-tungstenite`** | Twitch Heat WebSocket (`wss://heat-api.j38.net/...`) |

## Sprites

Skins from the oneko archive: <https://bomvel.neocities.org/neko/>.
Format: 8x6 grid, 32x32 tiles with **1 px separation** (tile origin = `(col*33, row*33)`). Solid backgrounds are removed by making the pixel (0,0) color transparent (`Pixbuf::add_alpha`).

Cell → animation mapping is done via the **native GTK4 editor** (`src/mapper.rs`), opened fullscreen from the tray (**"Options / Dashboard"**). You choose an animation, click cells in frame order, and click **"Save mapping"** to write `assets/pets/<skin>.json`. The app **hot-reloads the mapping**. To add a skin: drop its `*.png` in `assets/pets/` (it will appear in the Dashboard).

## Credits
Credits: The Neko Archive Project & Neko (software).

## Installation & Autostart

```sh
./install.sh
```
Compiles in release mode, installs the binary to `~/.local/bin/nekoland`, copies assets to `~/.local/share/nekoland/assets` (auto-resolved by `assets_dir()`), and creates `.desktop` entries (applications + XDG autostart).

**Hyprland** does not read XDG autostart by default — add this to `~/.config/hypr/hyprland.conf`:
```
exec-once = ~/.local/bin/nekoland
```
