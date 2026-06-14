//! Neko desktop — overlay Wayland (layer-shell) qui anime un chat à l'écran.
//!
//! Architecture :
//!   - UN overlay layer-shell PAR moniteur (always-on-top, click-through). Le
//!     chat évolue dans un espace de coordonnées GLOBAL (union des moniteurs) ;
//!     chaque overlay le dessine décalé de l'offset de son moniteur.
//!     → contourne l'interdiction Wayland de repositionner sa propre fenêtre.
//!   - Un thread tokio écoute Twitch Heat et pousse une cible dans un état partagé.
//!   - Un tick GTK (~10 fps) fait avancer la logique du chat et redessine.
//!   - Une icône de barre système (ksni) pour quitter.

mod pet;
mod tray;
mod twitch;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gtk::gdk::prelude::*;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use pet::{Pet, Sprites, State};
use twitch::Target;

const APP_ID: &str = "com.warmadon.neko_rust";
/// Skin par défaut (fichiers `assets/pets/<skin>.png` et `<skin>.json`).
const DEFAULT_SKIN: &str = "neko";
/// Facteur d'agrandissement du sprite (32px natif → trop petit en hiDPI).
const SCALE: f64 = 1.5;

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let display = gtk::gdk::Display::default().expect("aucun display GDK");

    // ---- Espace global = union de tous les moniteurs ----
    let monitors = monitors(&display);
    let (orig_x, orig_y, total_w, total_h) = union_bounds(&monitors);

    // ---- Fond de fenêtre transparent (sinon l'overlay masque tout) ----
    let provider = gtk::CssProvider::new();
    provider.load_from_data("window, window.background { background: transparent; }");
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // ---- État partagé Twitch <-> GTK (coordonnées dans l'espace global) ----
    let shared = Arc::new(Mutex::new(Target {
        x: total_w / 2.0,
        y: total_h / 2.0,
        active: false,
    }));

    // ---- Skin : png + mapping JSON (var d'env NEKO_SKIN, défaut "neko") ----
    let skin = std::env::var("NEKO_SKIN").unwrap_or_else(|_| DEFAULT_SKIN.to_string());
    let sprite_path = format!("assets/pets/{skin}.png");
    let sprites = load_mapping(&skin);

    // ---- État du chat (thread GTK uniquement) ----
    let sprite_size = pet::TILE as f64 * SCALE;
    let pet = Rc::new(RefCell::new(Pet::new(total_w, total_h, sprite_size, sprites)));
    let pixbuf = load_sprite(&sprite_path);
    if pixbuf.is_none() {
        eprintln!("[neko] sprite introuvable : {sprite_path} (lance depuis le dossier du projet)");
    }

    // ---- Tray icon ----
    tray::spawn(pixbuf.as_ref());

    // ---- Un overlay layer-shell par moniteur ----
    let mut areas = Vec::with_capacity(monitors.len());
    for monitor in &monitors {
        let geo = monitor.geometry();
        let off_x = (geo.x() - orig_x) as f64; // offset du moniteur dans l'union
        let off_y = (geo.y() - orig_y) as f64;

        let window = ApplicationWindow::builder().application(app).build();
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_monitor(monitor);
        for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
            window.set_anchor(edge, true);
        }
        window.set_exclusive_zone(-1);
        window.set_keyboard_mode(KeyboardMode::None);

        let area = DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        {
            let pet = pet.clone();
            let pixbuf = pixbuf.clone();
            area.set_draw_func(move |_a, cr, _w, _h| {
                let Some(pb) = &pixbuf else { return };
                let p = pet.borrow();
                let (fx, fy) = p.current_frame();
                let sub = pb.new_subpixbuf(fx, fy, pet::TILE, pet::TILE);
                let _ = cr.save();
                // position globale du chat → coordonnées locales de ce moniteur
                cr.translate(p.x - off_x, p.y - off_y);
                cr.scale(SCALE, SCALE);
                cr.set_source_pixbuf(&sub, 0.0, 0.0);
                let _ = cr.paint();
                let _ = cr.restore();
            });
        }
        window.set_child(Some(&area));

        // Click-through : input region vide → tous les clics traversent.
        window.connect_map(|w| {
            if let Some(surface) = w.surface() {
                surface.set_input_region(&gtk::cairo::Region::create());
            }
        });

        window.present();
        areas.push(area);
    }

    // ---- Tick d'animation ~10 fps (redessine tous les overlays) ----
    {
        let pet = pet.clone();
        let shared = shared.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let (tx, ty, active) = {
                let t = shared.lock().unwrap();
                (t.x, t.y, t.active)
            };
            let state = if active { State::Chase } else { State::Autonom };
            pet.borrow_mut().update((tx, ty), state);
            for area in &areas {
                area.queue_draw();
            }
            glib::ControlFlow::Continue
        });
    }

    // ---- Thread Twitch Heat (optionnel : variable d'env NEKO_TWITCH_CHANNEL) ----
    if let Ok(channel) = std::env::var("NEKO_TWITCH_CHANNEL") {
        if !channel.is_empty() {
            let shared = shared.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("runtime tokio");
                rt.block_on(twitch::run(channel, shared, total_w, total_h));
            });
        }
    }
}

/// Charge le mapping d'un skin depuis `assets/pets/<skin>.json`. Retombe sur le
/// mapping neko embarqué si le fichier est absent ou invalide.
fn load_mapping(skin: &str) -> Sprites {
    let path = format!("assets/pets/{skin}.json");
    match std::fs::read_to_string(&path) {
        Ok(json) => match Sprites::from_json(&json) {
            Ok(s) => return s,
            Err(e) => eprintln!("[neko] {path} invalide ({e}), mapping par défaut utilisé"),
        },
        Err(_) => {
            if skin != DEFAULT_SKIN {
                eprintln!("[neko] {path} introuvable, mapping par défaut utilisé");
            }
        }
    }
    Sprites::default()
}

/// Charge le sprite-sheet et rend transparente la couleur de fond, définie comme
/// celle du pixel (0,0) (les sheets oneko ont un fond plein uni à retirer).
fn load_sprite(path: &str) -> Option<Pixbuf> {
    let pixbuf = Pixbuf::from_file(path).ok()?;
    // Lecture du pixel (0,0) : 1ers octets du buffer (RGB ou RGBA).
    let (r, g, b) = {
        let pixels = unsafe { pixbuf.pixels() };
        if pixels.len() < 3 {
            return Some(pixbuf);
        }
        (pixels[0], pixels[1], pixels[2])
    };
    // add_alpha renvoie un nouveau pixbuf RGBA où chaque pixel == (r,g,b) devient
    // totalement transparent.
    pixbuf.add_alpha(true, r, g, b).ok()
}

/// Liste des moniteurs connectés.
fn monitors(display: &gtk::gdk::Display) -> Vec<gtk::gdk::Monitor> {
    let model = display.monitors();
    (0..model.n_items())
        .filter_map(|i| model.item(i).and_downcast::<gtk::gdk::Monitor>())
        .collect()
}

/// Boîte englobante de tous les moniteurs : (origine_x, origine_y, largeur, hauteur).
/// Fallback 1920x1080 si aucun moniteur n'est rapporté.
fn union_bounds(monitors: &[gtk::gdk::Monitor]) -> (i32, i32, f64, f64) {
    if monitors.is_empty() {
        return (0, 0, 1920.0, 1080.0);
    }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for m in monitors {
        let g = m.geometry();
        min_x = min_x.min(g.x());
        min_y = min_y.min(g.y());
        max_x = max_x.max(g.x() + g.width());
        max_y = max_y.max(g.y() + g.height());
    }
    (min_x, min_y, (max_x - min_x) as f64, (max_y - min_y) as f64)
}
