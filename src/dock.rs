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
    pub class: String,
    pub title: String,
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
            class: String::new(),
            title: String::new(),
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
                Some(w) => {
                    if w.addr != f.addr {
                        f.since = Instant::now();
                        f.addr = w.addr;
                    }
                    f.class = w.class;
                    f.title = w.title;
                    f.x = w.x;
                    f.y = w.y;
                    f.w = w.w;
                    f.h = w.h;
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

/// Données brutes d'une fenêtre interrogée.
struct Win {
    addr: String,
    class: String,
    title: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Interroge Hyprland. `None` si pas de fenêtre focus ou hyprctl indisponible.
fn query() -> Option<Win> {
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
    Some(Win {
        addr,
        class: v.get("class").and_then(Value::as_str).unwrap_or("").to_string(),
        title: v.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
        x: at.first()?.as_f64()?,
        y: at.get(1)?.as_f64()?,
        w: size.first()?.as_f64()?,
        h: size.get(1)?.as_f64()?,
    })
}
