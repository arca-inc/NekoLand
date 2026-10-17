//! Logique du chat, portée depuis Pet.ts.
//!
//! Différence clé avec la version web/Tauri : ici on NE déplace PAS de fenêtre.
//! La fenêtre est un overlay fixe plein écran ; `(x, y)` est simplement la
//! position où l'on dessine le sprite. C'est ce qui rend le tout compatible
//! Wayland (une fenêtre ne peut pas se repositionner elle-même sous Wayland).
//!
//! Le mapping cellule→animation n'est plus codé en dur : il est chargé depuis un
//! JSON par skin (voir [`Sprites`]). Le mapping par défaut (neko) est embarqué.

use std::collections::HashMap;
use std::f64::consts::PI;
use crate::util::{clamp_to_screens, ScreenRect};

/// Pas de la grille du sprite-sheet oneko en pixels (tuile 32 px + 1 px de marge).
pub const GRID_STRIDE: i32 = 33;
/// Taille d'une tuile.
pub const TILE: i32 = 32;

const STEP: f64 = 16.0; // distance parcourue par tick

/// Mapping neko par défaut (fourni via tools/sprite_mapper.html), au même format
/// `{ "clip": [[col, row], ...] }` que celui exporté par l'outil.
pub const DEFAULT_MAPPING: &str = include_str!("../assets/pets/neko.json");

// Noms canoniques des clips (doivent matcher les clés du JSON).
const IDLE: &str = "idle";
const ALERT: &str = "alert";
const TIRED: &str = "tired";
const SLEEPING: &str = "sleeping";
const SCRATCH_SELF: &str = "scratchSelf";
const SCRATCH_WALL_N: &str = "scratchWallN";
const SCRATCH_WALL_S: &str = "scratchWallS";
const SCRATCH_WALL_W: &str = "scratchWallW";
const SCRATCH_WALL_E: &str = "scratchWallE";

/// Jeu de sprites d'un skin : clip nommé → liste de frames en coord. de grille.
#[derive(Clone)]
pub struct Sprites {
    clips: HashMap<String, Vec<(i32, i32)>>,
}

impl Sprites {
    /// Parse un mapping JSON `{ "clip": [[col,row], ...] }`.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let raw: HashMap<String, Vec<[i32; 2]>> = serde_json::from_str(json)?;
        let clips = raw
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(|a| (a[0], a[1])).collect()))
            .collect();
        Ok(Sprites { clips })
    }

    /// Frames d'un clip (fallback sur IDLE, puis la tuile (0,0) si absent/vide).
    fn frames(&self, clip: &str) -> &[(i32, i32)] {
        if let Some(frames) = self.clips.get(clip).filter(|s| !s.is_empty()) {
            return frames;
        }
        if clip != IDLE {
            if let Some(idle_frames) = self.clips.get(IDLE).filter(|s| !s.is_empty()) {
                return idle_frames;
            }
        }
        &[(0, 0)]
    }
}

impl Default for Sprites {
    fn default() -> Self {
        Self::from_json(DEFAULT_MAPPING).expect("DEFAULT_MAPPING JSON invalide")
    }
}

/// État de comportement. La cible (pelote, clic Twitch, ou sa propre position
/// pour se reposer) est décidée par l'appelant ; ici on ne fait que la chasser.
pub enum State {
    /// Chasse une cible.
    Chase,
    /// En cours de déplacement manuel (drag à la souris).
    Dragged,
}

pub struct Pet {
    pub x: f64,
    pub y: f64,
    bounds_w: f64,
    bounds_h: f64,
    /// Taille du sprite à l'écran en px (pour que le chat entier reste visible).
    sprite: f64,
    sleep_counter: i32,
    loop_counter: usize,
    current: &'static str,
    sprites: Sprites,
    screens: Vec<ScreenRect>,
    pub is_dragged: bool,
}

impl Pet {
    pub fn new(bounds_w: f64, bounds_h: f64, sprite: f64, sprites: Sprites, screens: Vec<ScreenRect>) -> Self {
        let init_x = bounds_w / 2.0;
        let init_y = bounds_h / 2.0;
        let (x, y) = if !screens.is_empty() {
            clamp_to_screens(init_x, init_y, sprite, sprite, &screens)
        } else {
            (init_x, init_y)
        };

        Pet {
            x,
            y,
            bounds_w,
            bounds_h,
            sprite,
            sleep_counter: 0,
            loop_counter: 0,
            current: IDLE,
            sprites,
            screens,
            is_dragged: false,
        }
    }

    /// Met à jour les écrans physiques connus pour le confinement strict.
    pub fn set_screens(&mut self, screens: Vec<ScreenRect>) {
        self.screens = screens;
        self.ensure_on_screen();
    }

    /// Vérifie et garantit que le chat se trouve actuellement sur un écran actif.
    pub fn ensure_on_screen(&mut self) {
        if !self.screens.is_empty() {
            let (cx, cy) = clamp_to_screens(self.x, self.y, self.sprite, self.sprite, &self.screens);
            self.x = cx;
            self.y = cy;
        } else {
            self.x = self.x.clamp(0.0, self.max_x());
            self.y = self.y.clamp(0.0, self.max_y());
        }
    }

    /// Vérifie si un clic / pointeur (gx, gy) touche la hitbox du chat.
    pub fn hit_test(&self, gx: f64, gy: f64) -> bool {
        // Marge de tolérance de 10px autour du sprite pour une saisie tactile/souris agréable
        let pad = 10.0;
        gx >= (self.x - pad)
            && gx <= (self.x + self.sprite + pad)
            && gy >= (self.y - pad)
            && gy <= (self.y + self.sprite + pad)
    }

    /// Début du drag à la souris.
    pub fn start_drag(&mut self) {
        self.is_dragged = true;
        self.sleep_counter = 0;
        self.current = ALERT;
    }

    /// Met à jour la position du chat pendant le drag manuel.
    pub fn drag_to(&mut self, new_x: f64, new_y: f64) {
        if !self.screens.is_empty() {
            let (cx, cy) = clamp_to_screens(new_x, new_y, self.sprite, self.sprite, &self.screens);
            self.x = cx;
            self.y = cy;
        } else {
            self.x = new_x.clamp(0.0, self.max_x());
            self.y = new_y.clamp(0.0, self.max_y());
        }
        self.sleep_counter = 0;
        self.current = ALERT;
    }

    /// Fin du drag à la souris : le chat atterrit au sol.
    pub fn end_drag(&mut self) {
        self.is_dragged = false;
        self.sleep_counter = 0;
        self.current = ALERT;
    }

    /// Borne supérieure atteignable pour le coin haut-gauche du sprite.
    fn max_x(&self) -> f64 {
        (self.bounds_w - self.sprite).max(0.0)
    }
    fn max_y(&self) -> f64 {
        (self.bounds_h - self.sprite).max(0.0)
    }

    /// Remplace le jeu de sprites (changement de skin à chaud).
    pub fn set_sprites(&mut self, sprites: Sprites) {
        self.sprites = sprites;
        self.loop_counter = 0;
    }

    /// Met à jour la taille du sprite à l'écran (changement d'échelle à chaud).
    pub fn set_sprite_size(&mut self, sprite: f64) {
        self.sprite = sprite;
        self.ensure_on_screen();
    }

    /// Nom de l'animation en cours (pour le debug visuel).
    pub fn current_clip(&self) -> &'static str {
        self.current
    }

    /// Vrai si le chat dort profondément (sleep_counter > 20).
    pub fn is_sleeping(&self) -> bool {
        self.sleep_counter > 20
    }

    /// Réveille le chat en sursaut : remet le compteur à zéro → séquence
    /// alert → scratchSelf → tired → sleeping recommence depuis le début.
    pub fn startle(&mut self) {
        if self.sleep_counter > 10 {
            self.sleep_counter = 0;
        }
    }

    /// Frame courante en **pixels** dans le sprite-sheet (coin haut-gauche de la
    /// tuile 32×32 à blitter).
    pub fn current_frame(&self) -> (i32, i32) {
        let frames = self.sprites.frames(self.current);
        let (col, row) = frames[self.loop_counter % frames.len()];
        (col * GRID_STRIDE, row * GRID_STRIDE)
    }

    /// Avance d'un tick. `target` = cible à chasser en pixels écran.
    pub fn update(&mut self, target: (f64, f64), state: State) {
        if self.is_dragged {
            // Pendant le drag, la position est pilotée manuellement
            let len = self.sprites.frames(self.current).len().max(1);
            self.loop_counter = (self.loop_counter + 1) % len;
            return;
        }

        match state {
            State::Chase => self.move_toward(target.0, target.1),
            State::Dragged => {}
        }

        let len = self.sprites.frames(self.current).len().max(1);
        self.loop_counter += 1;
        if self.loop_counter >= len {
            self.loop_counter = 0;
        }
    }

    /// `target` est la position visée par le **centre** du chat (pas son coin).
    fn move_toward(&mut self, target_x: f64, target_y: f64) {
        let half = self.sprite / 2.0;
        let cx = self.x + half;
        let cy = self.y + half;
        let delta_x = target_x - cx;
        let delta_y = cy - target_y; // repère y inversé, comme dans Pet.ts
        let theta = delta_y.atan2(delta_x);
        let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();

        if distance <= 32.0 {
            // Arrivé : s'il est au bord de l'écran, gratte le mur, sinon s'installe
            let at_left = self.x <= 4.0;
            let at_top = self.y <= 4.0;
            let at_right = self.x >= self.max_x() - 4.0;
            let at_bottom = self.y >= self.max_y() - 4.0;

            if at_left {
                self.current = SCRATCH_WALL_W;
            } else if at_right {
                self.current = SCRATCH_WALL_E;
            } else if at_top {
                self.current = SCRATCH_WALL_N;
            } else if at_bottom {
                self.current = SCRATCH_WALL_S;
            } else {
                self.sleep_counter += 1;
                self.current = match self.sleep_counter {
                    0..=2 => ALERT,
                    3..=7 => SCRATCH_SELF,
                    8..=20 => TIRED,
                    _ => SLEEPING,
                };
            }
        } else {
            self.sleep_counter = 0;
            let cand_x = self.x + theta.cos() * STEP;
            let cand_y = self.y - theta.sin() * STEP;

            if !self.screens.is_empty() {
                let (cx, cy) = clamp_to_screens(cand_x, cand_y, self.sprite, self.sprite, &self.screens);
                self.x = cx;
                self.y = cy;
            } else {
                self.x = cand_x.clamp(0.0, self.max_x());
                self.y = cand_y.clamp(0.0, self.max_y());
            }

            // theta : 0 = droite (E), π/2 = haut (N), repère y inversé.
            self.current = match theta {
                t if t >= 7.0 * PI / 8.0 || t <= -7.0 * PI / 8.0 => "W",
                t if (-PI / 8.0..=PI / 8.0).contains(&t) => "E",
                t if (3.0 * PI / 8.0..=5.0 * PI / 8.0).contains(&t) => "N",
                t if (-5.0 * PI / 8.0..=-3.0 * PI / 8.0).contains(&t) => "S",
                t if t > 5.0 * PI / 8.0 && t < 7.0 * PI / 8.0 => "NW",
                t if t > -7.0 * PI / 8.0 && t < -5.0 * PI / 8.0 => "SW",
                t if t > PI / 8.0 && t < 3.0 * PI / 8.0 => "NE",
                t if t > -3.0 * PI / 8.0 && t < -PI / 8.0 => "SE",
                _ => self.current,
            };
        }
    }
}