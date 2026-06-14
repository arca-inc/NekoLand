//! Neko desktop — overlay Wayland (layer-shell) qui anime un chat à l'écran.
//!
//! Architecture :
//!   - UN overlay layer-shell PAR moniteur (always-on-top, click-through). Le
//!     chat évolue dans un espace de coordonnées GLOBAL (union des moniteurs) ;
//!     chaque overlay le dessine décalé de l'offset de son moniteur.
//!     → contourne l'interdiction Wayland de repositionner sa propre fenêtre.
//!   - Le chat poursuit une pelote (toy) ; clics Twitch Heat prioritaires.
//!   - Réglages persistants (config.json) ; icône de barre système (ksni).

pub mod config;
mod dock;
pub mod i18n;
pub mod mapper;
pub mod pet;
pub mod toy;
#[cfg(target_os = "linux")]
mod tray;
mod twitch;
mod util;

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
#[cfg(target_os = "linux")]
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use pet::{Pet, Sprites, State};
use toy::Toy;
use twitch::Target;

const APP_ID: &str = "com.warmadon.neko_rust";
/// Skin sans warning si son `.json` manque (il partage le mapping par défaut).
const DEFAULT_SKIN: &str = "neko";
/// Durée de repos du chat après avoir attrapé la pelote (ticks ~10/s).
const REST_TICKS: i32 = 25;
/// Focus continu d'une fenêtre au-delà duquel le chat passe en mode « dock ».
const DOCK_DELAY: std::time::Duration = std::time::Duration::from_secs(60);

/// État pour le debug visuel (overlay), partagé tick → fonctions de dessin.
struct Dbg {
    on: bool,
    target: (f64, f64),
    behavior: &'static str,
    /// Fenêtre focus + secondes restantes avant dock (`<= 0` = déjà docké).
    /// `None` hors mode Autonome ou sans fenêtre focus.
    dock: Option<(f64, f64, f64, f64, i64)>,
}

fn main() -> glib::ExitCode {
    // Évite les fuites de VRAM sous GTK4 Vulkan lors de dessins continus sur de grandes surfaces
    std::env::set_var("GSK_RENDERER", "cairo");

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let display = gtk::gdk::Display::default().expect("aucun display GDK");
    let cfg = config::load();
    let assets = assets_dir();
    let debug = std::env::var("NEKO_DEBUG").is_ok();

    // ---- Espace global = union de tous les moniteurs ----
    let monitors = monitors(&display);
    let (orig_x, orig_y, total_w, total_h) = union_bounds(&monitors);

    if debug {
        eprintln!("[neko][debug] ── démarrage ──");
        eprintln!("[neko][debug] version       {}", env!("CARGO_PKG_VERSION"));
        eprintln!("[neko][debug] assets        {}", assets.display());
        eprintln!("[neko][debug] config        {}", config::config_path().display());
        eprintln!(
            "[neko][debug] réglages      skin={} toy={} scale={} mode={} twitch={}",
            cfg.skin,
            cfg.toy,
            cfg.scale,
            cfg.mode,
            if cfg.twitch_channel.is_empty() { "-" } else { &cfg.twitch_channel },
        );
        eprintln!(
            "[neko][debug] union         origine=({orig_x},{orig_y}) {total_w}×{total_h}",
        );
        for (i, m) in monitors.iter().enumerate() {
            let g = m.geometry();
            eprintln!(
                "[neko][debug] moniteur {i}    {}×{} @ ({},{})",
                g.width(), g.height(), g.x(), g.y(),
            );
        }
        eprintln!(
            "[neko][debug] catalogue     {} skins, {} pelotes",
            list_pngs(&assets.join("pets")).len(),
            list_pngs(&assets.join("toys")).len(),
        );
        eprintln!(
            "[neko][debug] hyprctl       {}",
            if std::process::Command::new("hyprctl").arg("version").output().is_ok() {
                "présent (mode dock possible)"
            } else {
                "absent (pas de mode dock)"
            },
        );
    }

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
    let dbg = Rc::new(RefCell::new(Dbg {
        on: debug,
        target: (total_w / 2.0, total_h / 2.0),
        behavior: "",
        dock: None,
    }));
    let sprite_size = pet::TILE as f64 * cfg.scale;

    let sprites = load_mapping(&assets, &cfg.skin);
    let pet = Rc::new(RefCell::new(Pet::new(total_w, total_h, sprite_size, sprites)));
    let pet_pix = Rc::new(RefCell::new(load_sprite(&pets_png(&assets, &cfg.skin))));

    let toy = Rc::new(RefCell::new(Toy::new(total_w, total_h, sprite_size)));
    let toy_pix = Rc::new(RefCell::new(load_sprite(&toys_png(&assets, &cfg.toy))));
    if cfg.mode != "Pelote" {
        toy.borrow_mut().hide(); // pas de pelote hors du mode jeu
    }

    // ---- Tray icon + état de contrôle partagé (tray ↔ GTK) ----
    let control = Arc::new(Mutex::new(config::Control::from_config(&cfg)));
    let scales = vec![1.0, 1.5, 2.0, 3.0];
    let modes: Vec<String> = ["Pelote", "Autonome", "Sommeil"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    #[cfg(target_os = "linux")]
    tray::spawn(
        pet_pix.borrow().as_ref(),
        list_pngs(&assets.join("pets")),
        list_pngs(&assets.join("toys")),
        scales,
        modes,
        control.clone(),
    );

    // ---- Un overlay layer-shell par moniteur ----
    let mut areas = Vec::with_capacity(monitors.len());
    for monitor in &monitors {
        let geo = monitor.geometry();
        let off_x = (geo.x() - orig_x) as f64; // offset du moniteur dans l'union
        let off_y = (geo.y() - orig_y) as f64;

        let window = ApplicationWindow::builder().application(app).build();
        #[cfg(target_os = "linux")]
        {
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_monitor(monitor);
            for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
                window.set_anchor(edge, true);
            }
            window.set_exclusive_zone(-1);
            window.set_keyboard_mode(KeyboardMode::None);
        }
        #[cfg(not(target_os = "linux"))]
        {
            window.set_decorated(false);
            let geo = monitor.geometry();
            window.set_default_size(geo.width(), geo.height());
            
            // On connect_realize, the window gets its GdkSurface.
            // We can then extract the native handle (HWND or NSWindow) and apply
            // the "always on top" and "pass-through" styles.
            window.connect_realize(|w| {
                #[cfg(target_os = "windows")]
                {
                    use gdk4_win32::Win32Surface;
                    use windows_sys::Win32::Foundation::HWND;
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos,
                        GWL_EXSTYLE, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
                        WS_EX_LAYERED, WS_EX_TRANSPARENT,
                    };
                    
                    if let Some(surface) = w.surface() {
                        if let Ok(win32_surface) = surface.downcast::<Win32Surface>() {
                            let hwnd = win32_surface.handle() as HWND;
                            unsafe {
                                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | (WS_EX_LAYERED | WS_EX_TRANSPARENT) as isize);
                                SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
                            }
                        }
                    }
                }
                
                #[cfg(target_os = "macos")]
                {
                    use gdk4_macos::MacOSSurface;
                    use objc2::{msg_send, ClassType};
                    use objc2::rc::Id;
                    use objc2_app_kit::NSWindow;
                    
                    if let Some(surface) = w.surface() {
                        if let Ok(mac_surface) = surface.downcast::<MacOSSurface>() {
                            let nswindow_ptr = mac_surface.nswindow();
                            unsafe {
                                let nswindow: *mut objc2::ffi::objc_object = nswindow_ptr as _;
                                // setIgnoresMouseEvents:YES
                                let _: () = msg_send![nswindow, setIgnoresMouseEvents: true];
                                // setLevel:NSFloatingWindowLevel (3)
                                let _: () = msg_send![nswindow, setLevel: 3isize];
                            }
                        }
                    }
                }
            });
        }

        let area = DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        {
            let pet = pet.clone();
            let pet_pix = pet_pix.clone();
            let toy = toy.clone();
            let toy_pix = toy_pix.clone();
            let scale = scale.clone();
            let dbg = dbg.clone();
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

                // Debug visuel : trajet vers la cible + marqueur + statut.
                let d = dbg.borrow();
                if d.on {
                    let p = pet.borrow();
                    let half = pet::TILE as f64 * sc / 2.0;
                    let cx = p.x - off_x + half;
                    let cy = p.y - off_y + half;
                    let tx = d.target.0 - off_x;
                    let ty = d.target.1 - off_y;

                    cr.set_line_width(2.0);
                    cr.set_source_rgba(1.0, 0.3, 0.3, 0.8); // trajet chat → cible
                    cr.move_to(cx, cy);
                    cr.line_to(tx, ty);
                    let _ = cr.stroke();
                    cr.move_to(tx - 7.0, ty); // croix sur la cible
                    cr.line_to(tx + 7.0, ty);
                    cr.move_to(tx, ty - 7.0);
                    cr.line_to(tx, ty + 7.0);
                    let _ = cr.stroke();

                    cr.set_source_rgba(0.3, 1.0, 0.5, 0.95); // statut près du chat
                    cr.set_font_size(13.0);
                    cr.move_to(cx + half + 4.0, cy - half - 4.0);
                    let _ = cr.show_text(&format!("{} · {}", d.behavior, p.current_clip()));

                    // Compte à rebours avant dock, en haut-droite de la fenêtre focus.
                    if let Some((wx, wy, ww, _wh, rem)) = d.dock {
                        let rx = wx + ww - off_x;
                        let ry = wy - off_y;
                        let label = if rem > 0 {
                            format!("dock dans {rem}s")
                        } else {
                            "● DOCK".to_string()
                        };
                        cr.set_font_size(13.0);
                        let tw = cr.text_extents(&label).map(|e| e.width()).unwrap_or(60.0);
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.6);
                        cr.rectangle(rx - tw - 10.0, ry + 2.0, tw + 8.0, 19.0);
                        let _ = cr.fill();
                        cr.set_source_rgba(1.0, 0.85, 0.2, 0.97);
                        cr.move_to(rx - tw - 6.0, ry + 16.0);
                        let _ = cr.show_text(&label);
                    }
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

    // ---- Suivi de la fenêtre active (mode dock) ----
    let focus = dock::spawn();

    // ---- Tick d'animation ~10 fps (logique de jeu + redessine les overlays) ----
    {
        let app = app.clone();
        let pet = pet.clone();
        let toy = toy.clone();
        let pet_pix = pet_pix.clone();
        let toy_pix = toy_pix.clone();
        let scale = scale.clone();
        let shared = shared.clone();
        let control = control.clone();
        let assets = assets.clone();
        let focus = focus.clone();
        let dbg = dbg.clone();
        let mut rest: i32 = 0; // ticks de repos restants après une prise
        let mut seen_version: u64 = 0;
        let mut seen_open: u64 = 0;
        let mut mode = cfg.mode.clone();
        let mut current_skin = cfg.skin.clone();
        let mut json_mtime = file_mtime(&pets_json(&assets, &current_skin));
        let mut wander = (total_w / 2.0, total_h / 2.0, 0.0_f64); // (x, y, compteur)
        let mut dbg_tick: u64 = 0;
        glib::timeout_add_local(Duration::from_millis(100), move || {
            // Reload à chaud si le tray a changé un réglage.
            let (version, skin, toyname, sc, cur_mode, open_config) = {
                let c = control.lock().unwrap();
                (c.version, c.skin.clone(), c.toy.clone(), c.scale, c.mode.clone(), c.open_config)
            };

            // Ouverture de l'éditeur de sprites (demandée depuis le tray).
            if open_config != seen_open {
                seen_open = open_config;
                mapper::open(&app, assets.clone(), control.clone());
            }
            if version != seen_version {
                seen_version = version;
                *pet_pix.borrow_mut() = load_sprite(&pets_png(&assets, &skin));
                pet.borrow_mut().set_sprites(load_mapping(&assets, &skin));
                *toy_pix.borrow_mut() = load_sprite(&toys_png(&assets, &toyname));
                let size = pet::TILE as f64 * sc;
                pet.borrow_mut().set_sprite_size(size);
                toy.borrow_mut().set_size(size);
                scale.set(sc);
                // La pelote n'existe qu'en mode « Pelote ».
                if cur_mode == "Pelote" {
                    toy.borrow_mut().spawn();
                } else {
                    toy.borrow_mut().hide();
                }
                mode = cur_mode;
                current_skin = skin;
                json_mtime = file_mtime(&pets_json(&assets, &current_skin));
            }

            // Reload du mapping si son .json a changé (édité via l'outil sprites).
            let m = file_mtime(&pets_json(&assets, &current_skin));
            if m != json_mtime {
                json_mtime = m;
                pet.borrow_mut().set_sprites(load_mapping(&assets, &current_skin));
            }

            let (twx, twy, twitch_active) = {
                let t = shared.lock().unwrap();
                (t.x, t.y, t.active)
            };
            let (px, py) = {
                let p = pet.borrow();
                (p.x, p.y)
            };
            let size = pet::TILE as f64 * scale.get();
            let half = size / 2.0;
            let (max_x, max_y) = ((total_w - size).max(0.0), (total_h - size).max(0.0));
            let cat_center = (px + half, py + half);

            // Cible à poursuivre. Twitch Heat est prioritaire dès qu'un clic arrive.
            let behavior;
            let target = if twitch_active {
                behavior = "twitch";
                (twx, twy)
            } else if mode == "Sommeil" {
                behavior = "sommeil";
                cat_center // reste sur place → la pose « arrivée » joue le sommeil
            } else if mode == "Autonome" {
                // Si une fenêtre est focus depuis assez longtemps → mode « dock » :
                // le chat se promène sous son bord bas, sans monter ni descendre.
                let docked = {
                    let f = focus.lock().unwrap();
                    (f.valid && f.since.elapsed() >= DOCK_DELAY).then(|| (f.x, f.y, f.w, f.h))
                };
                if let Some((wx, wy, ww, wh)) = docked {
                    behavior = "dock";
                    let dock_y = ((wy + wh) - size - orig_y as f64).clamp(0.0, max_y);
                    pet.borrow_mut().y = dock_y; // bloque la hauteur
                    // Bornes du centre du chat sous la fenêtre.
                    let cmin = (wx - orig_x as f64 + half).clamp(half, total_w - half);
                    let cmax = (wx + ww - orig_x as f64 - half).clamp(cmin, total_w - half);
                    wander.2 -= 1.0;
                    if wander.2 < 0.0 || wander.0 < cmin || wander.0 > cmax {
                        wander.0 = cmin + util::rand_unit() * (cmax - cmin).max(1.0);
                        wander.2 = 60.0 + util::rand_unit() * 100.0;
                    }
                    (wander.0.clamp(cmin, cmax), dock_y + half)
                } else {
                    behavior = "errance";
                    // Errance libre : nouvelle cible (centre) aléatoire de temps en temps.
                    wander.2 -= 1.0;
                    if wander.2 < 0.0 {
                        wander.0 = half + util::rand_unit() * max_x;
                        wander.1 = half + util::rand_unit() * max_y;
                        wander.2 = 80.0 + util::rand_unit() * 120.0;
                    }
                    (wander.0, wander.1)
                }
            } else {
                behavior = "pelote";
                // Mode « Pelote » : vise (et attrape) le centre de la pelote.
                let mut t = toy.borrow_mut();
                if t.active {
                    if t.update(cat_center.0, cat_center.1) {
                        rest = REST_TICKS; // attrapée → le chat se repose
                    }
                    t.center()
                } else if rest > 0 {
                    rest -= 1;
                    cat_center
                } else {
                    t.spawn();
                    t.center()
                }
            };

            pet.borrow_mut().update(target, State::Chase);

            // Partage cible + comportement + compte à rebours dock pour le debug.
            if debug {
                let dock = if mode == "Autonome" {
                    let f = focus.lock().unwrap();
                    f.valid.then(|| {
                        let rem = DOCK_DELAY.as_secs() as i64 - f.since.elapsed().as_secs() as i64;
                        (f.x, f.y, f.w, f.h, rem)
                    })
                } else {
                    None
                };
                let mut d = dbg.borrow_mut();
                d.target = target;
                d.behavior = behavior;
                d.dock = dock;
            }

            for area in &areas {
                area.queue_draw();
            }

            // État périodique (~toutes les 2 s) en mode debug.
            if debug {
                dbg_tick += 1;
                if dbg_tick % 20 == 0 {
                    let (cx, cy) = {
                        let p = pet.borrow();
                        (p.x, p.y)
                    };
                    let f = focus.lock().unwrap();
                    let win = if f.valid {
                        format!(
                            "{} « {} » {}×{}@({},{}) focus {}s",
                            f.class,
                            f.title.chars().take(30).collect::<String>(),
                            f.w as i32, f.h as i32, f.x as i32, f.y as i32,
                            f.since.elapsed().as_secs(),
                        )
                    } else {
                        "aucune".to_string()
                    };
                    eprintln!(
                        "[neko][debug] mode={mode} comportement={behavior} pos=({cx:.0},{cy:.0}) cible=({:.0},{:.0}) | fenêtre: {win}",
                        target.0, target.1,
                    );
                }
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

/// Noms (sans extension) des `.png` d'un répertoire, triés — pour les menus tray.
pub fn list_pngs(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.extension()? == "png")
                .then(|| p.file_stem()?.to_str().map(String::from))
                .flatten()
        })
        .collect();
    names.sort();
    names
}

fn pets_png(assets: &Path, skin: &str) -> PathBuf {
    assets.join("pets").join(format!("{skin}.png"))
}
fn pets_json(assets: &Path, skin: &str) -> PathBuf {
    assets.join("pets").join(format!("{skin}.json"))
}
fn toys_png(assets: &Path, toy: &str) -> PathBuf {
    assets.join("toys").join(format!("{toy}.png"))
}

/// Date de modification d'un fichier (pour le reload à chaud du mapping).
fn file_mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
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
pub fn load_sprite(path: &Path) -> Option<Pixbuf> {
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
