<#
.SYNOPSIS
    Installateur NekoLand pour Windows.
.DESCRIPTION
    Installe NekoLand, ses dépendances, ses assets, crée les raccourcis
    sur le Bureau et dans le Menu Démarrer, et configure le démarrage automatique.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File install.ps1
    powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall
#>

param(
    [switch]$Uninstall = $false
)

$ErrorActionPreference = "Stop"

$IsFrench = [System.Globalization.CultureInfo]::CurrentUICulture.Name.StartsWith("fr")

$InstallDir = Join-Path $env:LOCALAPPDATA "NekoLand"
$ExePath = Join-Path $InstallDir "nekoland.exe"
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$StartShortcut = Join-Path $StartMenuDir "NekoLand.lnk"
$DesktopDir = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::Desktop)
$DesktopShortcut = Join-Path $DesktopDir "NekoLand.lnk"
$AutoStartKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$AutoStartName = "Nekoland"

if ($Uninstall) {
    if ($IsFrench) {
        Write-Host "==> Désinstallation de NekoLand..." -ForegroundColor Yellow
    } else {
        Write-Host "==> Uninstalling NekoLand..." -ForegroundColor Yellow
    }

    # Supprime la clé d'autostart
    try {
        Remove-ItemProperty -Path $AutoStartKey -Name $AutoStartName -ErrorAction SilentlyContinue
    } catch {}

    # Supprime les raccourcis
    if (Test-Path $DesktopShortcut) { Remove-Item $DesktopShortcut -Force -ErrorAction SilentlyContinue }
    if (Test-Path $StartShortcut) { Remove-Item $StartShortcut -Force -ErrorAction SilentlyContinue }

    # Supprime le dossier d'installation
    if (Test-Path $InstallDir) {
        Remove-Item $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    if ($IsFrench) {
        Write-Host "✅ NekoLand a été désinstallé avec succès." -ForegroundColor Green
    } else {
        Write-Host "✅ NekoLand has been successfully uninstalled." -ForegroundColor Green
    }
    exit 0
}

if ($IsFrench) {
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host "  Installation de NekoLand pour Windows  " -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
} else {
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host "    NekoLand Windows Installer           " -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Recherche du binaire source
$SourceExe = $null
if (Test-Path (Join-Path $ScriptDir "nekoland.exe")) {
    $SourceExe = Join-Path $ScriptDir "nekoland.exe"
} elseif (Test-Path (Join-Path $ScriptDir "target\x86_64-pc-windows-gnu\release\nekoland.exe")) {
    $SourceExe = Join-Path $ScriptDir "target\x86_64-pc-windows-gnu\release\nekoland.exe"
} elseif (Test-Path (Join-Path $ScriptDir "target\release\nekoland.exe")) {
    $SourceExe = Join-Path $ScriptDir "target\release\nekoland.exe"
} else {
    if ($IsFrench) {
        Write-Host "❌ Aucun binaire 'nekoland.exe' trouvé." -ForegroundColor Red
        Write-Host "   Veuillez compiler avec 'cargo build --release' ou utiliser l'archive de release." -ForegroundColor Red
    } else {
        Write-Host "❌ No 'nekoland.exe' binary found." -ForegroundColor Red
        Write-Host "   Please build with 'cargo build --release' or use a release bundle." -ForegroundColor Red
    }
    exit 1
}

# Création du dossier d'installation
if ($IsFrench) {
    Write-Host "==> Création du dossier d'installation dans $InstallDir..." -ForegroundColor Gray
} else {
    Write-Host "==> Creating installation directory at $InstallDir..." -ForegroundColor Gray
}
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

# Copie de l'exécutable
Copy-Item -Path $SourceExe -Destination $ExePath -Force

# Copie des DLLs si présentes dans le dossier source
Get-ChildItem -Path $ScriptDir -Filter "*.dll" -ErrorAction SilentlyContinue | ForEach-Object {
    Copy-Item -Path $_.FullName -Destination $InstallDir -Force
}

# Copie des dossiers assets, lib et share
@("assets", "lib", "share") | ForEach-Object {
    $srcFolder = Join-Path $ScriptDir $_
    if (Test-Path $srcFolder) {
        $destFolder = Join-Path $InstallDir $_
        Copy-Item -Path $srcFolder -Destination $destFolder -Recurse -Force
    }
}

# Création des raccourcis Bureau et Menu Démarrer
if ($IsFrench) {
    Write-Host "==> Création des raccourcis..." -ForegroundColor Gray
} else {
    Write-Host "==> Creating shortcuts..." -ForegroundColor Gray
}

$WshShell = New-Object -ComObject WScript.Shell

$Shortcut = $WshShell.CreateShortcut($DesktopShortcut)
$Shortcut.TargetPath = $ExePath
$Shortcut.WorkingDirectory = $InstallDir
$Shortcut.Description = "NekoLand - Desktop Companion"
$Shortcut.Save()

$ShortcutStart = $WshShell.CreateShortcut($StartShortcut)
$ShortcutStart.TargetPath = $ExePath
$ShortcutStart.WorkingDirectory = $InstallDir
$ShortcutStart.Description = "NekoLand - Desktop Companion"
$ShortcutStart.Save()

# Configuration du démarrage automatique avec Windows
if ($IsFrench) {
    Write-Host "==> Configuration du démarrage automatique avec Windows..." -ForegroundColor Gray
} else {
    Write-Host "==> Configuring autostart with Windows..." -ForegroundColor Gray
}
Set-ItemProperty -Path $AutoStartKey -Name $AutoStartName -Value "`"$ExePath`"" -Type String

# Ajout au PATH utilisateur si non présent
$UserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
}

if ($IsFrench) {
    Write-Host ""
    Write-Host "✅ Installation terminée avec succès !" -ForegroundColor Green
    Write-Host ""
    Write-Host "  • Raccourci Bureau :        Créé sur votre Bureau"
    Write-Host "  • Menu Démarrer :           Disponible dans les applications"
    Write-Host "  • Démarrage automatique :   Activé à l'ouverture de session"
    Write-Host "  • En ligne de commande :    Vous pouvez taper 'nekoland --help'"
    Write-Host "  • Désinstallation :         powershell -File install.ps1 -Uninstall"
    Write-Host ""
} else {
    Write-Host ""
    Write-Host "✅ Installation completed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "  • Desktop Shortcut:        Created on your Desktop"
    Write-Host "  • Start Menu:              Available in Programs"
    Write-Host "  • Autostart:               Configured for Windows startup"
    Write-Host "  • Command line:            You can run 'nekoland --help'"
    Write-Host "  • Uninstall:               powershell -File install.ps1 -Uninstall"
    Write-Host ""
}
