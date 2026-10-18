//! NekoLand desktop — overlay multi-écrans toujours au premier plan.
//!
//! Architecture :
//!   - Un overlay par moniteur physique (always-on-top, click-through dynamique).
//!   - Espace de coordonnées global unifié avec confinement strict aux écrans visibles.
//!   - Drag & Drop interactif : le chat peut être attrapé à la souris/tactile et déplacé.
//!   - Le chat poursuit une pelote (toy) ou le curseur, ou erre de manière autonome.
//!   - Réglages persistants (config.json), menu de barre système (tray) et dashboard GTK4 épuré.

#![windows_subsystem = "windows"]

// GdkWin32Screen and gdk_win32_display_get_win32hcursor were removed in GTK4 4.15+ / 4.16+;
// gdk4-win32 0.9.5 still references them. Stubs provide missing symbols to linker.
#[cfg(target_os = "windows")]
#[no_mangle]
pub extern "C" fn gdk_win32_screen_get_type() -> u64 { 0 }

#[cfg(target_os = "windows")]
#[no_mangle]
pub extern "C" fn gdk_win32_display_get_win32hcursor(_display: *mut std::ffi::c_void, _cursor: *mut std::ffi::c_void) -> isize { 0 }

pub mod config;
mod dock;
pub mod i18n;
pub mod basket;
pub mod mapper;
mod mouse;
pub mod pet;
pub mod toy;
#[cfg(target_os = "linux")]
mod tray;
#[cfg(not(target_os = "linux"))]
mod tray_sys;
#[cfg(not(target_os = "linux"))]
use tray_sys as tray;
mod twitch;
pub mod util;

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

use basket::Basket;
use pet::{Pet, Sprites, State};
use toy::Toy;
use twitch::Target;
use util::{MonitorMapping, ScreenRect};

const APP_ID: &str = "com.warmadon.neko_rust";
const DEFAULT_SKIN: &str = "neko";
const REST_TICKS: i32 = 25;
const DOCK_DELAY: std::time::Duration = std::time::Duration::from_secs(60);
const TWITCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

enum Routine {
    Free,
    Zoomies { targets: Vec<(f64, f64)>, idx: usize },
    DockPerimeter { waypoints: Vec<(f64, f64)>, idx: usize },
    NapInBasket { ticks_remaining: i32 },
}

struct Dbg {
    on: bool,
    target: (f64, f64),
    behavior: &'static str,
    dock: Option<(f64, f64, f64, f64, i64)>,
}

fn map_cursor_to_logical(
    cx: f64,
    cy: f64,
    mappings: &[MonitorMapping],
    orig_x: i32,
    orig_y: i32,
) -> (f64, f64) {
    #[cfg(target_os = "windows")]
    {
        for m in mappings {
            if m.contains_phys(cx, cy) {
                return m.to_logical(cx, cy);
            }
        }
        if let Some(m) = mappings.first() {
            return m.to_logical(cx, cy);
        }
    }
    let _ = mappings;
    (cx - orig_x as f64, cy - orig_y as f64)
}

fn map_dock_to_logical(
    wx: f64,
    wy: f64,
    ww: f64,
    wh: f64,
    mappings: &[MonitorMapping],
    orig_x: i32,
    orig_y: i32,
) -> (f64, f64, f64, f64) {
    #[cfg(target_os = "windows")]
    {
        for m in mappings {
            if m.contains_phys(wx, wy) {
                let (lx, ly) = m.to_logical(wx, wy);
                return (lx, ly, ww / m.sf, wh / m.sf);
            }
        }
        if let Some(m) = mappings.first() {
            let (lx, ly) = m.to_logical(wx, wy);
            return (lx, ly, ww / m.sf, wh / m.sf);
        }
    }
    let _ = mappings;
    (wx - orig_x as f64, wy - orig_y as f64, ww, wh)
}

#[derive(Clone, Copy, PartialEq)]
enum DragTarget {
    None,
    Pet,
    Basket,
}

struct DragState {
    target: DragTarget,
    offset_x: f64,
    offset_y: f64,
}

fn handle_cli(cfg: &mut config::Config, assets: &Path) {
    let args: Vec<String> = std::env::args().collect();
    if !cfg.lang.is_empty() {
        i18n::set_lang(&cfg.lang);
    }

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => {
                #[cfg(target_os = "windows")]
                unsafe {
                    windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
                }
                println!("NekoLand v{}\n{}\n", env!("CARGO_PKG_VERSION"), i18n::t("cli_desc"));
                println!("{}\n", i18n::t("cli_usage"));
                println!("Options:");
                println!("  -h, --help               {}", i18n::t("cli_opt_help"));
                println!("  -v, --version            {}", i18n::t("cli_opt_version"));
                println!("  -s, --skin <NAME>        {}", i18n::t("cli_opt_skin"));
                println!("  -t, --toy <NAME>         {}", i18n::t("cli_opt_toy"));
                println!("  -b, --basket             {}", i18n::t("cli_opt_basket"));
                println!("  -m, --mode <MODE>        {}", i18n::t("cli_opt_mode"));
                println!("  -z, --scale <FACTOR>     {}", i18n::t("cli_opt_scale"));
                println!("  -c, --twitch <CHANNEL>   {}", i18n::t("cli_opt_twitch"));
                println!("  -l, --lang <LANG>        {}", i18n::t("cli_opt_lang"));
                println!("      --list-skins         {}", i18n::t("cli_opt_list_skins"));
                println!("      --list-toys          {}", i18n::t("cli_opt_list_toys"));
                println!("  -d, --debug              {}", i18n::t("cli_opt_debug"));
                std::process::exit(0);
            }
            "-v" | "--version" => {
                #[cfg(target_os = "windows")]
                unsafe {
                    windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
                }
                println!("NekoLand v{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--list-skins" => {
                #[cfg(target_os = "windows")]
                unsafe {
                    windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
                }
                println!("{}", i18n::t("cli_available_skins"));
                let skins = list_pngs(&assets.join("pets"));
                for s in skins {
                    println!("  - {}", s);
                }
                std::process::exit(0);
            }
            "--list-toys" => {
                #[cfg(target_os = "windows")]
                unsafe {
                    windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
                }
                println!("{}", i18n::t("cli_available_toys"));
                let toys = list_pngs(&assets.join("toys"));
                for t in toys {
                    println!("  - {}", t);
                }
                std::process::exit(0);
            }
            "-s" | "--skin" if i + 1 < args.len() => {
                i += 1;
                cfg.skin = args[i].clone();
            }
            "-t" | "--toy" if i + 1 < args.len() => {
                i += 1;
                cfg.toy = args[i].clone();
            }
            "-b" | "--basket" => {
                cfg.basket = true;
            }
            "-m" | "--mode" if i + 1 < args.len() => {
                i += 1;
                let m = args[i].to_lowercase();
                if m.starts_with("pel") || m.starts_with("yar") {
                    cfg.mode = "Pelote".to_string();
                } else if m.starts_with("auto") {
                    cfg.mode = "Autonome".to_string();
                } else if m.starts_with("sou") || m.starts_with("mous") {
                    cfg.mode = "Souris".to_string();
                } else if m.starts_with("som") || m.starts_with("sleep") {
                    cfg.mode = "Sommeil".to_string();
                } else {
                    cfg.mode = args[i].clone();
                }
            }
            "-z" | "--scale" if i + 1 < args.len() => {
                i += 1;
                if let Ok(sc) = args[i].parse::<f64>() {
                    cfg.scale = sc;
                }
            }
            "-c" | "--twitch" if i + 1 < args.len() => {
                i += 1;
                cfg.twitch_channel = args[i].clone();
            }
            "-l" | "--lang" if i + 1 < args.len() => {
                i += 1;
                let l = args[i].clone();
                cfg.lang = l.clone();
                i18n::set_lang(&l);
            }
            "-d" | "--debug" => {
                std::env::set_var("NEKO_DEBUG", "1");
            }
            _ => {}
        }
        i += 1;
    }
}

fn main() -> glib::ExitCode {
    let assets = assets_dir();
    let mut cfg = config::load();
    let initial_cfg = cfg.clone();
    handle_cli(&mut cfg, &assets);
    if cfg != initial_cfg {
        config::save(&cfg);
    }

    #[cfg(target_os = "windows")]
    if std::env::var("NEKO_DEBUG").is_ok() {
        unsafe { windows_sys::Win32::System::Console::AllocConsole(); }
    }

    std::env::set_var("GSK_RENDERER", "cairo");

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[])
}

fn build_ui(app: &Application) {
    let display = gtk::gdk::Display::default().expect("aucun display GDK");
    let cfg = config::load();
    let assets = assets_dir();
    let debug = std::env::var("NEKO_DEBUG").is_ok();

    if !cfg.lang.is_empty() {
        i18n::set_lang(&cfg.lang);
    }

    // ---- Espace global et calcul des écrans réels ----
    let monitors = monitors(&display);
    let (orig_x, orig_y, total_w, total_h) = union_bounds(&monitors);

    let mut screens = Vec::with_capacity(monitors.len());
    let mut monitor_mappings = Vec::with_capacity(monitors.len());
    for m in &monitors {
        let g = m.geometry();
        let sf = m.scale_factor().max(1) as f64;
        let s = ScreenRect::new(
            (g.x() - orig_x) as f64,
            (g.y() - orig_y) as f64,
            g.width() as f64,
            g.height() as f64,
        );
        monitor_mappings.push(MonitorMapping::new(
            g.x() as f64 * sf,
            g.y() as f64 * sf,
            g.width() as f64 * sf,
            g.height() as f64 * sf,
            s.x,
            s.y,
            sf,
        ));
        screens.push(s);
    }

    if debug {
        eprintln!("[neko][debug] ── démarrage ──");
        eprintln!("[neko][debug] version       {}", env!("CARGO_PKG_VERSION"));
        eprintln!("[neko][debug] assets        {}", assets.display());
        eprintln!("[neko][debug] config        {}", config::config_path().display());
        eprintln!(
            "[neko][debug] réglages      skin={} toy={} scale={} mode={} twitch={} lang={}",
            cfg.skin, cfg.toy, cfg.scale, cfg.mode,
            if cfg.twitch_channel.is_empty() { "-" } else { &cfg.twitch_channel },
            i18n::current_lang()
        );
        eprintln!("[neko][debug] union         origine=({orig_x},{orig_y}) {total_w}×{total_h}");
        for (i, s) in screens.iter().enumerate() {
            eprintln!("[neko][debug] écran {}       {}×{} @ ({},{})", i, s.w, s.h, s.x, s.y);
        }
    }

    let bg = std::env::var("NEKO_BG").unwrap_or_else(|_| "transparent".to_string());
    let provider = gtk::CssProvider::new();
    provider.load_from_data(&format!(
        "window.transparent-overlay, \
         window.transparent-overlay.background, \
         window.transparent-overlay .background, \
         .transparent-overlay, \
         .transparent-overlay * {{ \
            background: {bg}; \
            background-color: {bg}; \
            background-image: none; \
         }}"
    ));
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );

    let shared = Arc::new(Mutex::new(Target {
        x: total_w / 2.0,
        y: total_h / 2.0,
        active: false,
        updated: std::time::Instant::now(),
    }));

    let scale = Rc::new(Cell::new(cfg.scale));
    let dbg = Rc::new(RefCell::new(Dbg {
        on: debug,
        target: (total_w / 2.0, total_h / 2.0),
        behavior: "",
        dock: None,
    }));
    let sprite_size = pet::TILE as f64 * cfg.scale;

    let sprites = load_mapping(&assets, &cfg.skin);
    let pet = Rc::new(RefCell::new(Pet::new(total_w, total_h, sprite_size, sprites, screens.clone())));
    let pet_pix = Rc::new(RefCell::new(load_sprite(&pets_png(&assets, &cfg.skin))));

    let toy = Rc::new(RefCell::new(Toy::new(total_w, total_h, sprite_size, screens.clone())));
    let toy_pix = Rc::new(RefCell::new(load_sprite(&toys_png(&assets, &cfg.toy))));
    if cfg.mode != "Pelote" {
        toy.borrow_mut().hide();
    }

    let basket = Rc::new(RefCell::new(Basket::new(
        cfg.basket_x,
        cfg.basket_y,
        sprite_size,
        cfg.basket,
        screens.clone(),
    )));
    let basket_pix = Rc::new(RefCell::new(load_sprite(&assets.join("basket.png"))));

    let control = Arc::new(Mutex::new(config::Control::from_config(&cfg)));
    let scales = vec![1.0, 1.5, 2.0, 3.0];
    let modes: Vec<String> = ["Pelote", "Autonome", "Souris", "Sommeil"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    tray::spawn(
        pet_pix.borrow().as_ref(),
        list_pngs(&assets.join("pets")),
        list_pngs(&assets.join("toys")),
        scales,
        modes,
        control.clone(),
    );

    #[cfg(target_os = "windows")]
    let taskbar_owner: windows_sys::Win32::Foundation::HWND = unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, WS_EX_TOOLWINDOW, WS_POPUP, CW_USEDEFAULT,
        };
        let class: Vec<u16> = OsStr::new("Static").encode_wide().chain(Some(0)).collect();
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            CW_USEDEFAULT, CW_USEDEFAULT, 0, 0,
            0, 0, 0, std::ptr::null(),
        )
    };

    let drag_state = Rc::new(RefCell::new(DragState {
        target: DragTarget::None,
        offset_x: 0.0,
        offset_y: 0.0,
    }));

    let areas: Rc<RefCell<Vec<DrawingArea>>> = Rc::new(RefCell::new(Vec::with_capacity(monitors.len())));
    let mut overlay_windows = Vec::with_capacity(monitors.len());

    for (i, monitor) in monitors.iter().enumerate() {
        let geo = monitor.geometry();
        let off_x = screens[i].x;
        let off_y = screens[i].y;

        let window = ApplicationWindow::builder().application(app).build();
        window.add_css_class("transparent-overlay");

        #[cfg(target_os = "linux")]
        {
            if gtk4_layer_shell::is_supported() {
                window.init_layer_shell();
                window.set_layer(Layer::Overlay);
                window.set_monitor(monitor);
                for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
                    window.set_anchor(edge, true);
                }
                window.set_exclusive_zone(-1);
                window.set_keyboard_mode(KeyboardMode::None);
            } else {
                window.set_decorated(false);
                window.set_default_size(geo.width(), geo.height());
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            window.set_decorated(false);
            let sf = monitor.scale_factor();
            window.set_default_size(geo.width(), geo.height());

            let mx = geo.x() * sf;
            let my = geo.y() * sf;
            let mw = geo.width() * sf;
            let mh = geo.height() * sf;

            window.connect_map(move |w| {
                #[cfg(target_os = "windows")]
                {
                    use gdk4_win32::Win32Surface;
                    use windows_sys::Win32::Foundation::HWND;

                    if let Some(surface) = w.surface() {
                        if let Ok(win32_surface) = surface.downcast::<Win32Surface>() {
                            let hwnd = win32_surface.handle().0 as HWND;
                            glib::timeout_add_local_once(
                                std::time::Duration::from_millis(500),
                                move || {
                                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                                        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos,
                                        SetLayeredWindowAttributes,
                                        GWL_EXSTYLE, GWLP_HWNDPARENT, HWND_TOPMOST, LWA_ALPHA,
                                        WS_EX_LAYERED, WS_EX_TRANSPARENT, WS_EX_APPWINDOW,
                                    };
                                    unsafe {
                                        SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, taskbar_owner as isize);
                                        SetWindowPos(hwnd, HWND_TOPMOST, mx, my, mw, mh, 0);
                                        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                                        let new_style = (style
                                            | (WS_EX_LAYERED | WS_EX_TRANSPARENT) as isize)
                                            & !(WS_EX_APPWINDOW as isize);
                                        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
                                        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
                                    }
                                },
                            );
                        }
                    }
                }

                #[cfg(target_os = "macos")]
                {
                    use gdk4_macos::MacosSurface;
                    use objc::{msg_send, sel, sel_impl};

                    if let Some(surface) = w.surface() {
                        if let Ok(mac_surface) = surface.downcast::<MacosSurface>() {
                            let nswindow_ptr = mac_surface.native();
                            unsafe {
                                let nswindow: *mut objc::runtime::Object = nswindow_ptr as _;
                                let _: () = msg_send![nswindow, setIgnoresMouseEvents: true];
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

        // Dessin du panier, de la pelote, du chat et du débug
        {
            let pet = pet.clone();
            let pet_pix = pet_pix.clone();
            let toy = toy.clone();
            let toy_pix = toy_pix.clone();
            let basket = basket.clone();
            let basket_pix = basket_pix.clone();
            let scale = scale.clone();
            let dbg = dbg.clone();

            area.set_draw_func(move |_a, cr, _w, _h| {
                let sc = scale.get();
                let blit = |pb: &Pixbuf, fx: i32, fy: i32, gx: f64, gy: f64| {
                    let sub = pb.new_subpixbuf(fx, fy, pet::TILE, pet::TILE);
                    let _ = cr.save();
                    cr.translate(gx - off_x, gy - off_y);
                    cr.scale(sc, sc);
                    cr.set_source_pixbuf(&sub, 0.0, 0.0);
                    let _ = cr.paint();
                    let _ = cr.restore();
                };

                // Dessin du panier douillet (en dessous du chat)
                if let Some(bp) = basket_pix.borrow().as_ref() {
                    let b = basket.borrow();
                    if b.active {
                        let (fx, fy) = b.current_frame();
                        blit(bp, fx, fy, b.x, b.y);
                    }
                }

                if let Some(tp) = toy_pix.borrow().as_ref() {
                    let t = toy.borrow();
                    if t.active {
                        let (fx, fy) = t.current_frame();
                        blit(tp, fx, fy, t.x, t.y);
                    }
                }

                if let Some(pb) = pet_pix.borrow().as_ref() {
                    let p = pet.borrow();
                    let (fx, fy) = p.current_frame();
                    blit(pb, fx, fy, p.x, p.y);
                }

                let d = dbg.borrow();
                if d.on {
                    let p = pet.borrow();
                    let half = pet::TILE as f64 * sc / 2.0;
                    let cx = p.x - off_x + half;
                    let cy = p.y - off_y + half;
                    let tx = d.target.0 - off_x;
                    let ty = d.target.1 - off_y;

                    cr.set_line_width(2.0);
                    cr.set_source_rgba(1.0, 0.3, 0.3, 0.8);
                    cr.move_to(cx, cy);
                    cr.line_to(tx, ty);
                    let _ = cr.stroke();
                    cr.move_to(tx - 7.0, ty);
                    cr.line_to(tx + 7.0, ty);
                    cr.move_to(tx, ty - 7.0);
                    cr.line_to(tx, ty + 7.0);
                    let _ = cr.stroke();

                    cr.set_source_rgba(0.3, 1.0, 0.5, 0.95);
                    cr.set_font_size(13.0);
                    cr.move_to(cx + half + 4.0, cy - half - 4.0);
                    let beh_name = crate::i18n::translate_behavior(d.behavior);
                    let clip_name = crate::i18n::translate_clip(p.current_clip());
                    let _ = cr.show_text(&format!("{} · {}", beh_name, clip_name));

                    if let Some((wx, wy, ww, _wh, rem)) = d.dock {
                        let rx = wx + ww - off_x;
                        let ry = wy - off_y;
                        let label = if rem > 0 {
                            format!("{} {rem}s", crate::i18n::t("dock_in"))
                        } else {
                            crate::i18n::t("docked")
                        };
                        cr.set_font_size(13.0);
                        let tw = cr.text_extents(&label).map(|e| e.width()).unwrap_or(60.0);
                        cr.move_to(rx - tw - 6.0, ry + 16.0);
                        let _ = cr.show_text(&label);
                    }
                }
            });
        }

        // Contrôleur de clic et de drag GTK pour Linux et macOS
        {
            let click = gtk::GestureClick::new();
            let pet_click = pet.clone();
            let basket_click = basket.clone();
            let drag_click = drag_state.clone();
            let all_areas_click = areas.clone();

            click.connect_pressed(move |_g, _n, start_x, start_y| {
                let gx = start_x + off_x;
                let gy = start_y + off_y;
                let p = pet_click.borrow();
                if p.hit_test(gx, gy) {
                    let mut ds = drag_click.borrow_mut();
                    ds.target = DragTarget::Pet;
                    ds.offset_x = gx - p.x;
                    ds.offset_y = gy - p.y;
                    drop(p);
                    pet_click.borrow_mut().start_drag();
                    for a in all_areas_click.borrow().iter() {
                        a.queue_draw();
                    }
                } else {
                    drop(p);
                    let b = basket_click.borrow();
                    if b.hit_test(gx, gy) {
                        let mut ds = drag_click.borrow_mut();
                        ds.target = DragTarget::Basket;
                        ds.offset_x = gx - b.x;
                        ds.offset_y = gy - b.y;
                        drop(b);
                        basket_click.borrow_mut().start_drag();
                        for a in all_areas_click.borrow().iter() {
                            a.queue_draw();
                        }
                    }
                }
            });

            let pet_rel = pet.clone();
            let basket_rel = basket.clone();
            let drag_rel = drag_state.clone();
            let all_areas_rel = areas.clone();
            let control_rel = control.clone();
            click.connect_released(move |_g, _n, _x, _y| {
                let mut ds = drag_rel.borrow_mut();
                match ds.target {
                    DragTarget::Pet => {
                        ds.target = DragTarget::None;
                        pet_rel.borrow_mut().end_drag();
                        for a in all_areas_rel.borrow().iter() {
                            a.queue_draw();
                        }
                    }
                    DragTarget::Basket => {
                        ds.target = DragTarget::None;
                        let mut b = basket_rel.borrow_mut();
                        b.end_drag();
                        let mut ctrl = control_rel.lock().unwrap();
                        ctrl.basket_x = b.x;
                        ctrl.basket_y = b.y;
                        ctrl.version += 1;
                        for a in all_areas_rel.borrow().iter() {
                            a.queue_draw();
                        }
                    }
                    DragTarget::None => {}
                }
            });

            area.add_controller(click);

            let gesture = gtk::GestureDrag::new();
            let pet_begin = pet.clone();
            let basket_begin = basket.clone();
            let drag_begin = drag_state.clone();
            let all_areas_begin = areas.clone();

            gesture.connect_drag_begin(move |_g, start_x, start_y| {
                let gx = start_x + off_x;
                let gy = start_y + off_y;
                let p = pet_begin.borrow();
                if p.hit_test(gx, gy) {
                    let mut ds = drag_begin.borrow_mut();
                    ds.target = DragTarget::Pet;
                    ds.offset_x = gx - p.x;
                    ds.offset_y = gy - p.y;
                    drop(p);
                    pet_begin.borrow_mut().start_drag();
                    for a in all_areas_begin.borrow().iter() {
                        a.queue_draw();
                    }
                } else {
                    drop(p);
                    let b = basket_begin.borrow();
                    if b.hit_test(gx, gy) {
                        let mut ds = drag_begin.borrow_mut();
                        ds.target = DragTarget::Basket;
                        ds.offset_x = gx - b.x;
                        ds.offset_y = gy - b.y;
                        drop(b);
                        basket_begin.borrow_mut().start_drag();
                        for a in all_areas_begin.borrow().iter() {
                            a.queue_draw();
                        }
                    }
                }
            });

            let pet_move = pet.clone();
            let basket_move = basket.clone();
            let drag_move = drag_state.clone();
            let all_areas_move = areas.clone();
            gesture.connect_drag_update(move |_g, offset_x, offset_y| {
                let ds = drag_move.borrow();
                let start_point = _g.start_point().unwrap_or((0.0, 0.0));
                let cur_gx = start_point.0 + offset_x + off_x;
                let cur_gy = start_point.1 + offset_y + off_y;
                let target_x = cur_gx - ds.offset_x;
                let target_y = cur_gy - ds.offset_y;

                match ds.target {
                    DragTarget::Pet => {
                        pet_move.borrow_mut().drag_to(target_x, target_y);
                        for a in all_areas_move.borrow().iter() {
                            a.queue_draw();
                        }
                    }
                    DragTarget::Basket => {
                        basket_move.borrow_mut().drag_to(target_x, target_y);
                        for a in all_areas_move.borrow().iter() {
                            a.queue_draw();
                        }
                    }
                    DragTarget::None => {}
                }
            });

            let pet_end = pet.clone();
            let basket_end = basket.clone();
            let drag_end = drag_state.clone();
            let all_areas_end = areas.clone();
            let control_end = control.clone();
            let scale_end = scale.clone();

            gesture.connect_drag_end(move |_g, _offset_x, _offset_y| {
                let mut ds = drag_end.borrow_mut();
                match ds.target {
                    DragTarget::Pet => {
                        ds.target = DragTarget::None;
                        pet_end.borrow_mut().end_drag();
                        // Si le chat est lâché tout près du panier actif, il se pose dedans !
                        let b = basket_end.borrow();
                        if b.active {
                            let (bcx, bcy) = b.center();
                            let half = pet::TILE as f64 * scale_end.get() / 2.0;
                            let mut p = pet_end.borrow_mut();
                            let (pcx, pcy) = (p.x + half, p.y + half);
                            if ((pcx - bcx).powi(2) + (pcy - bcy).powi(2)).sqrt() < 40.0 {
                                p.x = b.x;
                                p.y = b.y;
                                p.startle();
                            }
                        }
                        for a in all_areas_end.borrow().iter() {
                            a.queue_draw();
                        }
                    }
                    DragTarget::Basket => {
                        ds.target = DragTarget::None;
                        let mut b = basket_end.borrow_mut();
                        b.end_drag();
                        let mut ctrl = control_end.lock().unwrap();
                        ctrl.basket_x = b.x;
                        ctrl.basket_y = b.y;
                        ctrl.version += 1;
                        for a in all_areas_end.borrow().iter() {
                            a.queue_draw();
                        }
                    }
                    DragTarget::None => {}
                }
            });

            area.add_controller(gesture);
        }

        window.set_child(Some(&area));
        window.present();

        areas.borrow_mut().push(area);
        overlay_windows.push(window);
    }

    let focus = dock::spawn();
    let cursor = mouse::spawn();

    // ---- Boucle principale d'animation à ~10 FPS ----
    {
        let app = app.clone();
        let pet = pet.clone();
        let toy = toy.clone();
        let basket = basket.clone();
        let pet_pix = pet_pix.clone();
        let toy_pix = toy_pix.clone();
        let scale = scale.clone();
        let shared = shared.clone();
        let control = control.clone();
        let assets = assets.clone();
        let focus = focus.clone();
        let cursor = cursor.clone();
        let dbg = dbg.clone();
        let areas = areas.clone();
        let overlay_windows = overlay_windows.clone();
        let screens = screens.clone();
        let monitor_mappings = monitor_mappings.clone();
        let drag_state = drag_state.clone();

        let mut rest: i32 = 0;
        let mut seen_version: u64 = 0;
        let mut seen_open: u64 = 0;
        let mut mode = cfg.mode.clone();
        let mut current_skin = cfg.skin.clone();
        let mut json_mtime = file_mtime(&pets_json(&assets, &current_skin));

        let init_pos = util::random_point_on_screens(&screens, sprite_size, sprite_size);
        let mut wander = (init_pos.0, init_pos.1, 0.0_f64);
        let mut routine = Routine::Free;
        let mut dbg_tick: u64 = 0;
        let mut last_saved = cfg.clone();

        glib::timeout_add_local(Duration::from_millis(100), move || {
            // Rechargement à chaud si le tray ou dashboard a modifié la configuration
            let (version, skin, toyname, sc, cur_mode, cur_lang, cur_basket, cur_bx, cur_by, open_config) = {
                let c = control.lock().unwrap();
                (
                    c.version,
                    c.skin.clone(),
                    c.toy.clone(),
                    c.scale,
                    c.mode.clone(),
                    c.lang.clone(),
                    c.basket,
                    c.basket_x,
                    c.basket_y,
                    c.open_config,
                )
            };

            if open_config != seen_open {
                seen_open = open_config;
                mapper::open(&app, assets.clone(), control.clone());
            }

            if version != seen_version {
                seen_version = version;
                if !cur_lang.is_empty() {
                    i18n::set_lang(&cur_lang);
                }
                *pet_pix.borrow_mut() = load_sprite(&pets_png(&assets, &skin));
                pet.borrow_mut().set_sprites(load_mapping(&assets, &skin));
                *toy_pix.borrow_mut() = load_sprite(&toys_png(&assets, &toyname));
                let size = pet::TILE as f64 * sc;
                pet.borrow_mut().set_sprite_size(size);
                toy.borrow_mut().set_size(size);
                scale.set(sc);

                if cur_mode == "Pelote" {
                    toy.borrow_mut().spawn();
                } else {
                    toy.borrow_mut().hide();
                }

                basket.borrow_mut().active = cur_basket;
                if cur_bx >= 0.0 && cur_by >= 0.0 {
                    basket.borrow_mut().x = cur_bx;
                    basket.borrow_mut().y = cur_by;
                } else if cur_bx < 0.0 {
                    basket.borrow_mut().center_on_screen();
                }
                basket.borrow_mut().set_size(size);

                mode = cur_mode.clone();
                current_skin = skin;
                json_mtime = file_mtime(&pets_json(&assets, &current_skin));
            }

            let m = file_mtime(&pets_json(&assets, &current_skin));
            if m != json_mtime {
                json_mtime = m;
                pet.borrow_mut().set_sprites(load_mapping(&assets, &current_skin));
            }

            if cur_mode != mode {
                mode = cur_mode.clone();
                if mode == "Pelote" {
                    toy.borrow_mut().spawn();
                } else {
                    toy.borrow_mut().hide();
                }
            }

            // Sauvegarde automatique dès modification
            let snapshot = { control.lock().unwrap().to_config() };
            if snapshot != last_saved {
                config::save(&snapshot);
                last_saved = snapshot;
            }

            // Gestion du Drag & Drop via souris globale (spécifique Windows où WS_EX_TRANSPARENT ignore GTK)
            #[cfg(target_os = "windows")]
            {
                let c = cursor.lock().unwrap();
                let mut ds = drag_state.borrow_mut();
                if c.valid && c.left_down {
                    let (gx, gy) = map_cursor_to_logical(c.x, c.y, &monitor_mappings, orig_x, orig_y);
                    if ds.target == DragTarget::None {
                        if pet.borrow().hit_test(gx, gy) {
                            ds.target = DragTarget::Pet;
                            ds.offset_x = gx - pet.borrow().x;
                            ds.offset_y = gy - pet.borrow().y;
                            pet.borrow_mut().start_drag();
                        } else if basket.borrow().hit_test(gx, gy) {
                            ds.target = DragTarget::Basket;
                            ds.offset_x = gx - basket.borrow().x;
                            ds.offset_y = gy - basket.borrow().y;
                            basket.borrow_mut().start_drag();
                        }
                    } else if ds.target == DragTarget::Pet {
                        pet.borrow_mut().drag_to(gx - ds.offset_x, gy - ds.offset_y);
                    } else if ds.target == DragTarget::Basket {
                        basket.borrow_mut().drag_to(gx - ds.offset_x, gy - ds.offset_y);
                    }
                } else if !c.left_down && ds.target != DragTarget::None {
                    if ds.target == DragTarget::Pet {
                        pet.borrow_mut().end_drag();
                    } else if ds.target == DragTarget::Basket {
                        basket.borrow_mut().end_drag();
                        let mut ctrl = control.lock().unwrap();
                        ctrl.basket_x = basket.borrow().x;
                        ctrl.basket_y = basket.borrow().y;
                        ctrl.version += 1;
                    }
                    ds.target = DragTarget::None;
                }
            }

            let (twx, twy, twitch_active) = {
                let t = shared.lock().unwrap();
                (t.x, t.y, t.active && t.updated.elapsed() < TWITCH_TIMEOUT)
            };

            let (px, py) = {
                let p = pet.borrow();
                (p.x, p.y)
            };
            let size = pet::TILE as f64 * scale.get();
            let half = size / 2.0;
            let cat_center = (px + half, py + half);

            // Mise à jour dynamique de l'input region de GTK
            // Permet le click-through partout sur l'écran sauf sur le chat et le panier
            let is_dragging = drag_state.borrow().target != DragTarget::None;

            for (idx, win) in overlay_windows.iter().enumerate() {
                if let Some(surface) = win.surface() {
                    let s = screens[idx];
                    if is_dragging {
                        // Pendant le drag, on capture l'input sur toute la fenêtre
                        surface.set_input_region(&gtk::cairo::Region::create_rectangle(
                            &gtk::cairo::RectangleInt::new(0, 0, s.w as i32, s.h as i32),
                        ));
                    } else {
                        let mut region_opt: Option<gtk::cairo::Region> = None;
                        let local_x = px - s.x;
                        let local_y = py - s.y;
                        let is_pet_here = local_x + size >= 0.0
                            && local_x <= s.w
                            && local_y + size >= 0.0
                            && local_y <= s.h;

                        if is_pet_here {
                            let pad = 24.0;
                            let rx = ((local_x - pad).max(0.0)).floor() as i32;
                            let ry = ((local_y - pad).max(0.0)).floor() as i32;
                            let max_w = (s.w as i32 - rx).max(1);
                            let max_h = (s.h as i32 - ry).max(1);
                            let rw = ((size + pad * 2.0).ceil() as i32).min(max_w);
                            let rh = ((size + pad * 2.0).ceil() as i32).min(max_h);
                            let rect = gtk::cairo::RectangleInt::new(rx, ry, rw, rh);
                            region_opt = Some(gtk::cairo::Region::create_rectangle(&rect));
                        }

                        let b = basket.borrow();
                        if b.active {
                            let lbx = b.x - s.x;
                            let lby = b.y - s.y;
                            let is_basket_here = lbx + size >= 0.0
                                && lbx <= s.w
                                && lby + size >= 0.0
                                && lby <= s.h;
                            if is_basket_here {
                                let pad = 24.0;
                                let rx = ((lbx - pad).max(0.0)).floor() as i32;
                                let ry = ((lby - pad).max(0.0)).floor() as i32;
                                let max_w = (s.w as i32 - rx).max(1);
                                let max_h = (s.h as i32 - ry).max(1);
                                let rw = ((size + pad * 2.0).ceil() as i32).min(max_w);
                                let rh = ((size + pad * 2.0).ceil() as i32).min(max_h);
                                let rect = gtk::cairo::RectangleInt::new(rx, ry, rw, rh);
                                if let Some(r) = region_opt.as_mut() {
                                    let _ = r.union_rectangle(&rect);
                                } else {
                                    region_opt = Some(gtk::cairo::Region::create_rectangle(&rect));
                                }
                            }
                        }

                        if let Some(r) = region_opt {
                            surface.set_input_region(&r);
                        } else {
                            surface.set_input_region(&gtk::cairo::Region::create());
                        }
                    }
                }
            }

            // Détermination du comportement et de la cible de déplacement
            let behavior;
            let target = if pet.borrow().is_dragged {
                behavior = "drag";
                cat_center
            } else if twitch_active {
                behavior = "twitch";
                (twx, twy)
            } else if mode == "Sommeil" {
                behavior = "sommeil";
                if basket.borrow().active {
                    let bc = basket.borrow().center();
                    let dist = ((cat_center.0 - bc.0).powi(2) + (cat_center.1 - bc.1).powi(2)).sqrt();
                    if dist > 20.0 {
                        bc
                    } else {
                        pet.borrow_mut().x = basket.borrow().x;
                        pet.borrow_mut().y = basket.borrow().y;
                        cat_center
                    }
                } else {
                    cat_center
                }
            } else if mode == "Souris" {
                let c = cursor.lock().unwrap();
                if c.valid {
                    behavior = "souris";
                    let (gx, gy) = map_cursor_to_logical(c.x, c.y, &monitor_mappings, orig_x, orig_y);
                    let pt = util::clamp_to_screens(gx - half, gy - half, size, size, &screens);
                    (pt.0 + half, pt.1 + half)
                } else {
                    behavior = "souris";
                    cat_center
                }
            } else if mode == "Autonome" {
                let docked = {
                    let f = focus.lock().unwrap();
                    (f.valid && f.since.elapsed() >= DOCK_DELAY).then(|| (f.x, f.y, f.w, f.h))
                };

                match &routine {
                    Routine::DockPerimeter { .. } if docked.is_none() => routine = Routine::Free,
                    Routine::Zoomies { .. } if docked.is_some() => routine = Routine::Free,
                    _ => {}
                }

                if let Some((raw_wx, raw_wy, raw_ww, raw_wh)) = docked {
                    let (wx, wy, ww, wh) = map_dock_to_logical(raw_wx, raw_wy, raw_ww, raw_wh, &monitor_mappings, orig_x, orig_y);
                    let dock_y = (wy + wh) - size;
                    let top_y = wy + half;
                    let left_x = wx + half;
                    let right_x = (wx + ww - half).max(left_x);

                    let mut next_routine: Option<Routine> = None;
                    let target = match &mut routine {
                        Routine::DockPerimeter { waypoints, idx } => {
                            behavior = "périmètre";
                            let wp = waypoints[*idx];
                            let dist = ((px + half - wp.0).powi(2) + (py + half - wp.1).powi(2)).sqrt();
                            if dist <= 40.0 {
                                *idx += 1;
                                if *idx >= waypoints.len() {
                                    next_routine = Some(Routine::Free);
                                    let pt = util::random_point_on_screens(&screens, size, size);
                                    wander.0 = pt.0 + half;
                                    wander.1 = pt.1 + half;
                                    wander.2 = 60.0 + util::rand_unit() * 100.0;
                                }
                            }
                            wp
                        }
                        _ => {
                            behavior = "dock";
                            pet.borrow_mut().y = dock_y;
                            wander.2 -= 1.0;
                            if wander.2 < 0.0 || wander.0 < left_x || wander.0 > right_x {
                                if util::rand_unit() < 0.15 {
                                    let waypoints = vec![
                                        (right_x, dock_y + half),
                                        (right_x, top_y),
                                        (left_x, top_y),
                                        (left_x, dock_y + half),
                                    ];
                                    next_routine = Some(Routine::DockPerimeter { waypoints, idx: 0 });
                                } else {
                                    wander.0 = left_x + util::rand_unit() * (right_x - left_x).max(1.0);
                                    wander.2 = 60.0 + util::rand_unit() * 100.0;
                                }
                            }
                            (wander.0.clamp(left_x, right_x), dock_y + half)
                        }
                    };
                    if let Some(r) = next_routine { routine = r; }
                    target
                } else {
                    let mut next_routine: Option<Routine> = None;
                    let target = match &mut routine {
                        Routine::NapInBasket { ticks_remaining } => {
                            behavior = "sieste";
                            *ticks_remaining -= 1;
                            let bx = basket.borrow().x;
                            let by = basket.borrow().y;
                            pet.borrow_mut().x = bx;
                            pet.borrow_mut().y = by;
                            if *ticks_remaining <= 0 {
                                next_routine = Some(Routine::Free);
                                let pt = util::random_point_on_screens(&screens, size, size);
                                wander.0 = pt.0 + half;
                                wander.1 = pt.1 + half;
                                wander.2 = 80.0 + util::rand_unit() * 120.0;
                                pet.borrow_mut().startle();
                            }
                            cat_center
                        }
                        Routine::Zoomies { targets, idx } => {
                            behavior = "zoomies";
                            let t = targets[*idx];
                            let dist = ((px + half - t.0).powi(2) + (py + half - t.1).powi(2)).sqrt();
                            if dist <= 32.0 {
                                *idx += 1;
                                if *idx >= targets.len() {
                                    next_routine = Some(Routine::Free);
                                    let pt = util::random_point_on_screens(&screens, size, size);
                                    wander.0 = pt.0 + half;
                                    wander.1 = pt.1 + half;
                                    wander.2 = 80.0 + util::rand_unit() * 120.0;
                                }
                            }
                            t
                        }
                        _ => {
                            behavior = "errance";
                            wander.2 -= 1.0;
                            if wander.2 < 0.0 {
                                // Si le panier est actif, 35% de chance de s'y rendre pour une sieste
                                if basket.borrow().active && util::rand_unit() < 0.35 {
                                    let bc = basket.borrow().center();
                                    wander.0 = bc.0;
                                    wander.1 = bc.1;
                                    wander.2 = 100.0;
                                } else if util::rand_unit() < 0.10 {
                                    let n = 3 + (util::rand_unit() * 3.0) as usize;
                                    let targets: Vec<(f64, f64)> = (0..n)
                                        .map(|_| {
                                            let pt = util::random_point_on_screens(&screens, size, size);
                                            (pt.0 + half, pt.1 + half)
                                        })
                                        .collect();
                                    next_routine = Some(Routine::Zoomies { targets, idx: 0 });
                                } else {
                                    let pt = util::random_point_on_screens(&screens, size, size);
                                    wander.0 = pt.0 + half;
                                    wander.1 = pt.1 + half;
                                    wander.2 = 80.0 + util::rand_unit() * 120.0;
                                }
                            }

                            // Détection de l'arrivée au panier actif pour commencer la sieste
                            if basket.borrow().active {
                                let bc = basket.borrow().center();
                                let dist = ((px + half - bc.0).powi(2) + (py + half - bc.1).powi(2)).sqrt();
                                if dist <= 24.0 && wander.2 < 95.0 {
                                    pet.borrow_mut().x = basket.borrow().x;
                                    pet.borrow_mut().y = basket.borrow().y;
                                    next_routine = Some(Routine::NapInBasket {
                                        ticks_remaining: 120 + (util::rand_unit() * 80.0) as i32,
                                    });
                                }
                            }
                            (wander.0, wander.1)
                        }
                    };
                    if let Some(r) = next_routine { routine = r; }
                    target
                }
            } else {
                // Mode Pelote
                if toy.borrow().active {
                    behavior = "chasse";
                    toy.borrow().center()
                } else if rest > 0 {
                    behavior = "repos";
                    rest -= 1;
                    if rest == 0 {
                        toy.borrow_mut().spawn();
                        pet.borrow_mut().startle();
                    }
                    if basket.borrow().active {
                        let bc = basket.borrow().center();
                        let dist = ((cat_center.0 - bc.0).powi(2) + (cat_center.1 - bc.1).powi(2)).sqrt();
                        if dist > 20.0 {
                            bc
                        } else {
                            pet.borrow_mut().x = basket.borrow().x;
                            pet.borrow_mut().y = basket.borrow().y;
                            cat_center
                        }
                    } else {
                        cat_center
                    }
                } else {
                    behavior = "attente";
                    cat_center
                }
            };

            // Mise à jour de la pelote (garantie confinée aux écrans réels)
            if toy.borrow_mut().update(cat_center.0, cat_center.1) {
                rest = REST_TICKS;
            }

            // Déplacement du chat vers la cible (si non dragué)
            pet.borrow_mut().update(target, State::Chase);

            // Log de débogage périodique
            if debug {
                dbg_tick += 1;
                if dbg_tick % 30 == 0 {
                    let p = pet.borrow();
                    let t = toy.borrow();
                    eprintln!(
                        "[neko][tick] mode={behavior} clip={} pos=({:.0},{:.0}) target=({:.0},{:.0}) toy_active={}",
                        p.current_clip(), p.x, p.y, target.0, target.1, t.active
                    );
                }
                let mut d = dbg.borrow_mut();
                d.target = target;
                d.behavior = behavior;
                let f = focus.lock().unwrap();
                d.dock = (f.valid && mode == "Autonome").then(|| {
                    let rem = DOCK_DELAY.saturating_sub(f.since.elapsed()).as_secs() as i64;
                    let (wx, wy, ww, wh) = map_dock_to_logical(f.x, f.y, f.w, f.h, &monitor_mappings, orig_x, orig_y);
                    (wx, wy, ww, wh, rem)
                });
            }

            // Animation du panier douillet
            basket.borrow_mut().tick();

            // Demande de redessin de toutes les zones
            for area in areas.borrow().iter() {
                area.queue_draw();
            }

            glib::ControlFlow::Continue
        });
    }

    // Gestionnaire de connexion Twitch Heat
    {
        let shared = shared.clone();
        let control = control.clone();
        let mut last_channel = cfg.twitch_channel.clone();
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

        if !last_channel.is_empty() {
            rt.spawn(twitch::run(last_channel.clone(), shared.clone(), total_w, total_h));
        }

        glib::timeout_add_local(Duration::from_millis(500), move || {
            let current = { control.lock().unwrap().twitch_channel.clone() };
            if current != last_channel {
                last_channel = current.clone();
                if !last_channel.is_empty() {
                    rt.spawn(twitch::run(last_channel.clone(), shared.clone(), total_w, total_h));
                }
            }
            glib::ControlFlow::Continue
        });
    }
}

pub fn assets_dir() -> PathBuf {
    if let Ok(d) = std::env::var("NEKO_ASSETS") {
        return PathBuf::from(d);
    }

    let mut candidates = Vec::new();
    candidates.push(PathBuf::from("assets"));

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("assets"));
            if let Some(up) = dir.parent().and_then(Path::parent) {
                candidates.push(up.join("assets"));
            }
        }
    }

    candidates.push(PathBuf::from("/usr/share/neko-desktop/assets"));

    let mut default_user_dir = None;
    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        default_user_dir = Some(PathBuf::from(appdata).join("neko-desktop").join("assets"));
    }
    #[cfg(not(target_os = "windows"))]
    if let Ok(home) = std::env::var("HOME") {
        default_user_dir = Some(PathBuf::from(home).join(".local/share/neko-desktop/assets"));
    }

    if let Some(ref d) = default_user_dir {
        candidates.push(d.clone());
    }

    if let Some(found) = candidates.iter().find(|p| p.join("pets").is_dir()) {
        return found.clone();
    }

    let target = default_user_dir.unwrap_or_else(|| PathBuf::from("assets"));
    let _ = std::fs::create_dir_all(&target);

    let zip_data = include_bytes!("../default_resources.zip");
    if let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(zip_data)) {
        let _ = archive.extract(&target);
    }

    target
}

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

fn file_mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

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

fn monitors(display: &gtk::gdk::Display) -> Vec<gtk::gdk::Monitor> {
    let model = display.monitors();
    (0..model.n_items())
        .filter_map(|i| model.item(i).and_downcast::<gtk::gdk::Monitor>())
        .collect()
}

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
