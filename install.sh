#!/usr/bin/env bash
# NekoLand - Installateur Linux (GNOME, KDE, Hyprland, Sway, XFCE...)
# Usage:
#   ./install.sh             # Installe ou compile NekoLand
#   ./install.sh --uninstall # Désinstalle NekoLand

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.local/share/nekoland"
APP_DIR="${HOME}/.local/share/applications"
AUTOSTART_DIR="${HOME}/.config/autostart"
BIN="${BIN_DIR}/nekoland"

is_fr() {
    [[ "${LANG:-}" =~ ^fr ]] || [[ "${LC_ALL:-}" =~ ^fr ]]
}

if [[ "${1:-}" == "--uninstall" || "${1:-}" == "-u" ]]; then
    if is_fr; then
        echo "==> Désinstallation de NekoLand..."
    else
        echo "==> Uninstalling NekoLand..."
    fi
    rm -f "${BIN}"
    rm -f "${APP_DIR}/nekoland.desktop"
    rm -f "${AUTOSTART_DIR}/nekoland.desktop"
    rm -rf "${DATA_DIR}"
    if is_fr; then
        echo "✅ NekoLand a été désinstallé avec succès."
    else
        echo "✅ NekoLand has been successfully uninstalled."
    fi
    exit 0
fi

if is_fr; then
    echo "========================================="
    echo "  Installation de NekoLand pour Linux    "
    echo "========================================="
else
    echo "========================================="
    echo "    NekoLand Linux Installer             "
    echo "========================================="
fi

# Vérification du binaire précompilé ou compilation via cargo
if [[ -f "${SCRIPT_DIR}/nekoland" ]]; then
    SOURCE_BIN="${SCRIPT_DIR}/nekoland"
elif [[ -f "${SCRIPT_DIR}/target/release/nekoland" ]]; then
    SOURCE_BIN="${SCRIPT_DIR}/target/release/nekoland"
else
    if ! command -v cargo &>/dev/null; then
        if is_fr; then
            echo "❌ 'cargo' (Rust) est requis pour compiler NekoLand."
            echo "   Installez Rust via https://rustup.rs ou le gestionnaire de paquets de votre distribution."
        else
            echo "❌ 'cargo' (Rust) is required to build NekoLand."
            echo "   Install Rust from https://rustup.rs or using your system package manager."
        fi
        exit 1
    fi
    if is_fr; then
        echo "==> Compilation en mode release avec Cargo..."
    else
        echo "==> Building in release mode with Cargo..."
    fi
    cargo build --release --manifest-path "${SCRIPT_DIR}/Cargo.toml"
    SOURCE_BIN="${SCRIPT_DIR}/target/release/nekoland"
fi

# Installation du binaire
if is_fr; then
    echo "==> Copie du binaire vers ${BIN}..."
else
    echo "==> Installing binary to ${BIN}..."
fi
mkdir -p "${BIN_DIR}"
install -m755 "${SOURCE_BIN}" "${BIN}"

# Installation des assets
if is_fr; then
    echo "==> Copie des assets vers ${DATA_DIR}/assets..."
else
    echo "==> Copying assets to ${DATA_DIR}/assets..."
fi
mkdir -p "${DATA_DIR}/assets"
if [[ -d "${SCRIPT_DIR}/assets" ]]; then
    cp -r "${SCRIPT_DIR}/assets/." "${DATA_DIR}/assets/"
fi

# Entrée Desktop et Autostart
if is_fr; then
    echo "==> Configuration des raccourcis et du démarrage automatique..."
else
    echo "==> Configuring desktop shortcuts and autostart..."
fi
mkdir -p "${APP_DIR}" "${AUTOSTART_DIR}"

cat > "${APP_DIR}/nekoland.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=NekoLand
Comment=Compagnon félin animé pour votre bureau
Exec=${BIN}
Icon=face-smile
Terminal=false
Categories=Game;Utility;
X-GNOME-Autostart-enabled=true
EOF

cp "${APP_DIR}/nekoland.desktop" "${AUTOSTART_DIR}/nekoland.desktop"

# Vérification du PATH
PATH_NOTE=""
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    if is_fr; then
        PATH_NOTE="⚠️  ${BIN_DIR} n'est pas dans votre \$PATH. Ajoutez 'export PATH=\"\$HOME/.local/bin:\$PATH\"' à votre ~/.bashrc ou ~/.zshrc."
    else
        PATH_NOTE="⚠️  ${BIN_DIR} is not in your \$PATH. Add 'export PATH=\"\$HOME/.local/bin:\$PATH\"' to your ~/.bashrc or ~/.zshrc."
    fi
fi

if is_fr; then
    echo ""
    echo "✅ Installation terminée !"
    echo ""
    echo "  • Lancer maintenant :      ${BIN}"
    echo "  • Lancer avec options :    ${BIN} --help"
    echo "  • Démarrage automatique :  Déjà actif via ~/.config/autostart/nekoland.desktop"
    echo "  • Sous Hyprland :          Ajoutez 'exec-once = ${BIN}' à votre hyprland.conf"
    echo "  • Désinstallation :        ./install.sh --uninstall"
    if [[ -n "${PATH_NOTE}" ]]; then
        echo ""
        echo "  ${PATH_NOTE}"
    fi
    echo ""
else
    echo ""
    echo "✅ Installation complete!"
    echo ""
    echo "  • Launch now:              ${BIN}"
    echo "  • Launch with options:     ${BIN} --help"
    echo "  • Autostart:               Configured via ~/.config/autostart/nekoland.desktop"
    echo "  • On Hyprland:             Add 'exec-once = ${BIN}' to your hyprland.conf"
    echo "  • Uninstall:               ./install.sh --uninstall"
    if [[ -n "${PATH_NOTE}" ]]; then
        echo ""
        echo "  ${PATH_NOTE}"
    fi
    echo ""
fi
