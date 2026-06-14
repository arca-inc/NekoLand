//! La pelote de laine — le jouet que le chat poursuit (porté depuis Toy.ts).
//!
//! Elle rebondit en diagonale sur les bords de la zone globale. Quand le chat
//! l'atteint, elle est attrapée et se cache ; `main` la fait réapparaître après
//! une pause de repos du chat.

use crate::util::rand_unit;

/// Sheet `wool.png` : 6 frames de 32×32 en ligne (x = frame*32, y = 0).
const FRAMES: usize = 6;
const V_STEP: f64 = 16.0; // pas vertical par tick
const CATCH: f64 = 24.0; // distance d'attrapage (coins haut-gauche)

#[derive(Clone, Copy)]
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
}

impl Toy {
    pub fn new(bounds_w: f64, bounds_h: f64, size: f64) -> Self {
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
        };
        toy.spawn();
        toy
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

    /// (Ré)apparition à une position aléatoire, direction aléatoire.
    pub fn spawn(&mut self) {
        self.x = rand_unit() * self.max_x();
        self.y = rand_unit() * self.max_y();
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

    /// Avance d'un tick. `(pet_x, pet_y)` = coin haut-gauche du chat.
    /// Renvoie `true` si le chat vient d'attraper la pelote.
    pub fn update(&mut self, pet_x: f64, pet_y: f64) -> bool {
        if !self.active {
            return false;
        }

        if (pet_x - self.x).abs() <= CATCH && (pet_y - self.y).abs() <= CATCH {
            self.hide();
            return true;
        }

        let (mx, my) = (self.max_x(), self.max_y());
        match self.dir {
            Dir::TopLeft => {
                if self.x <= 0.0 {
                    self.dir = Dir::TopRight;
                } else if self.y <= 0.0 {
                    self.dir = Dir::BottomLeft;
                } else {
                    self.x -= self.curvature;
                    self.y -= V_STEP;
                }
            }
            Dir::TopRight => {
                if self.x >= mx {
                    self.dir = Dir::TopLeft;
                } else if self.y <= 0.0 {
                    self.dir = Dir::BottomRight;
                } else {
                    self.x += self.curvature;
                    self.y -= V_STEP;
                }
            }
            Dir::BottomLeft => {
                if self.x <= 0.0 {
                    self.dir = Dir::BottomRight;
                } else if self.y >= my {
                    self.dir = Dir::TopLeft;
                } else {
                    self.x -= self.curvature;
                    self.y += V_STEP;
                }
            }
            Dir::BottomRight => {
                if self.x >= mx {
                    self.dir = Dir::BottomLeft;
                } else if self.y >= my {
                    self.dir = Dir::TopRight;
                } else {
                    self.x += self.curvature;
                    self.y += V_STEP;
                }
            }
        }
        self.x = self.x.clamp(0.0, mx);
        self.y = self.y.clamp(0.0, my);

        if rand_unit() < 0.1 {
            self.curvature = 14.0 + (rand_unit() * 4.0).floor();
        }

        self.loop_counter = self.loop_counter.wrapping_add(1);
        false
    }
}
