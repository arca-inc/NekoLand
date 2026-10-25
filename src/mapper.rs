//! Tableau de bord de configuration (Dashboard), sobre, moderne et adaptatif.
//!
//! Présente une interface de réglages épurée par défaut, avec l'éditeur de
//! sprites accessible en mode Avancé, dimensionnée dynamiquement selon l'écran.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gtk::gdk::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, DrawingArea, DropDown,
    Entry, Grid, Label, Orientation, ScrolledWindow, Stack, StackTransitionType, StringList,
    Switch,
};
use crate::config::Control;

// ---- Autostart Windows (registre HKCU\...\Run) --------------------------------
#[cfg(target_os = "windows")]
const AUTOSTART_KEY: &str = "Nekoland";

#[cfg(target_os = "windows")]
fn autostart_get() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{
        RegOpenKeyExW, RegQueryValueExW, RegCloseKey, HKEY_CURRENT_USER,
        KEY_READ, REG_SZ,
    };
    unsafe {
        let subkey: Vec<u16> = OsStr::new(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run"
        ).encode_wide().chain(Some(0)).collect();
        let mut hkey = 0isize;
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return false;
        }
        let name: Vec<u16> = OsStr::new(AUTOSTART_KEY).encode_wide().chain(Some(0)).collect();
        let mut kind = 0u32;
        let mut size = 0u32;
        let exists = RegQueryValueExW(
            hkey, name.as_ptr(), std::ptr::null_mut(), &mut kind,
            std::ptr::null_mut(), &mut size,
        ) == 0 && kind == REG_SZ;
        RegCloseKey(hkey);
        exists
    }
}

#[cfg(target_os = "windows")]
fn autostart_set(enable: bool) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{
        RegOpenKeyExW, RegSetValueExW, RegDeleteValueW, RegCloseKey,
        HKEY_CURRENT_USER, KEY_WRITE, REG_SZ,
    };
    unsafe {
        let subkey: Vec<u16> = OsStr::new(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run"
        ).encode_wide().chain(Some(0)).collect();
        let mut hkey = 0isize;
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_WRITE, &mut hkey) != 0 {
            return;
        }
        let name: Vec<u16> = OsStr::new(AUTOSTART_KEY).encode_wide().chain(Some(0)).collect();
        if enable {
            if let Ok(exe) = std::env::current_exe() {
                let val = format!("\"{}\"", exe.display());
                let val_w: Vec<u16> = OsStr::new(&val).encode_wide().chain(Some(0)).collect();
                RegSetValueExW(
                    hkey, name.as_ptr(), 0, REG_SZ,
                    val_w.as_ptr() as *const u8,
                    (val_w.len() * 2) as u32,
                );
            }
        } else {
            RegDeleteValueW(hkey, name.as_ptr());
        }
        RegCloseKey(hkey);
    }
}

#[cfg(target_os = "linux")]
fn autostart_get() -> bool {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/autostart/nekoland.desktop").is_file()
    } else {
        false
    }
}

#[cfg(target_os = "linux")]
fn autostart_set(enable: bool) {
    if let Ok(home) = std::env::var("HOME") {
        let dir = PathBuf::from(&home).join(".config/autostart");
        let file = dir.join("nekoland.desktop");
        if enable {
            let _ = std::fs::create_dir_all(&dir);
            let bin = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "nekoland".into());
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=NekoLand\nExec={bin}\nIcon=face-smile\nTerminal=false\nCategories=Game;Utility;\nX-GNOME-Autostart-enabled=true\n"
            );
            let _ = std::fs::write(&file, content);
        } else {
            let _ = std::fs::remove_file(&file);
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn autostart_get() -> bool { false }
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn autostart_set(_enable: bool) {}

const COLS: i32 = 8;
const ROWS: i32 = 6;
const STRIDE: i32 = 33; // tuile 32 + 1px de marge
const TILE: i32 = 32;
const CELL: f64 = 2.6; // taille d'une cellule dans l'éditeur de sprites

const ANIMS: &[&str] = &[
    "idle", "alert", "tired", "sleeping", "scratchSelf", "scratchWallN", "scratchWallS",
    "scratchWallW", "scratchWallE", "N", "NE", "E", "SE", "S", "SW", "W", "NW",
];

const MODES: &[(&str, &str)] = &[
    ("mode_pelote", "Pelote"),
    ("mode_autonome", "Autonome"),
    ("mode_souris", "Souris"),
    ("mode_sommeil", "Sommeil"),
];
const SCALES: &[(&str, f64)] = &[("1.0x", 1.0), ("1.5x", 1.5), ("2.0x", 2.0), ("3.0x", 3.0)];
const LANGS: &[(&str, &str)] = &[("Français", "fr"), ("English", "en"), ("Deutsch", "de")];

type Mapping = HashMap<String, Vec<(i32, i32)>>;

fn rounded_rect(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}

pub fn open(app: &Application, assets: PathBuf, control: Arc<Mutex<Control>>) {
    let (initial_skin, initial_toy, initial_mode, initial_scale, initial_twitch, initial_lang, initial_basket) = {
        let c = control.lock().unwrap();
        (c.skin.clone(), c.toy.clone(), c.mode.clone(), c.scale, c.twitch_channel.clone(), c.lang.clone(), c.basket)
    };

    let json_path = Rc::new(RefCell::new(assets.join("pets").join(format!("{initial_skin}.json"))));
    let mapping: Rc<RefCell<Mapping>> = Rc::new(RefCell::new(load_or_default(&json_path.borrow())));
    let active: Rc<RefCell<String>> = Rc::new(RefCell::new(ANIMS[0].to_string()));
    let sheet = Rc::new(RefCell::new(crate::load_sprite(&assets.join("pets").join(format!("{initial_skin}.png")))));

    // Feuille de style CSS sobre, moderne et épurée
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        r#"
        window.dashboard {
            background-color: #0d1117;
            color: #c9d1d9;
            font-size: 13px;
        }
        .dashboard .card-box {
            background-color: #161b22;
            border: 1px solid #30363d;
            border-radius: 12px;
            padding: 18px 20px;
            margin-bottom: 14px;
        }
        .dashboard .card-title {
            font-size: 12px;
            font-weight: 700;
            letter-spacing: 0.6px;
            text-transform: uppercase;
            color: #58a6ff;
            margin-bottom: 12px;
        }
        .dashboard .setting-label {
            font-size: 13px;
            font-weight: 500;
            color: #e6edf3;
        }
        .dashboard .app-title {
            font-size: 22px;
            font-weight: 800;
            color: #f0f6fc;
        }
        .dashboard .app-subtitle {
            font-size: 13px;
            color: #8b949e;
        }
        .dashboard dropdown {
            background: transparent;
            border: none;
        }
        .dashboard dropdown > button,
        .dashboard entry {
            background-color: #0d1117;
            color: #f0f6fc;
            border-radius: 8px;
            border: 1px solid #30363d;
            padding: 7px 12px;
            min-height: 18px;
            transition: all 0.15s ease;
        }
        .dashboard dropdown > button:hover,
        .dashboard entry:hover {
            background-color: #21262d;
            border-color: #58a6ff;
        }
        .dashboard dropdown > button:focus,
        .dashboard entry:focus-within {
            border-color: #58a6ff;
            box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.2);
        }
        .dashboard .hint-banner {
            background-color: rgba(56, 139, 253, 0.1);
            border: 1px solid rgba(56, 139, 253, 0.25);
            border-radius: 8px;
            padding: 10px 14px;
            color: #79c0ff;
            font-size: 12px;
            margin-bottom: 14px;
        }
        .dashboard .primary-btn {
            background-color: #238636;
            color: #ffffff;
            font-weight: 600;
            border-radius: 8px;
            padding: 9px 18px;
            border: 1px solid rgba(240, 246, 252, 0.1);
            transition: all 0.15s ease;
        }
        .dashboard .primary-btn:hover {
            background-color: #2ea043;
        }
        .dashboard .ghost-btn {
            background-color: #21262d;
            color: #c9d1d9;
            font-weight: 500;
            border-radius: 8px;
            padding: 9px 16px;
            border: 1px solid #30363d;
            transition: all 0.15s ease;
        }
        .dashboard .ghost-btn:hover {
            background-color: #30363d;
            color: #ffffff;
            border-color: #58a6ff;
        }
        .dashboard .close-btn {
            background-color: transparent;
            color: #8b949e;
            font-weight: 500;
            border-radius: 8px;
            padding: 8px 16px;
            border: 1px solid #30363d;
        }
        .dashboard .close-btn:hover {
            background-color: #21262d;
            color: #f0f6fc;
        }
        .dashboard .editor-bar {
            background-color: #161b22;
            border-bottom: 1px solid #30363d;
            padding: 12px 20px;
        }
        .dashboard .editor-title {
            font-size: 15px;
            font-weight: 700;
            color: #f0f6fc;
        }
        .dashboard row {
            padding: 8px 12px;
            border-radius: 6px;
            margin-bottom: 3px;
            color: #8b949e;
            font-weight: 500;
        }
        .dashboard row:selected {
            background-color: #1f6feb;
            color: #ffffff;
            font-weight: 600;
        }
        .dashboard row:hover:not(:selected) {
            background-color: #21262d;
            color: #c9d1d9;
        }
        "#,
    );

    let display = gtk::gdk::Display::default().unwrap();
    gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    // Calcul de la taille de fenêtre réactive au moniteur actuel
    let (mon_w, mon_h) = {
        let monitors = display.monitors();
        if let Some(m) = monitors.item(0).and_then(|o| o.downcast::<gtk::gdk::Monitor>().ok()) {
            let g = m.geometry();
            (g.width(), g.height())
        } else {
            (1920, 1080)
        }
    };

    let settings_w = (mon_w as f64 * 0.30).clamp(460.0, 560.0) as i32;
    let settings_h = (mon_h as f64 * 0.68).clamp(540.0, 720.0) as i32;

    let editor_w = (mon_w as f64 * 0.65).clamp(840.0, 1140.0) as i32;
    let editor_h = (mon_h as f64 * 0.75).clamp(620.0, 850.0) as i32;

    let window = ApplicationWindow::builder()
        .application(app)
        .title(&crate::i18n::t("dashboard_title"))
        .build();
    window.add_css_class("dashboard");
    window.set_decorated(true);
    window.set_resizable(true);
    window.set_default_size(settings_w, settings_h);

    let stack = Stack::new();
    stack.set_transition_type(StackTransitionType::SlideLeftRight);
    stack.set_transition_duration(200);

    // =========================================================================
    // VUE 1 : PARAMÈTRES ÉPURÉS (Page principale)
    // =========================================================================
    let page_settings = ScrolledWindow::new();
    page_settings.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    page_settings.set_hexpand(true);
    page_settings.set_vexpand(true);

    let settings_box = GtkBox::new(Orientation::Vertical, 16);
    settings_box.set_margin_start(24);
    settings_box.set_margin_end(24);
    settings_box.set_margin_top(20);
    settings_box.set_margin_bottom(20);

    // Header avec aperçu animé
    let header_box = GtkBox::new(Orientation::Horizontal, 16);
    header_box.set_valign(gtk::Align::Center);

    let mini_preview = DrawingArea::new();
    mini_preview.set_content_width(64);
    mini_preview.set_content_height(64);
    let mini_tick = Rc::new(RefCell::new(0usize));
    {
        let sheet = sheet.clone();
        let mapping = mapping.clone();
        let mini_tick = mini_tick.clone();
        mini_preview.set_draw_func(move |_a, cr, w, h| {
            rounded_rect(cr, 0.0, 0.0, w as f64, h as f64, 12.0);
            cr.set_source_rgb(0.09, 0.12, 0.16);
            let _ = cr.fill();

            if let Some(s) = sheet.borrow().as_ref() {
                let m = mapping.borrow();
                let frames = m.get("idle").or_else(|| m.get("alert")).and_then(|f| if f.is_empty() { None } else { Some(f) });
                if let Some(f) = frames {
                    let tick = *mini_tick.borrow();
                    let (fc, fr) = f[tick % f.len()];
                    if fr * STRIDE + TILE <= s.height() && fc * STRIDE + TILE <= s.width() {
                        let sub = s.new_subpixbuf(fc * STRIDE, fr * STRIDE, TILE, TILE);
                        let _ = cr.save();
                        cr.translate((w as f64 - TILE as f64 * 1.6) / 2.0, (h as f64 - TILE as f64 * 1.6) / 2.0);
                        cr.scale(1.6, 1.6);
                        cr.set_source_pixbuf(&sub, 0.0, 0.0);
                        let _ = cr.paint();
                        let _ = cr.restore();
                    }
                }
            }
        });
    }
    header_box.append(&mini_preview);

    let title_box = GtkBox::new(Orientation::Vertical, 4);
    title_box.set_valign(gtk::Align::Center);
    let app_title = Label::new(Some("NekoLand"));
    app_title.add_css_class("app-title");
    app_title.set_halign(gtk::Align::Start);
    let app_sub = Label::new(Some(&crate::i18n::t("dashboard_title")));
    app_sub.add_css_class("app-subtitle");
    app_sub.set_halign(gtk::Align::Start);
    title_box.append(&app_title);
    title_box.append(&app_sub);
    header_box.append(&title_box);
    settings_box.append(&header_box);

    // Bannière d'aide discrète
    let hint_lbl = Label::new(Some(&crate::i18n::t("drag_hint")));
    hint_lbl.add_css_class("hint-banner");
    hint_lbl.set_wrap(true);
    hint_lbl.set_xalign(0.0);
    settings_box.append(&hint_lbl);

    // --- CARTE 1 : APPARENCE ---
    let card_app = GtkBox::new(Orientation::Vertical, 12);
    card_app.add_css_class("card-box");

    let card_app_title = Label::new(Some(&crate::i18n::t("section_appearance")));
    card_app_title.add_css_class("card-title");
    card_app_title.set_halign(gtk::Align::Start);
    card_app.append(&card_app_title);

    // Skin dropdown avec vignettes
    let row_skin = GtkBox::new(Orientation::Horizontal, 12);
    row_skin.set_valign(gtk::Align::Center);
    let lbl_skin = Label::new(Some(&crate::i18n::t("skin")));
    lbl_skin.add_css_class("setting-label");
    lbl_skin.set_hexpand(true);
    lbl_skin.set_halign(gtk::Align::Start);

    let skins = crate::list_pngs(&assets.join("pets"));
    let skins_ref: Vec<&str> = skins.iter().map(|s| s.as_str()).collect();
    let skin_list = StringList::new(&skins_ref);

    let mut skin_textures = HashMap::new();
    for skin in &skins {
        let path = assets.join("pets").join(format!("{skin}.png"));
        if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(&path) {
            if pixbuf.width() >= 32 && pixbuf.height() >= 32 {
                let sub = pixbuf.new_subpixbuf(0, 0, 32, 32);
                skin_textures.insert(skin.clone(), gtk::gdk::Texture::for_pixbuf(&sub));
            }
        }
    }
    let skin_textures = Rc::new(skin_textures);

    let factory_skin = gtk::SignalListItemFactory::new();
    factory_skin.connect_setup(|_, obj| {
        let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
        let hbox = GtkBox::new(Orientation::Horizontal, 8);
        let img = gtk::Image::new();
        img.set_pixel_size(22);
        let lbl = Label::new(None);
        hbox.append(&img);
        hbox.append(&lbl);
        list_item.set_child(Some(&hbox));
    });

    let skin_textures_bind = skin_textures.clone();
    factory_skin.connect_bind(move |_, obj| {
        let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
        let item = list_item.item().unwrap();
        let string_obj = item.downcast_ref::<gtk::StringObject>().unwrap();
        let name = string_obj.string();
        let hbox = list_item.child().unwrap().downcast::<GtkBox>().unwrap();
        let img = hbox.first_child().unwrap().downcast::<gtk::Image>().unwrap();
        let lbl = img.next_sibling().unwrap().downcast::<Label>().unwrap();
        lbl.set_label(&name);
        if let Some(tex) = skin_textures_bind.get(name.as_str()) {
            img.set_paintable(Some(tex));
        } else {
            img.clear();
        }
    });

    let skin_drop = DropDown::new(Some(skin_list), gtk::Expression::NONE);
    skin_drop.set_factory(Some(&factory_skin));
    skin_drop.set_list_factory(Some(&factory_skin));
    if let Some(pos) = skins.iter().position(|s| s == &initial_skin) {
        skin_drop.set_selected(pos as u32);
    }
    row_skin.append(&lbl_skin);
    row_skin.append(&skin_drop);
    card_app.append(&row_skin);

    // Jouet / Pelote dropdown
    let row_toy = GtkBox::new(Orientation::Horizontal, 12);
    row_toy.set_valign(gtk::Align::Center);
    let lbl_toy = Label::new(Some(&crate::i18n::t("toy")));
    lbl_toy.add_css_class("setting-label");
    lbl_toy.set_hexpand(true);
    lbl_toy.set_halign(gtk::Align::Start);

    let toys = crate::list_pngs(&assets.join("toys"));
    let toys_ref: Vec<&str> = toys.iter().map(|s| s.as_str()).collect();
    let toy_list = StringList::new(&toys_ref);

    let mut toy_textures = HashMap::new();
    for toy_name in &toys {
        let path = assets.join("toys").join(format!("{toy_name}.png"));
        if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(&path) {
            if pixbuf.width() >= 32 && pixbuf.height() >= 32 {
                let sub = pixbuf.new_subpixbuf(0, 0, 32, 32);
                toy_textures.insert(toy_name.clone(), gtk::gdk::Texture::for_pixbuf(&sub));
            }
        }
    }
    let toy_textures = Rc::new(toy_textures);

    let factory_toy = gtk::SignalListItemFactory::new();
    factory_toy.connect_setup(|_, obj| {
        let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
        let hbox = GtkBox::new(Orientation::Horizontal, 8);
        let img = gtk::Image::new();
        img.set_pixel_size(22);
        let lbl = Label::new(None);
        hbox.append(&img);
        hbox.append(&lbl);
        list_item.set_child(Some(&hbox));
    });

    let toy_textures_bind = toy_textures.clone();
    factory_toy.connect_bind(move |_, obj| {
        let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
        let item = list_item.item().unwrap();
        let string_obj = item.downcast_ref::<gtk::StringObject>().unwrap();
        let name = string_obj.string();
        let hbox = list_item.child().unwrap().downcast::<GtkBox>().unwrap();
        let img = hbox.first_child().unwrap().downcast::<gtk::Image>().unwrap();
        let lbl = img.next_sibling().unwrap().downcast::<Label>().unwrap();
        lbl.set_label(&name);
        if let Some(tex) = toy_textures_bind.get(name.as_str()) {
            img.set_paintable(Some(tex));
        } else {
            img.clear();
        }
    });

    let toy_drop = DropDown::new(Some(toy_list), gtk::Expression::NONE);
    toy_drop.set_factory(Some(&factory_toy));
    toy_drop.set_list_factory(Some(&factory_toy));
    if let Some(pos) = toys.iter().position(|t| t == &initial_toy) {
        toy_drop.set_selected(pos as u32);
    }
    row_toy.append(&lbl_toy);
    row_toy.append(&toy_drop);
    card_app.append(&row_toy);

    // Échelle (Scale)
    let row_scale = GtkBox::new(Orientation::Horizontal, 12);
    row_scale.set_valign(gtk::Align::Center);
    let lbl_scale = Label::new(Some(&crate::i18n::t("scale")));
    lbl_scale.add_css_class("setting-label");
    lbl_scale.set_hexpand(true);
    lbl_scale.set_halign(gtk::Align::Start);

    let scale_names: Vec<&str> = SCALES.iter().map(|(n, _)| *n).collect();
    let scale_list = StringList::new(&scale_names);
    let scale_drop = DropDown::new(Some(scale_list), gtk::Expression::NONE);
    if let Some(pos) = SCALES.iter().position(|(_, v)| (v - initial_scale).abs() < 0.1) {
        scale_drop.set_selected(pos as u32);
    }
    row_scale.append(&lbl_scale);
    row_scale.append(&scale_drop);
    card_app.append(&row_scale);

    settings_box.append(&card_app);

    // --- CARTE 2 : COMPORTEMENT & LANGUE ---
    let card_beh = GtkBox::new(Orientation::Vertical, 12);
    card_beh.add_css_class("card-box");

    let card_beh_title = Label::new(Some(&crate::i18n::t("section_behavior")));
    card_beh_title.add_css_class("card-title");
    card_beh_title.set_halign(gtk::Align::Start);
    card_beh.append(&card_beh_title);

    // Mode
    let row_mode = GtkBox::new(Orientation::Horizontal, 12);
    row_mode.set_valign(gtk::Align::Center);
    let lbl_mode = Label::new(Some(&crate::i18n::t("mode")));
    lbl_mode.add_css_class("setting-label");
    lbl_mode.set_hexpand(true);
    lbl_mode.set_halign(gtk::Align::Start);

    let mode_labels: Vec<String> = MODES.iter().map(|(key, _)| crate::i18n::t(key)).collect();
    let mode_labels_ref: Vec<&str> = mode_labels.iter().map(|s| s.as_str()).collect();
    let mode_list = StringList::new(&mode_labels_ref);
    let mode_drop = DropDown::new(Some(mode_list.clone()), gtk::Expression::NONE);
    if let Some(pos) = MODES.iter().position(|(_, m)| *m == initial_mode) {
        mode_drop.set_selected(pos as u32);
    }
    row_mode.append(&lbl_mode);
    row_mode.append(&mode_drop);
    card_beh.append(&row_mode);

    // Panier douillet
    let row_basket = GtkBox::new(Orientation::Vertical, 6);
    let top_basket = GtkBox::new(Orientation::Horizontal, 12);
    top_basket.set_valign(gtk::Align::Center);

    let basket_switch = Switch::new();
    basket_switch.set_active(initial_basket);
    basket_switch.set_valign(gtk::Align::Center);

    let lbl_basket = Label::new(Some(&crate::i18n::t("basket")));
    lbl_basket.add_css_class("setting-label");
    lbl_basket.set_hexpand(true);
    lbl_basket.set_halign(gtk::Align::Start);

    let btn_center_basket = Button::with_label(&crate::i18n::t("center_basket"));
    btn_center_basket.add_css_class("flat-btn");
    btn_center_basket.set_sensitive(initial_basket);

    top_basket.append(&lbl_basket);
    top_basket.append(&btn_center_basket);
    top_basket.append(&basket_switch);

    let lbl_basket_sub = Label::new(Some(&crate::i18n::t("basket_desc")));
    lbl_basket_sub.add_css_class("section-desc");
    lbl_basket_sub.set_halign(gtk::Align::Start);

    row_basket.append(&top_basket);
    row_basket.append(&lbl_basket_sub);
    card_beh.append(&row_basket);

    // Langue
    let row_lang = GtkBox::new(Orientation::Horizontal, 12);
    row_lang.set_valign(gtk::Align::Center);
    let lbl_lang = Label::new(Some(&crate::i18n::t("language")));
    lbl_lang.add_css_class("setting-label");
    lbl_lang.set_hexpand(true);
    lbl_lang.set_halign(gtk::Align::Start);

    let lang_names: Vec<&str> = LANGS.iter().map(|(n, _)| *n).collect();
    let lang_list = StringList::new(&lang_names);
    let lang_drop = DropDown::new(Some(lang_list), gtk::Expression::NONE);
    let cur_l = if initial_lang.is_empty() { crate::i18n::current_lang() } else { initial_lang.as_str() };
    if let Some(pos) = LANGS.iter().position(|(_, c)| *c == cur_l) {
        lang_drop.set_selected(pos as u32);
    }
    row_lang.append(&lbl_lang);
    row_lang.append(&lang_drop);
    card_beh.append(&row_lang);

    settings_box.append(&card_beh);

    // --- CARTE 3 : SYSTÈME & INTÉGRATION ---
    let card_sys = GtkBox::new(Orientation::Vertical, 12);
    card_sys.add_css_class("card-box");

    let card_sys_title = Label::new(Some(&crate::i18n::t("section_system")));
    card_sys_title.add_css_class("card-title");
    card_sys_title.set_halign(gtk::Align::Start);
    card_sys.append(&card_sys_title);

    // Démarrage auto
    let autostart_check = CheckButton::with_label(&crate::i18n::t("autostart"));
    autostart_check.set_active(autostart_get());
    autostart_check.connect_toggled(|btn| {
        autostart_set(btn.is_active());
    });
    card_sys.append(&autostart_check);

    // Twitch
    let row_twitch = GtkBox::new(Orientation::Vertical, 6);
    let lbl_twitch = Label::new(Some(&crate::i18n::t("twitch_channel")));
    lbl_twitch.add_css_class("setting-label");
    lbl_twitch.set_halign(gtk::Align::Start);

    let twitch_entry = Entry::new();
    twitch_entry.set_placeholder_text(Some(&crate::i18n::t("twitch_placeholder")));
    twitch_entry.set_text(&initial_twitch);
    row_twitch.append(&lbl_twitch);
    row_twitch.append(&twitch_entry);
    card_sys.append(&row_twitch);

    settings_box.append(&card_sys);

    // Barre d'actions en bas de la vue paramètres
    let action_bar = GtkBox::new(Orientation::Horizontal, 12);
    action_bar.set_margin_top(8);

    let close_btn = Button::with_label(&crate::i18n::t("close"));
    close_btn.add_css_class("close-btn");
    action_bar.append(&close_btn);

    let act_spacer = GtkBox::new(Orientation::Horizontal, 0);
    act_spacer.set_hexpand(true);
    action_bar.append(&act_spacer);

    let to_advanced_btn = Button::with_label(&crate::i18n::t("advanced_mode"));
    to_advanced_btn.add_css_class("ghost-btn");
    action_bar.append(&to_advanced_btn);

    settings_box.append(&action_bar);

    page_settings.set_child(Some(&settings_box));
    stack.add_named(&page_settings, Some("settings"));

    // =========================================================================
    // VUE 2 : ÉDITEUR DE SPRITES (Mode avancé)
    // =========================================================================
    let page_editor = GtkBox::new(Orientation::Vertical, 0);

    // Barre d'en-tête de l'éditeur
    let editor_bar = GtkBox::new(Orientation::Horizontal, 16);
    editor_bar.add_css_class("editor-bar");

    let back_btn = Button::with_label(&crate::i18n::t("back_to_settings"));
    back_btn.add_css_class("ghost-btn");
    editor_bar.append(&back_btn);

    let editor_title = Label::new(Some(&format!("{} : {}", crate::i18n::t("sprite_editor_tab"), initial_skin)));
    editor_title.add_css_class("editor-title");
    editor_title.set_hexpand(true);
    editor_title.set_halign(gtk::Align::Start);
    editor_bar.append(&editor_title);

    let save_btn = Button::with_label(&crate::i18n::t("save_mapping"));
    save_btn.add_css_class("primary-btn");
    editor_bar.append(&save_btn);

    page_editor.append(&editor_bar);

    // Contenu de l'éditeur : grille de tuiles + liste d'animations
    let editor_body = GtkBox::new(Orientation::Horizontal, 20);
    editor_body.set_margin_start(20);
    editor_body.set_margin_end(20);
    editor_body.set_margin_top(16);
    editor_body.set_margin_bottom(16);
    editor_body.set_vexpand(true);

    let grid = Grid::new();
    grid.set_row_spacing(6);
    grid.set_column_spacing(6);
    grid.set_halign(gtk::Align::Center);
    grid.set_valign(gtk::Align::Center);
    grid.set_hexpand(true);

    let cells: Rc<RefCell<Vec<DrawingArea>>> = Rc::new(RefCell::new(Vec::new()));

    for r in 0..ROWS {
        for c in 0..COLS {
            let da = DrawingArea::new();
            da.set_content_width((TILE as f64 * CELL) as i32);
            da.set_content_height((TILE as f64 * CELL) as i32);
            {
                let sheet = sheet.clone();
                let mapping = mapping.clone();
                da.set_draw_func(move |_a, cr, w, h| {
                    rounded_rect(cr, 0.0, 0.0, w as f64, h as f64, 6.0);
                    cr.set_source_rgb(0.1, 0.13, 0.18);
                    let _ = cr.fill();

                    if let Some(s) = sheet.borrow().as_ref() {
                        if r * STRIDE + TILE <= s.height() && c * STRIDE + TILE <= s.width() {
                            let sub = s.new_subpixbuf(c * STRIDE, r * STRIDE, TILE, TILE);
                            let _ = cr.save();
                            cr.scale(CELL, CELL);
                            cr.set_source_pixbuf(&sub, 0.0, 0.0);
                            let _ = cr.paint();
                            let _ = cr.restore();
                        }
                    }

                    let names: Vec<String> = mapping
                        .borrow()
                        .iter()
                        .filter(|(_, v)| v.iter().any(|p| *p == (c, r)))
                        .map(|(k, _)| k.clone())
                        .collect();
                    if !names.is_empty() {
                        let badge_h = 16.0;
                        rounded_rect(cr, 3.0, h as f64 - badge_h - 3.0, w as f64 - 6.0, badge_h, 4.0);
                        cr.set_source_rgba(0.12, 0.43, 0.92, 0.9);
                        let _ = cr.fill();

                        cr.set_source_rgb(1.0, 1.0, 1.0);
                        cr.set_font_size(10.0);
                        cr.move_to(6.0, h as f64 - 7.0);
                        let txt = names.join(",");
                        let _ = cr.show_text(&txt);
                    }
                });
            }
            grid.attach(&da, c, r, 1, 1);
            cells.borrow_mut().push(da);
        }
    }
    editor_body.append(&grid);

    // Panneau droit : prévisualisation animée + liste des clips
    let side_anims = GtkBox::new(Orientation::Vertical, 12);
    side_anims.set_size_request(200, -1);

    let preview_area = DrawingArea::new();
    preview_area.set_content_width(96);
    preview_area.set_content_height(96);
    preview_area.set_halign(gtk::Align::Center);
    let preview_tick = Rc::new(RefCell::new(0usize));
    {
        let sheet = sheet.clone();
        let mapping = mapping.clone();
        let active = active.clone();
        let preview_tick = preview_tick.clone();
        preview_area.set_draw_func(move |_a, cr, w, h| {
            rounded_rect(cr, 0.0, 0.0, w as f64, h as f64, 10.0);
            cr.set_source_rgb(0.08, 0.11, 0.15);
            let _ = cr.fill();

            if let Some(s) = sheet.borrow().as_ref() {
                let m = mapping.borrow();
                let act = active.borrow();
                if let Some(frames) = m.get(&*act).filter(|f| !f.is_empty()) {
                    let tick = *preview_tick.borrow();
                    let (fc, fr) = frames[tick % frames.len()];
                    if fr * STRIDE + TILE <= s.height() && fc * STRIDE + TILE <= s.width() {
                        let sub = s.new_subpixbuf(fc * STRIDE, fr * STRIDE, TILE, TILE);
                        let _ = cr.save();
                        cr.translate((w as f64 - TILE as f64 * 2.5) / 2.0, (h as f64 - TILE as f64 * 2.5) / 2.0);
                        cr.scale(2.5, 2.5);
                        cr.set_source_pixbuf(&sub, 0.0, 0.0);
                        let _ = cr.paint();
                        let _ = cr.restore();
                    }
                }
            }
        });
    }
    side_anims.append(&preview_area);

    let scroll_list = ScrolledWindow::new();
    scroll_list.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll_list.set_vexpand(true);

    let anim_list = gtk::ListBox::new();
    for anim in ANIMS {
        let row = gtk::ListBoxRow::new();
        let display = format!("{} ({})", anim, crate::i18n::translate_clip(anim));
        let lbl = Label::new(Some(&display));
        lbl.set_halign(gtk::Align::Start);
        row.set_child(Some(&lbl));
        anim_list.append(&row);
    }
    anim_list.select_row(anim_list.row_at_index(0).as_ref());

    {
        let active = active.clone();
        anim_list.connect_row_selected(move |_, row| {
            if let Some(r) = row {
                let idx = r.index() as usize;
                if idx < ANIMS.len() {
                    *active.borrow_mut() = ANIMS[idx].to_string();
                }
            }
        });
    }
    scroll_list.set_child(Some(&anim_list));
    side_anims.append(&scroll_list);
    editor_body.append(&side_anims);

    page_editor.append(&editor_body);
    stack.add_named(&page_editor, Some("editor"));

    window.set_child(Some(&stack));

    // Bascule dynamique entre Mode Paramètres et Mode Avancé
    {
        let stack_adv = stack.clone();
        let win_adv = window.clone();
        to_advanced_btn.connect_clicked(move |_| {
            win_adv.set_default_size(editor_w, editor_h);
            stack_adv.set_visible_child_name("editor");
        });
    }
    {
        let stack_back = stack.clone();
        let win_back = window.clone();
        back_btn.connect_clicked(move |_| {
            win_back.set_default_size(settings_w, settings_h);
            stack_back.set_visible_child_name("settings");
        });
    }

    // Timer d'animation des prévisualisations
    {
        let preview_area = preview_area.clone();
        let preview_tick = preview_tick.clone();
        let mini_preview = mini_preview.clone();
        let mini_tick = mini_tick.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            *preview_tick.borrow_mut() += 1;
            *mini_tick.borrow_mut() += 1;
            preview_area.queue_draw();
            mini_preview.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    // Gestion du clic sur les cellules de l'éditeur de sprites
    for (idx, da) in cells.borrow().iter().enumerate() {
        let c = (idx % COLS as usize) as i32;
        let r = (idx / COLS as usize) as i32;
        let mapping = mapping.clone();
        let active = active.clone();
        let da_clone = da.clone();

        let click = gtk::GestureClick::new();
        click.set_button(0);
        {
            let cells = cells.clone();
            click.connect_pressed(move |g, _n, _x, _y| {
                let btn = g.current_button();
                let mut m = mapping.borrow_mut();
                let act = active.borrow().clone();

                if btn == 3 {
                    for frames in m.values_mut() {
                        frames.retain(|p| *p != (c, r));
                    }
                } else if btn == 1 {
                    let entry = m.entry(act).or_default();
                    if let Some(pos) = entry.iter().position(|p| *p == (c, r)) {
                        entry.remove(pos);
                    } else {
                        entry.push((c, r));
                    }
                }
                da_clone.queue_draw();
                for other in cells.borrow().iter() {
                    other.queue_draw();
                }
            });
        }
        da.add_controller(click);
    }

    let updating_ui = Rc::new(Cell::new(false));

    // Synchronisation des changements de paramètres avec Control
    {
        let control = control.clone();
        let updating_ui_mode = updating_ui.clone();
        mode_drop.connect_selected_notify(move |drop| {
            if updating_ui_mode.get() {
                return;
            }
            let idx = drop.selected() as usize;
            if idx < MODES.len() {
                let mut ctrl = control.lock().unwrap();
                ctrl.mode = MODES[idx].1.to_string();
                ctrl.version += 1;
            }
        });
    }

    {
        let control = control.clone();
        let updating_ui_scale = updating_ui.clone();
        scale_drop.connect_selected_notify(move |drop| {
            if updating_ui_scale.get() {
                return;
            }
            let idx = drop.selected() as usize;
            if idx < SCALES.len() {
                let mut ctrl = control.lock().unwrap();
                ctrl.scale = SCALES[idx].1;
                ctrl.version += 1;
            }
        });
    }

    // Gestion de l'activation et du centrage du panier douillet
    {
        let control = control.clone();
        let btn_center = btn_center_basket.clone();
        basket_switch.connect_active_notify(move |sw| {
            let active = sw.is_active();
            btn_center.set_sensitive(active);
            let mut ctrl = control.lock().unwrap();
            ctrl.basket = active;
            ctrl.version += 1;
        });
    }

    {
        let control = control.clone();
        btn_center_basket.connect_clicked(move |_| {
            let mut ctrl = control.lock().unwrap();
            ctrl.basket_x = -1.0;
            ctrl.basket_y = -1.0;
            ctrl.basket = true;
            ctrl.version += 1;
        });
    }

    {
        let control = control.clone();
        let mode_list = mode_list.clone();
        let mode_drop = mode_drop.clone();
        let updating_ui_lang = updating_ui.clone();

        let window = window.clone();
        let app_sub = app_sub.clone();
        let hint_lbl = hint_lbl.clone();
        let card_app_title = card_app_title.clone();
        let lbl_skin = lbl_skin.clone();
        let lbl_toy = lbl_toy.clone();
        let lbl_scale = lbl_scale.clone();
        let card_beh_title = card_beh_title.clone();
        let lbl_mode = lbl_mode.clone();
        let lbl_basket = lbl_basket.clone();
        let btn_center_basket = btn_center_basket.clone();
        let lbl_basket_sub = lbl_basket_sub.clone();
        let lbl_lang = lbl_lang.clone();
        let card_sys_title = card_sys_title.clone();
        let autostart_check = autostart_check.clone();
        let lbl_twitch = lbl_twitch.clone();
        let twitch_entry = twitch_entry.clone();
        let close_btn = close_btn.clone();
        let to_advanced_btn = to_advanced_btn.clone();
        let back_btn = back_btn.clone();
        let save_btn = save_btn.clone();
        let editor_title = editor_title.clone();
        let skin_drop = skin_drop.clone();

        lang_drop.connect_selected_notify(move |drop| {
            if updating_ui_lang.get() {
                return;
            }
            let idx = drop.selected() as usize;
            if idx < LANGS.len() {
                let code = LANGS[idx].1;
                crate::i18n::set_lang(code);
                {
                    let mut ctrl = control.lock().unwrap();
                    ctrl.lang = code.to_string();
                    ctrl.version += 1;
                }

                updating_ui_lang.set(true);

                // Rafraîchir les libellés traduits des modes sans déclencher le changement de mode
                let cur_mode_sel = mode_drop.selected();
                for (i, (key, _)) in MODES.iter().enumerate() {
                    let text = crate::i18n::t(key);
                    mode_list.splice(i as u32, 1, &[&text]);
                }
                if cur_mode_sel < MODES.len() as u32 {
                    mode_drop.set_selected(cur_mode_sel);
                }

                // Mettre à jour tous les libellés de l'interface dans la nouvelle langue
                window.set_title(Some(&crate::i18n::t("dashboard_title")));
                app_sub.set_label(&crate::i18n::t("dashboard_title"));
                hint_lbl.set_label(&crate::i18n::t("drag_hint"));
                card_app_title.set_label(&crate::i18n::t("section_appearance"));
                lbl_skin.set_label(&crate::i18n::t("skin"));
                lbl_toy.set_label(&crate::i18n::t("toy"));
                lbl_scale.set_label(&crate::i18n::t("scale"));
                card_beh_title.set_label(&crate::i18n::t("section_behavior"));
                lbl_mode.set_label(&crate::i18n::t("mode"));
                lbl_basket.set_label(&crate::i18n::t("basket"));
                btn_center_basket.set_label(&crate::i18n::t("center_basket"));
                lbl_basket_sub.set_label(&crate::i18n::t("basket_desc"));
                lbl_lang.set_label(&crate::i18n::t("language"));
                card_sys_title.set_label(&crate::i18n::t("section_system"));
                autostart_check.set_label(Some(&crate::i18n::t("autostart")));
                lbl_twitch.set_label(&crate::i18n::t("twitch_channel"));
                twitch_entry.set_placeholder_text(Some(&crate::i18n::t("twitch_placeholder")));
                close_btn.set_label(&crate::i18n::t("close"));
                to_advanced_btn.set_label(&crate::i18n::t("advanced_mode"));
                back_btn.set_label(&crate::i18n::t("back_to_settings"));
                save_btn.set_label(&crate::i18n::t("save_mapping"));

                if let Some(item) = skin_drop.selected_item() {
                    if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                        editor_title.set_text(&format!("{} : {}", crate::i18n::t("sprite_editor_tab"), string_obj.string()));
                    }
                }

                updating_ui_lang.set(false);
            }
        });
    }

    {
        let control = control.clone();
        twitch_entry.connect_activate(move |entry| {
            let mut ctrl = control.lock().unwrap();
            ctrl.twitch_channel = entry.text().to_string();
            ctrl.version += 1;
        });
    }

    {
        let control = control.clone();
        let toys = toys.clone();
        toy_drop.connect_selected_notify(move |drop| {
            let idx = drop.selected() as usize;
            if idx < toys.len() {
                let mut ctrl = control.lock().unwrap();
                ctrl.toy = toys[idx].clone();
                ctrl.version += 1;
            }
        });
    }

    // Changement de Skin (recharge aussi les sprites de l'éditeur)
    {
        let mapping = mapping.clone();
        let json_path = json_path.clone();
        let editor_title = editor_title.clone();
        let sheet = sheet.clone();
        let control = control.clone();
        let cells = cells.clone();
        let assets = assets.clone();

        skin_drop.connect_selected_notify(move |drop| {
            if let Some(item) = drop.selected_item() {
                if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                    let new_skin = string_obj.string().to_string();

                    *sheet.borrow_mut() = crate::load_sprite(&assets.join("pets").join(format!("{new_skin}.png")));
                    let new_json = assets.join("pets").join(format!("{new_skin}.json"));
                    *mapping.borrow_mut() = load_or_default(&new_json);
                    *json_path.borrow_mut() = new_json.clone();

                    {
                        let mut ctrl = control.lock().unwrap();
                        if ctrl.skin != new_skin {
                            ctrl.skin = new_skin.clone();
                            ctrl.version += 1;
                        }
                    }

                    editor_title.set_text(&format!("{} : {}", crate::i18n::t("sprite_editor_tab"), new_skin));
                    for da in cells.borrow().iter() { da.queue_draw(); }
                }
            }
        });
    }

    // Sauvegarde du mapping JSON
    {
        let mapping = mapping.clone();
        let json_path = json_path.clone();
        let save_btn = save_btn.clone();
        save_btn.connect_clicked(move |btn| {
            let m = mapping.borrow();
            let out: HashMap<&String, Vec<[i32; 2]>> = m
                .iter()
                .map(|(k, v)| (k, v.iter().map(|p| [p.0, p.1]).collect()))
                .collect();
            match serde_json::to_string_pretty(&out).map(|s| std::fs::write(&*json_path.borrow(), s)) {
                Ok(Ok(())) => btn.set_label(&crate::i18n::t("saved")),
                _ => btn.set_label(&crate::i18n::t("failed")),
            }

            let b2 = btn.clone();
            glib::timeout_add_seconds_local(2, move || {
                b2.set_label(&crate::i18n::t("save_mapping"));
                glib::ControlFlow::Break
            });
        });
    }

    // Fermeture avec Échap
    {
        let window = window.clone();
        close_btn.connect_clicked(move |_| window.close());
    }
    let keys = gtk::EventControllerKey::new();
    {
        let window = window.clone();
        keys.connect_key_pressed(move |_c, key, _code, _state| {
            if key == gtk::gdk::Key::Escape {
                window.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    window.add_controller(keys);

    window.present();
}

fn load_or_default(path: &Path) -> Mapping {
    let raw: HashMap<String, Vec<[i32; 2]>> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .or_else(|| serde_json::from_str(crate::pet::DEFAULT_MAPPING).ok())
        .unwrap_or_default();
    raw.into_iter()
        .map(|(k, v)| (k, v.into_iter().map(|a| (a[0], a[1])).collect()))
        .collect()
}
