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

/// Trouve l'écran qui contient le point (px, py).
pub fn find_screen_for_point(px: f64, py: f64, screens: &[ScreenRect]) -> Option<ScreenRect> {
    screens.iter().find(|s| s.contains_point(px, py)).copied()
}

/// Trouve l'écran géométriquement le plus proche d'un point (px, py).
pub fn closest_screen(px: f64, py: f64, screens: &[ScreenRect]) -> Option<ScreenRect> {
    screens.iter().min_by(|a, b| {
        let da = (px - (a.x + a.w / 2.0)).powi(2) + (py - (a.y + a.h / 2.0)).powi(2);
        let db = (px - (b.x + b.w / 2.0)).powi(2) + (py - (b.y + b.h / 2.0)).powi(2);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    }).copied()
}

/// Projette et clampe une boîte (sprite) sur l'ensemble des écrans physiques.
/// Permet au sprite de franchir sans encombre les frontières entre écrans adjacents
/// tout en l'empêchant strictement de sortir sur les bords extérieurs du bureau.
pub fn clamp_to_screens(rx: f64, ry: f64, rw: f64, rh: f64, screens: &[ScreenRect]) -> (f64, f64) {
    if screens.is_empty() {
        return (rx.max(0.0), ry.max(0.0));
    }

    let cx = rx + rw / 2.0;
    let cy = ry + rh / 2.0;

    // 1. Si le centre est sur un écran réel, on autorise le sprite à chevaucher
    // ou traverser vers un écran adjacent, mais on le retient sur les bords extérieurs dans le vide.
    if let Some(s) = find_screen_for_point(cx, cy, screens) {
        let mut clamped_x = rx;
        let mut clamped_y = ry;

        // Dépassement à gauche : autorisé seulement si un écran est présent à gauche
        if rx < s.x && !is_point_on_any_screen(rx, cy, screens) {
            clamped_x = s.x;
        }
        // Dépassement à droite : autorisé seulement si un écran est présent à droite
        if rx + rw > (s.x + s.w) && !is_point_on_any_screen(rx + rw, cy, screens) {
            clamped_x = s.x + s.w - rw;
        }
        // Dépassement en haut : autorisé seulement si un écran est présent en haut
        if ry < s.y && !is_point_on_any_screen(cx, ry, screens) {
            clamped_y = s.y;
        }
        // Dépassement en bas : autorisé seulement si un écran est présent en bas
        if ry + rh > (s.y + s.h) && !is_point_on_any_screen(cx, ry + rh, screens) {
            clamped_y = s.y + s.h - rh;
        }

        return (clamped_x, clamped_y);
    }

    // 2. Si le centre est hors de tous les écrans (ex: dans un vide ou au-delà de la zone visible),
    // on le projette sur l'écran le plus proche.
    if let Some(s) = closest_screen(cx, cy, screens) {
        s.clamp_rect(rx, ry, rw, rh)
    } else {
        (rx, ry)
    }
}

/// Téléporte un sprite depuis le bord d'un écran source vers le bord d'entrée
/// correspondant d'un écran cible (utilisé lorsque les écrans ne sont pas adjacents ou décalés).
pub fn teleport_between_screens(
    from: &ScreenRect,
    to: &ScreenRect,
    cur_x: f64,
    cur_y: f64,
    sprite: f64,
) -> (f64, f64) {
    let from_cx = from.x + from.w / 2.0;
    let from_cy = from.y + from.h / 2.0;
    let to_cx = to.x + to.w / 2.0;
    let to_cy = to.y + to.h / 2.0;

    let diff_x = to_cx - from_cx;
    let diff_y = to_cy - from_cy;

    let new_x;
    let new_y;

    if diff_x.abs() >= diff_y.abs() {
        // Déplacement horizontal prédominant
        if diff_x > 0.0 {
            // L'écran cible est à droite -> on entre par son bord gauche
            new_x = to.x + 2.0;
        } else {
            // L'écran cible est à gauche -> on entre par son bord droit
            new_x = (to.x + to.w - sprite - 2.0).max(to.x);
        }
        // Conservation de la hauteur relative (Y proportionnel)
        let rel_y = if from.h > sprite {
            (cur_y - from.y) / (from.h - sprite)
        } else {
            0.5
        };
        new_y = to.y + rel_y.clamp(0.0, 1.0) * (to.h - sprite).max(0.0);
    } else {
        // Déplacement vertical prédominant
        if diff_y > 0.0 {
            // L'écran cible est en bas -> on entre par son bord haut
            new_y = to.y + 2.0;
        } else {
            // L'écran cible est en haut -> on entre par son bord bas
            new_y = (to.y + to.h - sprite - 2.0).max(to.y);
        }
        // Conservation de la position horizontale relative (X proportionnel)
        let rel_x = if from.w > sprite {
            (cur_x - from.x) / (from.w - sprite)
        } else {
            0.5
        };
        new_x = to.x + rel_x.clamp(0.0, 1.0) * (to.w - sprite).max(0.0);
    }

    (new_x, new_y)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonitorMapping {
    pub phys_x: f64,
    pub phys_y: f64,
    pub phys_w: f64,
    pub phys_h: f64,
    pub log_x: f64,
    pub log_y: f64,
    pub sf: f64,
}

impl MonitorMapping {
    pub fn new(phys_x: f64, phys_y: f64, phys_w: f64, phys_h: f64, log_x: f64, log_y: f64, sf: f64) -> Self {
        MonitorMapping {
            phys_x,
            phys_y,
            phys_w,
            phys_h,
            log_x,
            log_y,
            sf: if sf > 0.0 { sf } else { 1.0 },
        }
    }

    pub fn to_logical(&self, cx: f64, cy: f64) -> (f64, f64) {
        let local_x = (cx - self.phys_x) / self.sf;
        let local_y = (cy - self.phys_y) / self.sf;
        (self.log_x + local_x, self.log_y + local_y)
    }

    pub fn contains_phys(&self, cx: f64, cy: f64) -> bool {
        cx >= self.phys_x && cx <= (self.phys_x + self.phys_w)
            && cy >= self.phys_y && cy <= (self.phys_y + self.phys_h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_rect_clamp() {
        let s = ScreenRect::new(0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(s.clamp_rect(-50.0, -50.0, 64.0, 64.0), (0.0, 0.0));
        assert_eq!(s.clamp_rect(2000.0, 2000.0, 64.0, 64.0), (1920.0 - 64.0, 1080.0 - 64.0));
    }

    #[test]
    fn test_monitor_mapping_hidpi_200() {
        // Laptop tactile 1920x1080 à 200% (échelle 2.0)
        // Logique GTK : 960x540
        let m = MonitorMapping::new(0.0, 0.0, 1920.0, 1080.0, 0.0, 0.0, 2.0);
        assert!(m.contains_phys(960.0, 540.0));
        let (lx, ly) = m.to_logical(960.0, 540.0);
        assert!((lx - 480.0).abs() < 1e-6);
        assert!((ly - 270.0).abs() < 1e-6);

        let (lx, ly) = m.to_logical(1920.0, 1080.0);
        assert!((lx - 960.0).abs() < 1e-6);
        assert!((ly - 540.0).abs() < 1e-6);
    }

    #[test]
    fn test_clamp_to_screens_adjacent_crossing() {
        let s1 = ScreenRect::new(0.0, 0.0, 1920.0, 1080.0);
        let s2 = ScreenRect::new(1920.0, 0.0, 1920.0, 1080.0);
        let screens = vec![s1, s2];

        // Sprite à cheval sur la frontière (x=1910, w=32 -> 1910 à 1942)
        // Le centre est à 1926 (sur s2). Le côté gauche (1910) est sur s1.
        let (cx, cy) = clamp_to_screens(1910.0, 500.0, 32.0, 32.0, &screens);
        assert_eq!(cx, 1910.0); // Non bloqué car s1 est adjacent !
        assert_eq!(cy, 500.0);

        // Mur extérieur gauche de s1 -> bloqué à 0
        let (cx, _) = clamp_to_screens(-20.0, 500.0, 32.0, 32.0, &screens);
        assert_eq!(cx, 0.0);

        // Mur extérieur droit de s2 -> bloqué à 3840 - 32 = 3808
        let (cx, _) = clamp_to_screens(3850.0, 500.0, 32.0, 32.0, &screens);
        assert_eq!(cx, 3840.0 - 32.0);
    }

    #[test]
    fn test_teleport_between_screens() {
        // Écran 1 (0..1920) et Écran 2 (3000..4920) séparés par un vide de 1080px
        let s1 = ScreenRect::new(0.0, 0.0, 1920.0, 1080.0);
        let s2 = ScreenRect::new(3000.0, 0.0, 1920.0, 1080.0);

        // Téléportation vers s2 (vers la droite)
        let (tx, ty) = teleport_between_screens(&s1, &s2, 1888.0, 540.0, 32.0);
        assert_eq!(tx, 3000.0 + 2.0); // Entre par le bord gauche de s2
        assert!((ty - 540.0).abs() <= 2.0);

        // Téléportation vers s1 (vers la gauche)
        let (tx, ty) = teleport_between_screens(&s2, &s1, 3000.0, 540.0, 32.0);
        assert_eq!(tx, 1920.0 - 32.0 - 2.0); // Entre par le bord droit de s1
        assert!((ty - 540.0).abs() <= 2.0);
    }
}
