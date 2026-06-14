//! Suivi de la fenêtre active (Hyprland, via `hyprctl activewindow -j`) pour le
//! mode « dock » : si la même fenêtre reste focus assez longtemps, le chat va se
//! promener sous son bord inférieur.
//!
//! Dégradation gracieuse : si `hyprctl` est absent (autre compositeur), la
//! fenêtre est simplement marquée invalide et le mode dock ne s'active jamais.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

/// État de la fenêtre focus, en coordonnées globales du compositeur.
pub struct Focus {
    pub addr: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Instant où *cette* fenêtre a pris le focus (réinitialisé au changement).
    pub since: Instant,
    pub valid: bool,
}

impl Focus {
    fn empty() -> Self {
        Focus {
            addr: String::new(),
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            since: Instant::now(),
            valid: false,
        }
    }
}

/// Lance le thread de sondage (1 Hz) et renvoie l'état partagé.
pub fn spawn() -> Arc<Mutex<Focus>> {
    let shared = Arc::new(Mutex::new(Focus::empty()));
    let out = shared.clone();
    std::thread::spawn(move || loop {
        let info = query();
        {
            let mut f = out.lock().unwrap();
            match info {
                Some((addr, x, y, w, h)) => {
                    if addr != f.addr {
                        f.since = Instant::now();
                        f.addr = addr;
                    }
                    f.x = x;
                    f.y = y;
                    f.w = w;
                    f.h = h;
                    f.valid = true;
                }
                None => {
                    f.valid = false;
                    f.addr.clear();
                }
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    });
    shared
}

/// Interroge Hyprland. `None` si pas de fenêtre focus ou hyprctl indisponible.
fn query() -> Option<(String, f64, f64, f64, f64)> {
    let out = std::process::Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    let addr = v.get("address")?.as_str()?.to_string();
    if addr.is_empty() {
        return None;
    }
    let at = v.get("at")?.as_array()?;
    let size = v.get("size")?.as_array()?;
    Some((
        addr,
        at.first()?.as_f64()?,
        at.get(1)?.as_f64()?,
        size.first()?.as_f64()?,
        size.get(1)?.as_f64()?,
    ))
}
