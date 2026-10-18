//! Le panier douillet pour chat — un endroit confortable où le chat aime se reposer.
//!
//! Peut être placé n'importe où sur l'écran, déplacé à la souris (drag & drop),
//! et attire le chat pour ses siestes en mode Autonome et Sommeil.

use crate::util::{clamp_to_screens, ScreenRect};

const FRAMES: usize = 6;

pub struct Basket {
    pub x: f64,
    pub y: f64,
    pub active: bool,
    pub size: f64,
    pub is_dragged: bool,
    pub loop_counter: usize,
    screens: Vec<ScreenRect>,
}

impl Basket {
    pub fn new(x: f64, y: f64, size: f64, active: bool, screens: Vec<ScreenRect>) -> Self {
        let mut b = Basket {
            x,
            y,
            active,
            size,
            is_dragged: false,
            loop_counter: 0,
            screens,
        };
        if b.active {
            b.ensure_on_screen();
        }
        b
    }

    /// Met à jour les écrans physiques connus pour le confinement.
    pub fn set_screens(&mut self, screens: Vec<ScreenRect>) {
        self.screens = screens;
        if self.active {
            self.ensure_on_screen();
        }
    }

    /// Met à jour la taille du sprite (changement d'échelle).
    pub fn set_size(&mut self, size: f64) {
        self.size = size;
        if self.active {
            self.ensure_on_screen();
        }
    }

    /// S'assure que le panier est bien positionné sur un écran visible.
    pub fn ensure_on_screen(&mut self) {
        if !self.screens.is_empty() {
            // Si le panier n'a pas encore de position définie (< 0), le placer au centre du premier écran
            if self.x < 0.0 || self.y < 0.0 {
                let s0 = &self.screens[0];
                self.x = s0.x + (s0.w - self.size) / 2.0;
                self.y = s0.y + (s0.h - self.size) / 2.0;
            } else {
                let (cx, cy) = clamp_to_screens(self.x, self.y, self.size, self.size, &self.screens);
                self.x = cx;
                self.y = cy;
            }
        }
    }

    /// Place le panier au centre de l'écran principal ou actif.
    pub fn center_on_screen(&mut self) {
        if !self.screens.is_empty() {
            let s0 = &self.screens[0];
            self.x = s0.x + (s0.w - self.size) / 2.0;
            self.y = s0.y + (s0.h - self.size) / 2.0;
        }
    }

    /// Centre du panier en coordonnées globales.
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.size / 2.0, self.y + self.size / 2.0)
    }

    /// Hit-test pour attraper le panier au clic ou au toucher.
    pub fn hit_test(&self, gx: f64, gy: f64) -> bool {
        if !self.active {
            return false;
        }
        let pad = 24.0;
        gx >= (self.x - pad)
            && gx <= (self.x + self.size + pad)
            && gy >= (self.y - pad)
            && gy <= (self.y + self.size + pad)
    }

    /// Début du glisser-déposer du panier.
    pub fn start_drag(&mut self) {
        self.is_dragged = true;
    }

    /// Déplacement du panier vers une nouvelle position globale.
    pub fn drag_to(&mut self, target_x: f64, target_y: f64) {
        if !self.screens.is_empty() {
            let (cx, cy) = clamp_to_screens(target_x, target_y, self.size, self.size, &self.screens);
            self.x = cx;
            self.y = cy;
        } else {
            self.x = target_x;
            self.y = target_y;
        }
    }

    /// Fin du glisser-déposer.
    pub fn end_drag(&mut self) {
        self.is_dragged = false;
        self.ensure_on_screen();
    }

    /// Frame courante pour l'animation (douce).
    pub fn current_frame(&self) -> (i32, i32) {
        let frame_idx = (self.loop_counter / 6) % FRAMES;
        (frame_idx as i32 * 32, 0)
    }

    /// Avance d'un tick d'horloge.
    pub fn tick(&mut self) {
        self.loop_counter = self.loop_counter.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basket_clamping_and_hit_test() {
        let screens = vec![ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        }];
        let mut basket = Basket::new(100.0, 100.0, 48.0, true, screens);
        assert!(basket.hit_test(100.0, 100.0));
        assert!(basket.hit_test(100.0 + 48.0 + 20.0, 100.0));
        assert!(!basket.hit_test(500.0, 500.0));

        basket.drag_to(3000.0, 3000.0);
        assert!(basket.x <= 1920.0 - 48.0);
        assert!(basket.y <= 1080.0 - 48.0);
    }

    #[test]
    fn test_basket_frames_and_tick() {
        let mut basket = Basket::new(0.0, 0.0, 48.0, true, vec![]);
        assert_eq!(basket.current_frame(), (0, 0));
        for _ in 0..6 {
            basket.tick();
        }
        assert_eq!(basket.current_frame(), (32, 0));
    }
}
