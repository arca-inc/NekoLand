//! Utilitaires : PRNG xorshift et gestion géométrique multi-écrans.

use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn rand_unit() -> f64 {
    thread_local!(static SEED: Cell<u64> = Cell::new(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15)
            | 1
    ));
    SEED.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl ScreenRect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        ScreenRect { x, y, w, h }
    }

    #[inline]
    pub fn contains_point(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= (self.x + self.w) && py >= self.y && py <= (self.y + self.h)
    }

    #[inline]
    pub fn contains_rect(&self, rx: f64, ry: f64, rw: f64, rh: f64) -> bool {
        rx >= self.x && (rx + rw) <= (self.x + self.w) && ry >= self.y && (ry + rh) <= (self.y + self.h)
    }

    #[inline]
    pub fn clamp_rect(&self, rx: f64, ry: f64, rw: f64, rh: f64) -> (f64, f64) {
        let max_x = (self.x + self.w - rw).max(self.x);
        let max_y = (self.y + self.h - rh).max(self.y);
        (rx.clamp(self.x, max_x), ry.clamp(self.y, max_y))
    }
}

/// Vérifie si un point se trouve sur au moins un moniteur physique.
pub fn is_point_on_any_screen(px: f64, py: f64, screens: &[ScreenRect]) -> bool {
    screens.iter().any(|s| s.contains_point(px, py))
}

/// Sélectionne un point aléatoire garanti à l'intérieur d'un écran réel visible.
pub fn random_point_on_screens(screens: &[ScreenRect], margin_w: f64, margin_h: f64) -> (f64, f64) {
    if screens.is_empty() {
        return (100.0, 100.0);
    }
    // Choix aléatoire d'un moniteur
    let idx = (rand_unit() * screens.len() as f64).floor() as usize % screens.len();
    let s = screens[idx];
    let max_w = (s.w - margin_w).max(0.0);
    let max_h = (s.h - margin_h).max(0.0);
    (s.x + rand_unit() * max_w, s.y + rand_unit() * max_h)
}

/// Projette et clampe une boîte (sprite) sur l'écran le plus adapté,
/// empêchant strictement le sprite de sortir ou d'errer dans un vide inter-écrans.
pub fn clamp_to_screens(rx: f64, ry: f64, rw: f64, rh: f64, screens: &[ScreenRect]) -> (f64, f64) {
    if screens.is_empty() {
        return (rx.max(0.0), ry.max(0.0));
    }

    let cx = rx + rw / 2.0;
    let cy = ry + rh / 2.0;

    // 1. Si le centre est sur un écran, on clampe sur cet écran
    for s in screens {
        if s.contains_point(cx, cy) {
            return s.clamp_rect(rx, ry, rw, rh);
        }
    }

    // 2. Si le rectangle n'est sur aucun écran (ex: dans un trou ou hors limites),
    // on trouve l'écran le plus proche du centre et on projette dedans
    let mut best_dist = f64::MAX;
    let mut best_screen = screens[0];

    for s in screens {
        let scx = s.x + s.w / 2.0;
        let scy = s.y + s.h / 2.0;
        let dist = (cx - scx).powi(2) + (cy - scy).powi(2);
        if dist < best_dist {
            best_dist = dist;
            best_screen = *s;
        }
    }

    best_screen.clamp_rect(rx, ry, rw, rh)
}
