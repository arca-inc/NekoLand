//! Position globale du curseur (Hyprland, via `hyprctl cursorpos -j`) pour le
//! mode « Souris » : le chat chasse le pointeur de la souris.
//!
//! L'overlay est click-through (région d'input vide), donc GTK ne reçoit aucun
//! événement de mouvement : il faut interroger le compositeur. Dégradation
//! gracieuse : si `hyprctl` est absent (autre compositeur), la position est
//! marquée invalide et le mode Souris ne bouge pas le chat.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

/// Position du curseur en coordonnées globales du compositeur.
pub struct Cursor {
    pub x: f64,
    pub y: f64,
    pub valid: bool,
}

impl Cursor {
    fn empty() -> Self {
        Cursor { x: 0.0, y: 0.0, valid: false }
    }
}

/// Lance le thread de sondage (~20 Hz) et renvoie l'état partagé.
pub fn spawn() -> Arc<Mutex<Cursor>> {
    let shared = Arc::new(Mutex::new(Cursor::empty()));
    let out = shared.clone();
    std::thread::spawn(move || loop {
        match query() {
            Some((x, y)) => {
                let mut c = out.lock().unwrap();
                c.x = x;
                c.y = y;
                c.valid = true;
            }
            None => {
                out.lock().unwrap().valid = false;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    });
    shared
}

/// Interroge la position du curseur.
fn query() -> Option<(f64, f64)> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut point = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
        unsafe {
            if GetCursorPos(&mut point) != 0 {
                return Some((point.x as f64, point.y as f64));
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        let out = std::process::Command::new("hyprctl")
            .args(["cursorpos", "-j"])
            .output()
            .ok()?;
        let v: Value = serde_json::from_slice(&out.stdout).ok()?;
        Some((v.get("x")?.as_f64()?, v.get("y")?.as_f64()?))
    }
}
