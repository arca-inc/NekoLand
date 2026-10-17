#!/usr/bin/env bash
# NekoLand - Installateur macOS (Apple Silicon & Intel)
# Usage:
#   ./install_macos.sh             # Installe NekoLand.app
#   ./install_macos.sh --uninstall # Désinstalle NekoLand.app

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="${HOME}/Applications"
APP_BUNDLE="${APP_DIR}/NekoLand.app"
BIN_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/Library/Application Support/nekoland"
LAUNCH_AGENT="${HOME}/Library/LaunchAgents/com.warmadon.nekoland.plist"

is_fr() {
    [[ "${LANG:-}" =~ ^fr ]] || [[ "${LC_ALL:-}" =~ ^fr ]]
}

if [[ "${1:-}" == "--uninstall" || "${1:-}" == "-u" ]]; then
    if is_fr; then
        echo "==> Désinstallation de NekoLand..."
    else
        echo "==> Uninstalling NekoLand..."
    fi
    launchctl unload "${LAUNCH_AGENT}" 2>/dev/null || true
    rm -f "${LAUNCH_AGENT}"
    rm -rf "${APP_BUNDLE}"
    rm -f "${BIN_DIR}/nekoland"
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
    echo "   Installation de NekoLand pour macOS   "
    echo "========================================="
else
    echo "========================================="
    echo "    NekoLand macOS Installer             "
    echo "========================================="
fi

# Vérification de Homebrew
if ! command -v brew &>/dev/null; then
    if is_fr; then
        echo "❌ Homebrew n'est pas installé."
        echo "   Veuillez installer Homebrew depuis https://brew.sh avant de continuer."
    else
        echo "❌ Homebrew is not installed."
        echo "   Please install Homebrew from https://brew.sh before continuing."
    fi
    exit 1
fi

# Vérification des dépendances GTK4
if ! brew list gtk4 &>/dev/null; then
    if is_fr; then
        echo "==> Installation des dépendances GTK4 via Homebrew..."
    else
        echo "==> Installing GTK4 dependencies with Homebrew..."
    fi
    brew install gtk4 pkg-config
fi

# Vérification du binaire précompilé ou compilation
if [[ -f "${SCRIPT_DIR}/nekoland" ]]; then
    SOURCE_BIN="${SCRIPT_DIR}/nekoland"
elif [[ -f "${SCRIPT_DIR}/target/release/nekoland" ]]; then
    SOURCE_BIN="${SCRIPT_DIR}/target/release/nekoland"
else
    if ! command -v cargo &>/dev/null; then
        if is_fr; then
            echo "❌ Rust / Cargo est requis pour compiler NekoLand."
            echo "   Installez Rust via https://rustup.rs."
        else
            echo "❌ Rust / Cargo is required to build NekoLand."
            echo "   Install Rust from https://rustup.rs."
        fi
        exit 1
    fi
    if is_fr; then
        echo "==> Compilation native pour votre Mac..."
    else
        echo "==> Compiling native release binary for your Mac..."
    fi
    cargo build --release --manifest-path "${SCRIPT_DIR}/Cargo.toml"
    SOURCE_BIN="${SCRIPT_DIR}/target/release/nekoland"
fi

# Création du bundle Application macOS
if is_fr; then
    echo "==> Création de l'application ${APP_BUNDLE}..."
else
    echo "==> Creating application bundle at ${APP_BUNDLE}..."
fi
mkdir -p "${APP_DIR}"
mkdir -p "${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${APP_BUNDLE}/Contents/Resources"
mkdir -p "${DATA_DIR}/assets"

# Copie du binaire et des assets
cp "${SOURCE_BIN}" "${APP_BUNDLE}/Contents/MacOS/nekoland_bin"
if [[ -d "${SCRIPT_DIR}/assets" ]]; then
    cp -r "${SCRIPT_DIR}/assets/." "${DATA_DIR}/assets/"
    cp -r "${SCRIPT_DIR}/assets/." "${APP_BUNDLE}/Contents/Resources/assets/"
fi

# Script lanceur dans l'app (configure l'environnement Homebrew dynamiquement)
cat > "${APP_BUNDLE}/Contents/MacOS/nekoland" <<'EOF'
#!/bin/bash
DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -d "/opt/homebrew" ]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
elif [ -d "/usr/local" ]; then
    eval "$(/usr/local/bin/brew shellenv)"
fi
export NEKO_ASSETS="${DIR}/../Resources/assets"
exec "${DIR}/nekoland_bin" "$@"
EOF
chmod +x "${APP_BUNDLE}/Contents/MacOS/nekoland"

# Info.plist
cat > "${APP_BUNDLE}/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleName</key>
    <string>NekoLand</string>
    <key>CFBundleDisplayName</key>
    <string>NekoLand</string>
    <key>CFBundleIdentifier</key>
    <string>com.warmadon.nekoland</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>nekoland</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Symlink CLI optionnel dans ~/.local/bin
mkdir -p "${BIN_DIR}"
ln -sf "${APP_BUNDLE}/Contents/MacOS/nekoland" "${BIN_DIR}/nekoland"

# Configuration LaunchAgent pour l'autostart optionnel
mkdir -p "$(dirname "${LAUNCH_AGENT}")"
cat > "${LAUNCH_AGENT}" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.warmadon.nekoland</string>
    <key>ProgramArguments</key>
    <array>
        <string>${APP_BUNDLE}/Contents/MacOS/nekoland</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
EOF

if is_fr; then
    echo ""
    echo "✅ Installation terminée avec succès !"
    echo ""
    echo "  • Emplacement :            ${APP_BUNDLE}"
    echo "  • Lancer depuis Spotlight: Tapez 'NekoLand' dans Spotlight (Cmd+Espace)"
    echo "  • Lancer en ligne de commande : nekoland --help"
    echo "  • Démarrage automatique :  Activé au démarrage de session"
    echo "  • Désinstallation :        ./install_macos.sh --uninstall"
    echo ""
else
    echo ""
    echo "✅ Installation completed successfully!"
    echo ""
    echo "  • Location:                ${APP_BUNDLE}"
    echo "  • Launch with Spotlight:   Press Cmd+Space and type 'NekoLand'"
    echo "  • Launch in terminal:      nekoland --help"
    echo "  • Autostart:               Configured for login"
    echo "  • Uninstall:               ./install_macos.sh --uninstall"
    echo ""
fi
