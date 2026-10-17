//! La pelote de laine — le jouet que le chat poursuit (porté depuis Toy.ts).
//!
//! Elle rebondit en diagonale sur les bords de la zone globale et des moniteurs réels.
//! Quand le chat l'atteint, elle est attrapée et se cache ; `main` la fait réapparaître après
//! une pause de repos du chat.

use crate::util::{rand_unit, random_point_on_screens, clamp_to_screens, ScreenRect};

/// Sheet `wool.png` : 6 frames de 32×32 en ligne (x = frame*32, y = 0).
const FRAMES: usize = 6;
const V_STEP: f64 = 16.0; // pas vertical par tick
const CATCH: f64 = 24.0; // distance d'attrapage (coins haut-gauche)

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub struct Toy {
    pub x: f64,
    pub y: f64,
    pub active: bool,
    bounds_w: f64,
    bounds_h: f64,
    size: f64, // taille à l'écran (pour borner comme le chat)
    dir: Dir,
    curvature: f64,
    loop_counter: usize,
    screens: Vec<ScreenRect>,
}

impl Toy {
    pub fn new(bounds_w: f64, bounds_h: f64, size: f64, screens: Vec<ScreenRect>) -> Self {
        let mut toy = Toy {
            x: 0.0,
            y: 0.0,
            active: false,
            bounds_w,
            bounds_h,
            size,
            dir: Dir::TopLeft,
            curvature: 14.0,
            loop_counter: 0,
            screens,
        };
        toy.spawn();
        toy
    }

    /// Met à jour la liste des écrans physiques actifs.
    pub fn set_screens(&mut self, screens: Vec<ScreenRect>) {
        self.screens = screens;
    }

    /// Met à jour la taille à l'écran (changement d'échelle à chaud).
    pub fn set_size(&mut self, size: f64) {
        self.size = size;
    }

    fn max_x(&self) -> f64 {
        (self.bounds_w - self.size).max(0.0)
    }
    fn max_y(&self) -> f64 {
        (self.bounds_h - self.size).max(0.0)
    }

    /// (Ré)apparition à une position aléatoire garantie SUR un écran réel visible.
    pub fn spawn(&mut self) {
        if !self.screens.is_empty() {
            let (rx, ry) = random_point_on_screens(&self.screens, self.size, self.size);
            self.x = rx;
            self.y = ry;
        } else {
            self.x = rand_unit() * self.max_x();
            self.y = rand_unit() * self.max_y();
        }

        self.dir = match (rand_unit() * 4.0) as u32 {
            0 => Dir::TopLeft,
            1 => Dir::TopRight,
            2 => Dir::BottomLeft,
            _ => Dir::BottomRight,
        };
        self.curvature = 14.0;
        self.active = true;
    }

    pub fn hide(&mut self) {
        self.active = false;
    }

    /// Frame courante en pixels dans `wool.png`.
    pub fn current_frame(&self) -> (i32, i32) {
        ((self.loop_counter % FRAMES) as i32 * 32, 0)
    }

    /// Centre de la pelote à l'écran.
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.size / 2.0, self.y + self.size / 2.0)
    }

    /// Avance d'un tick. `(cat_cx, cat_cy)` = **centre** du chat.
    /// Renvoie `true` si le chat vient d'attraper la pelote (centre à centre).
    pub fn update(&mut self, cat_cx: f64, cat_cy: f64) -> bool {
        if !self.active {
            return false;
        }

        let (tcx, tcy) = self.center();
        if (cat_cx - tcx).abs() <= CATCH && (cat_cy - tcy).abs() <= CATCH {
            self.hide();
            return true;
        }

        // Prochaine position proposée
        let (dx, dy) = match self.dir {
            Dir::TopLeft => (-self.curvature, -V_STEP),
            Dir::TopRight => (self.curvature, -V_STEP),
            Dir::BottomLeft => (-self.curvature, V_STEP),
            Dir::BottomRight => (self.curvature, V_STEP),
        };

        let cand_x = self.x + dx;
        let cand_y = self.y + dy;

        let (clamped_x, clamped_y) = if !self.screens.is_empty() {
            clamp_to_screens(cand_x, cand_y, self.size, self.size, &self.screens)
        } else {
            (cand_x.clamp(0.0, self.max_x()), cand_y.clamp(0.0, self.max_y()))
        };

        // Si le clamp a bloqué en X, on rebondit horizontalement
        if (clamped_x - cand_x).abs() > 0.5 {
            self.dir = match self.dir {
                Dir::TopLeft => Dir::TopRight,
                Dir::TopRight => Dir::TopLeft,
                Dir::BottomLeft => Dir::BottomRight,
                Dir::BottomRight => Dir::BottomLeft,
            };
        }

        // Si le clamp a bloqué en Y, on rebondit verticalement
        if (clamped_y - cand_y).abs() > 0.5 {
            self.dir = match self.dir {
                Dir::TopLeft => Dir::BottomLeft,
                Dir::TopRight => Dir::BottomRight,
                Dir::BottomLeft => Dir::TopLeft,
                Dir::BottomRight => Dir::TopRight,
            };
        }

        self.x = clamped_x;
        self.y = clamped_y;

        if rand_unit() < 0.1 {
            self.curvature = 14.0 + (rand_unit() * 4.0).floor();
        }

        self.loop_counter = self.loop_counter.wrapping_add(1);
        false
    }
}
