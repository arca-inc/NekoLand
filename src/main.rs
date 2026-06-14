//! Neko desktop — overlay Wayland (layer-shell) qui anime un chat à l'écran.
//!
//! Architecture :
//!   - UN overlay layer-shell PAR moniteur (always-on-top, click-through). Le
//!     chat évolue dans un espace de coordonnées GLOBAL (union des moniteurs) ;
//!     chaque overlay le dessine décalé de l'offset de son moniteur.
//!     → contourne l'interdiction Wayland de repositionner sa propre fenêtre.
//!   - Le chat poursuit une pelote (toy) ; clics Twitch Heat prioritaires.
//!   - Réglages persistants (config.json) ; icône de barre système (ksni).

mod config;
mod pet;
mod toy;
mod tray;
mod twitch;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
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
use toy::Toy;
use twitch::Target;

const APP_ID: &str = "com.warmadon.neko_rust";
/// Skin sans warning si son `.json` manque (il partage le mapping par défaut).
const DEFAULT_SKIN: &str = "neko";
/// Durée de repos du chat après avoir attrapé la pelote (ticks ~10/s).
const REST_TICKS: i32 = 25;

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let display = gtk::gdk::Display::default().expect("aucun display GDK");
    let cfg = config::load();
    let assets = assets_dir();

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

    // ---- Ressources partagées (cellules → reload à chaud possible) ----
    let scale = Rc::new(Cell::new(cfg.scale));
    let sprite_size = pet::TILE as f64 * cfg.scale;

    let sprites = load_mapping(&assets, &cfg.skin);
    let pet = Rc::new(RefCell::new(Pet::new(total_w, total_h, sprite_size, sprites)));
    let pet_pix = Rc::new(RefCell::new(load_sprite(&pets_png(&assets, &cfg.skin))));

    let toy = Rc::new(RefCell::new(Toy::new(total_w, total_h, sprite_size)));
    let toy_pix = Rc::new(RefCell::new(load_sprite(&toys_png(&assets, &cfg.toy))));

    // ---- Tray icon ----
    tray::spawn(pet_pix.borrow().as_ref());

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
            let pet_pix = pet_pix.clone();
            let toy = toy.clone();
            let toy_pix = toy_pix.clone();
            let scale = scale.clone();
            area.set_draw_func(move |_a, cr, _w, _h| {
                let sc = scale.get();
                // Helper : blit d'une tuile 32×32 à une position globale.
                let blit = |pb: &Pixbuf, fx: i32, fy: i32, gx: f64, gy: f64| {
                    let sub = pb.new_subpixbuf(fx, fy, pet::TILE, pet::TILE);
                    let _ = cr.save();
                    cr.translate(gx - off_x, gy - off_y); // global → local moniteur
                    cr.scale(sc, sc);
                    cr.set_source_pixbuf(&sub, 0.0, 0.0);
                    let _ = cr.paint();
                    let _ = cr.restore();
                };

                // Pelote (sous le chat).
                if let Some(tp) = toy_pix.borrow().as_ref() {
                    let t = toy.borrow();
                    if t.active {
                        let (fx, fy) = t.current_frame();
                        blit(tp, fx, fy, t.x, t.y);
                    }
                }
                // Chat.
                if let Some(pb) = pet_pix.borrow().as_ref() {
                    let p = pet.borrow();
                    let (fx, fy) = p.current_frame();
                    blit(pb, fx, fy, p.x, p.y);
                }
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

    // ---- Tick d'animation ~10 fps (logique de jeu + redessine les overlays) ----
    {
        let pet = pet.clone();
        let toy = toy.clone();
        let shared = shared.clone();
        let mut rest: i32 = 0; // ticks de repos restants après une prise
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let (twx, twy, twitch_active) = {
                let t = shared.lock().unwrap();
                (t.x, t.y, t.active)
            };
            let (px, py) = {
                let p = pet.borrow();
                (p.x, p.y)
            };

            // Cible à poursuivre selon le mode.
            let target = if twitch_active {
                (twx, twy) // priorité aux clics Twitch Heat
            } else {
                let mut t = toy.borrow_mut();
                if t.active {
                    if t.update(px, py) {
                        rest = REST_TICKS; // attrapée → le chat se repose
                    }
                    (t.x, t.y)
                } else if rest > 0 {
                    rest -= 1;
                    (px, py) // immobile : la pose « arrivée » joue le sommeil
                } else {
                    t.spawn(); // nouvelle pelote
                    (t.x, t.y)
                }
            };

            pet.borrow_mut().update(target, State::Chase);
            for area in &areas {
                area.queue_draw();
            }
            glib::ControlFlow::Continue
        });
    }

    // ---- Thread Twitch Heat (si un canal est configuré) ----
    if !cfg.twitch_channel.is_empty() {
        let shared = shared.clone();
        let channel = cfg.twitch_channel.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("runtime tokio");
            rt.block_on(twitch::run(channel, shared, total_w, total_h));
        });
    }
}

/// Répertoire des assets : `$NEKO_ASSETS`, sinon `./assets`, à côté de l'exécutable,
/// `target/<profil>/../../assets` (dev cargo), `~/.local/share/...`, ou `/usr/share`.
fn assets_dir() -> PathBuf {
    if let Ok(d) = std::env::var("NEKO_ASSETS") {
        return PathBuf::from(d);
    }
    let mut candidates: Vec<PathBuf> = vec![PathBuf::from("assets")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("assets"));
            if let Some(up) = dir.parent().and_then(Path::parent) {
                candidates.push(up.join("assets")); // cargo: target/<profil>/exe → ../../assets
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(home).join(".local/share/neko-desktop/assets"));
    }
    candidates.push(PathBuf::from("/usr/share/neko-desktop/assets"));
    candidates
        .into_iter()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from("assets"))
}

fn pets_png(assets: &Path, skin: &str) -> PathBuf {
    assets.join("pets").join(format!("{skin}.png"))
}
fn toys_png(assets: &Path, toy: &str) -> PathBuf {
    assets.join("toys").join(format!("{toy}.png"))
}

/// Charge le mapping d'un skin depuis `<assets>/pets/<skin>.json`. Retombe sur le
/// mapping neko embarqué si le fichier est absent ou invalide.
fn load_mapping(assets: &Path, skin: &str) -> Sprites {
    let path = assets.join("pets").join(format!("{skin}.json"));
    match std::fs::read_to_string(&path) {
        Ok(json) => match Sprites::from_json(&json) {
            Ok(s) => return s,
            Err(e) => eprintln!("[neko] {} invalide ({e}), mapping par défaut", path.display()),
        },
        Err(_) => {
            if skin != DEFAULT_SKIN {
                eprintln!("[neko] {} introuvable, mapping par défaut", path.display());
            }
        }
    }
    Sprites::default()
}

/// Charge un sprite-sheet et rend transparente la couleur de fond — définie comme
/// celle du pixel (0,0) — **uniquement si ce pixel est opaque** (sinon le sheet a
/// déjà son propre alpha et on n'y touche pas).
fn load_sprite(path: &Path) -> Option<Pixbuf> {
    let pixbuf = Pixbuf::from_file(path)
        .map_err(|_| eprintln!("[neko] sprite introuvable : {}", path.display()))
        .ok()?;
    let nch = pixbuf.n_channels() as usize;
    let (r, g, b, a) = {
        let pixels = unsafe { pixbuf.pixels() };
        if pixels.len() < 3 {
            return Some(pixbuf);
        }
        let a = if nch >= 4 { pixels[3] } else { 255 };
        (pixels[0], pixels[1], pixels[2], a)
    };
    if a < 255 {
        return Some(pixbuf);
    }
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
