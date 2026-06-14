#!/usr/bin/env bash
# Installe neko-desktop pour l'utilisateur courant + configure l'autostart.
#   - binaire   → ~/.local/bin/neko-desktop
#   - assets    → ~/.local/share/neko-desktop/assets   (trouvés par assets_dir())
#   - .desktop  → ~/.local/share/applications + ~/.config/autostart
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.local/share/neko-desktop"
APP_DIR="${HOME}/.local/share/applications"
AUTOSTART_DIR="${HOME}/.config/autostart"
BIN="${BIN_DIR}/neko-desktop"

echo "==> Compilation (release)…"
cargo build --release --manifest-path "${SCRIPT_DIR}/Cargo.toml"

echo "==> Binaire → ${BIN}"
mkdir -p "${BIN_DIR}"
install -m755 "${SCRIPT_DIR}/target/release/neko_rust" "${BIN}"

echo "==> Assets → ${DATA_DIR}/assets"
mkdir -p "${DATA_DIR}/assets"
cp -r "${SCRIPT_DIR}/assets/." "${DATA_DIR}/assets/"

echo "==> Outils → ${DATA_DIR}/tools"
mkdir -p "${DATA_DIR}/tools"
cp -r "${SCRIPT_DIR}/tools/." "${DATA_DIR}/tools/"

echo "==> Entrées .desktop"
mkdir -p "${APP_DIR}" "${AUTOSTART_DIR}"
read -r -d '' DESKTOP <<EOF || true
[Desktop Entry]
Type=Application
Name=Neko Desktop
Comment=Un chat qui poursuit une pelote sur le bureau
Exec=${BIN}
Icon=face-smile
Terminal=false
Categories=Game;
X-GNOME-Autostart-enabled=true
EOF
printf '%s\n' "$DESKTOP" >"${APP_DIR}/neko-desktop.desktop"
printf '%s\n' "$DESKTOP" >"${AUTOSTART_DIR}/neko-desktop.desktop"

cat <<EOF

✅ Installé.

  Lancer maintenant :   ${BIN}

  Autostart :
    • Bureaux lisant le XDG autostart (KDE, GNOME…) : déjà actif via
      ${AUTOSTART_DIR}/neko-desktop.desktop
    • Hyprland : ajoute cette ligne à ~/.config/hypr/hyprland.conf
        exec-once = ${BIN}

  Désinstaller : rm -f "${BIN}" "${APP_DIR}/neko-desktop.desktop" \\
      "${AUTOSTART_DIR}/neko-desktop.desktop"; rm -rf "${DATA_DIR}"
EOF
