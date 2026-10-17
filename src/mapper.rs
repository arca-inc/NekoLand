//! Tableau de bord de configuration (Dashboard), natif GTK4 épuré et moderne.
//!
//! Permet de configurer le comportement global (mode, apparence, pelote, échelle, langue)
//! et d'éditer le mapping des sprites du skin actif.

use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gtk::gdk::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, DropDown, Entry, Grid,
    Label, Orientation, ScrolledWindow, StringList, CheckButton,
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
            if let Ok(exe) = std::env::current_exe() {
                let content = format!(
                    "[Desktop Entry]\nType=Application\nName=NekoLand\nComment=Desktop companion\nExec={}\nIcon=face-smile\nTerminal=false\nCategories=Game;\nX-GNOME-Autostart-enabled=true\n",
                    exe.display()
                );
                let _ = std::fs::write(file, content);
            }
        } else {
            let _ = std::fs::remove_file(file);
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
const CELL: f64 = 2.8; // agrandissement d'une cellule

const ANIMS: &[&str] = &[
    "idle", "alert", "tired", "sleeping", "scratchSelf", "scratchWallN", "scratchWallS",
    "scratchWallW", "scratchWallE", "N", "NE", "E", "SE", "S", "SW", "W", "NW",
];

const MODES: &[&str] = &["Pelote", "Autonome", "Souris", "Sommeil"];
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
    let (initial_skin, initial_toy, initial_mode, initial_scale, initial_twitch, initial_lang) = {
        let c = control.lock().unwrap();
        (c.skin.clone(), c.toy.clone(), c.mode.clone(), c.scale, c.twitch_channel.clone(), c.lang.clone())
    };

    let json_path = Rc::new(RefCell::new(assets.join("pets").join(format!("{initial_skin}.json"))));
    let mapping: Rc<RefCell<Mapping>> = Rc::new(RefCell::new(load_or_default(&json_path.borrow())));
    let active: Rc<RefCell<String>> = Rc::new(RefCell::new(ANIMS[0].to_string()));

    let sheet = Rc::new(RefCell::new(crate::load_sprite(&assets.join("pets").join(format!("{initial_skin}.png")))));

    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        r#"
        window.dashboard {
            background-color: #0d1117;
            color: #c9d1d9;
            font-size: 13px;
        }
        .dashboard .sidebar {
            background-color: #161b22;
            border-right: 1px solid #30363d;
            padding: 24px 20px;
        }
        .dashboard .sidebar-title {
            font-size: 20px;
            font-weight: 800;
            color: #f0f6fc;
            margin-bottom: 4px;
        }
        .dashboard .sidebar-subtitle {
            font-size: 12px;
            color: #8b949e;
            margin-bottom: 20px;
        }
        .dashboard .section-label {
            font-size: 11px;
            font-weight: 700;
            letter-spacing: 0.8px;
            text-transform: uppercase;
            color: #8b949e;
            margin-top: 14px;
            margin-bottom: 6px;
        }
        .dashboard dropdown {
            margin-bottom: 8px;
            background: transparent;
            border: none;
        }
        .dashboard dropdown > button,
        .dashboard entry {
            background-color: #0d1117;
            color: #f0f6fc;
            border-radius: 8px;
            border: 1px solid #30363d;
            padding: 8px 12px;
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
        .dashboard entry {
            margin-bottom: 8px;
            caret-color: #58a6ff;
        }
        .dashboard entry text { color: #f0f6fc; }
        .dashboard entry > text > placeholder { color: #6e7681; }

        .dashboard dropdown popover,
        .dashboard dropdown popover > contents {
            background-color: #161b22;
            color: #f0f6fc;
            border: 1px solid #30363d;
            border-radius: 10px;
        }
        .dashboard dropdown popover row {
            border-radius: 6px;
            padding: 7px 10px;
        }
        .dashboard dropdown popover row:selected {
            background-color: #1f6feb;
            color: #ffffff;
        }
        .dashboard checkbutton {
            color: #c9d1d9;
            font-size: 12px;
            margin-top: 8px;
        }
        .dashboard .hint-banner {
            background-color: rgba(56, 139, 253, 0.1);
            border: 1px solid rgba(56, 139, 253, 0.3);
            border-radius: 8px;
            padding: 10px 12px;
            margin-top: 14px;
            margin-bottom: 6px;
            color: #79c0ff;
            font-size: 11.5px;
            line-height: 1.4;
        }
        .dashboard button.primary-btn {
            background-color: #238636;
            color: #ffffff;
            border-radius: 8px;
            padding: 10px 16px;
            font-weight: 700;
            border: 1px solid rgba(240, 246, 252, 0.1);
            margin-top: 12px;
            transition: all 0.15s ease;
        }
        .dashboard button.primary-btn:hover {
            background-color: #2ea043;
            box-shadow: 0 4px 12px rgba(46, 160, 67, 0.3);
        }
        .dashboard button.close-btn {
            background-color: transparent;
            border: 1px solid #30363d;
            color: #f85149;
            border-radius: 8px;
            padding: 9px 16px;
            font-weight: 600;
            margin-top: 8px;
            transition: all 0.15s ease;
        }
        .dashboard button.close-btn:hover {
            background-color: #da3633;
            border-color: #da3633;
            color: #ffffff;
        }
        .dashboard .main-area {
            padding: 24px 28px;
            background-color: #0d1117;
        }
        .dashboard .main-title {
            font-size: 18px;
            font-weight: 700;
            color: #f0f6fc;
            margin-bottom: 16px;
        }
        .dashboard list {
            background-color: transparent;
            padding: 2px;
        }
        .dashboard row {
            padding: 8px 12px;
            border-radius: 6px;
            margin-bottom: 3px;
            color: #8b949e;
            font-weight: 500;
            transition: all 0.1s ease;
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
        .dashboard .credits-label {
            font-size: 11px;
            color: #6e7681;
            margin-top: 14px;
        }
        "#,
    );

    let display = gtk::gdk::Display::default().unwrap();
    gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    let window = ApplicationWindow::builder()
        .application(app)
        .title(&crate::i18n::t("dashboard_title"))
        .build();
    window.add_css_class("dashboard");
    window.set_decorated(true);
    window.set_resizable(true);
    window.set_default_size(960, 680);

    let root = GtkBox::new(Orientation::Horizontal, 0);

    // ==========================================
    // SIDEBAR (Paramètres généraux)
    // ==========================================
    let sidebar = GtkBox::new(Orientation::Vertical, 0);
    sidebar.add_css_class("sidebar");
    sidebar.set_size_request(310, -1);

    let sidebar_scroll = ScrolledWindow::new();
    sidebar_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    sidebar_scroll.set_hexpand(false);
    sidebar_scroll.set_vexpand(true);

    let sidebar_content = GtkBox::new(Orientation::Vertical, 0);

    let app_title = Label::new(Some(&crate::i18n::t("app_title")));
    app_title.add_css_class("sidebar-title");
    app_title.set_halign(gtk::Align::Start);
    sidebar_content.append(&app_title);

    let app_sub = Label::new(Some("NekoLand v0.1.0"));
    app_sub.add_css_class("sidebar-subtitle");
    app_sub.set_halign(gtk::Align::Start);
    sidebar_content.append(&app_sub);

    // -- Langue --
    let l_lang = Label::new(Some(&crate::i18n::t("language")));
    l_lang.add_css_class("section-label");
    l_lang.set_halign(gtk::Align::Start);
    sidebar_content.append(&l_lang);

    let lang_names: Vec<&str> = LANGS.iter().map(|(n, _)| *n).collect();
    let lang_list = StringList::new(&lang_names);
    let lang_drop = DropDown::new(Some(lang_list), gtk::Expression::NONE);
    let cur_l = if initial_lang.is_empty() { crate::i18n::current_lang() } else { initial_lang.as_str() };
    if let Some(pos) = LANGS.iter().position(|(_, c)| *c == cur_l) {
        lang_drop.set_selected(pos as u32);
    }
    sidebar_content.append(&lang_drop);

    // -- Mode --
    let l_mode = Label::new(Some(&crate::i18n::t("mode")));
    l_mode.add_css_class("section-label");
    l_mode.set_halign(gtk::Align::Start);
    sidebar_content.append(&l_mode);

    let mode_list = StringList::new(MODES);
    let mode_drop = DropDown::new(Some(mode_list), gtk::Expression::NONE);
    if let Some(pos) = MODES.iter().position(|&m| m == initial_mode) {
        mode_drop.set_selected(pos as u32);
    }
    sidebar_content.append(&mode_drop);

    // -- Skin --
    let l_skin = Label::new(Some(&crate::i18n::t("skin")));
    l_skin.add_css_class("section-label");
    l_skin.set_halign(gtk::Align::Start);
    sidebar_content.append(&l_skin);

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

    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, obj| {
        let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
        let hbox = GtkBox::new(Orientation::Horizontal, 8);
        let img = gtk::Image::new();
        img.set_pixel_size(24);
        let lbl = Label::new(None);
        hbox.append(&img);
        hbox.append(&lbl);
        list_item.set_child(Some(&hbox));
    });

    let skin_textures_bind = skin_textures.clone();
    factory.connect_bind(move |_, obj| {
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
    skin_drop.set_factory(Some(&factory));
    skin_drop.set_list_factory(Some(&factory));

    if let Some(pos) = skins.iter().position(|s| s == &initial_skin) {
        skin_drop.set_selected(pos as u32);
    }
    sidebar_content.append(&skin_drop);

    // -- Toy --
    let l_toy = Label::new(Some(&crate::i18n::t("toy")));
    l_toy.add_css_class("section-label");
    l_toy.set_halign(gtk::Align::Start);
    sidebar_content.append(&l_toy);

    let toys = crate::list_pngs(&assets.join("toys"));
    let toys_ref: Vec<&str> = toys.iter().map(|s| s.as_str()).collect();
    let toy_list = StringList::new(&toys_ref);

    let mut toy_textures = HashMap::new();
    for toy in &toys {
        let path = assets.join("toys").join(format!("{toy}.png"));
        if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(&path) {
            if pixbuf.width() >= 32 && pixbuf.height() >= 32 {
                let sub = pixbuf.new_subpixbuf(0, 0, 32, 32);
                toy_textures.insert(toy.clone(), gtk::gdk::Texture::for_pixbuf(&sub));
            }
        }
    }
    let toy_textures = Rc::new(toy_textures);

    let toy_factory = gtk::SignalListItemFactory::new();
    toy_factory.connect_setup(|_, obj| {
        let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
        let hbox = GtkBox::new(Orientation::Horizontal, 8);
        let img = gtk::Image::new();
        img.set_pixel_size(24);
        let lbl = Label::new(None);
        hbox.append(&img);
        hbox.append(&lbl);
        list_item.set_child(Some(&hbox));
    });

    let toy_textures_bind = toy_textures.clone();
    toy_factory.connect_bind(move |_, obj| {
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
    toy_drop.set_factory(Some(&toy_factory));
    toy_drop.set_list_factory(Some(&toy_factory));

    if let Some(pos) = toys.iter().position(|s| s == &initial_toy) {
        toy_drop.set_selected(pos as u32);
    }
    sidebar_content.append(&toy_drop);

    // -- Scale --
    let l_scale = Label::new(Some(&crate::i18n::t("scale")));
    l_scale.add_css_class("section-label");
    l_scale.set_halign(gtk::Align::Start);
    sidebar_content.append(&l_scale);

    let scale_strs: Vec<&str> = SCALES.iter().map(|(s, _)| *s).collect();
    let scale_list = StringList::new(&scale_strs);
    let scale_drop = DropDown::new(Some(scale_list), gtk::Expression::NONE);
    if let Some(pos) = SCALES.iter().position(|(_, v)| (v - initial_scale).abs() < 0.1) {
        scale_drop.set_selected(pos as u32);
    }
    sidebar_content.append(&scale_drop);

    // -- Twitch --
    let l_twitch = Label::new(Some(&crate::i18n::t("twitch_channel")));
    l_twitch.add_css_class("section-label");
    l_twitch.set_halign(gtk::Align::Start);
    sidebar_content.append(&l_twitch);

    let twitch_entry = Entry::new();
    twitch_entry.set_placeholder_text(Some(&crate::i18n::t("twitch_placeholder")));
    twitch_entry.set_text(&initial_twitch);
    sidebar_content.append(&twitch_entry);

    // -- Autostart Checkbox --
    let autostart_check = CheckButton::with_label(&crate::i18n::t("autostart"));
    autostart_check.set_active(autostart_get());
    autostart_check.connect_toggled(|btn| {
        autostart_set(btn.is_active());
    });
    sidebar_content.append(&autostart_check);

    // -- Hint Banner (Drag & drop) --
    let hint_banner = Label::new(Some(&crate::i18n::t("drag_hint")));
    hint_banner.add_css_class("hint-banner");
    hint_banner.set_wrap(true);
    hint_banner.set_max_width_chars(28);
    hint_banner.set_xalign(0.0);
    sidebar_content.append(&hint_banner);

    // Spacer
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    sidebar_content.append(&spacer);

    let save_btn = Button::with_label(&crate::i18n::t("save_mapping"));
    save_btn.add_css_class("primary-btn");
    sidebar_content.append(&save_btn);

    let close_btn = Button::with_label(&crate::i18n::t("close"));
    close_btn.add_css_class("close-btn");
    sidebar_content.append(&close_btn);

    let credits = Label::new(Some(&crate::i18n::t("credits")));
    credits.set_wrap(true);
    credits.set_max_width_chars(32);
    credits.set_xalign(0.0);
    credits.set_opacity(0.6);
    credits.add_css_class("credits-label");
    sidebar_content.append(&credits);

    sidebar_scroll.set_child(Some(&sidebar_content));
    sidebar.append(&sidebar_scroll);
    root.append(&sidebar);

    // ==========================================
    // MAIN AREA (Éditeur de Sprites)
    // ==========================================
    let main_area = GtkBox::new(Orientation::Vertical, 0);
    main_area.add_css_class("main-area");
    main_area.set_hexpand(true);
    main_area.set_vexpand(true);

    let title = Label::new(Some(&format!("{} : {}", crate::i18n::t("sprite_editor_tab"), initial_skin)));
    title.add_css_class("main-title");
    title.set_halign(gtk::Align::Start);
    main_area.append(&title);

    let body = GtkBox::new(Orientation::Horizontal, 20);
    body.set_vexpand(true);

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
    body.append(&grid);

    // Panneau de droite : sélecteur d'animation + preview animée
    let side_anims = GtkBox::new(Orientation::Vertical, 12);
    side_anims.set_size_request(190, -1);

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

    let scroll = ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_vexpand(true);

    let list = gtk::ListBox::new();
    for anim in ANIMS {
        let row = gtk::ListBoxRow::new();
        let lbl = Label::new(Some(anim));
        lbl.set_halign(gtk::Align::Start);
        row.set_child(Some(&lbl));
        list.append(&row);
    }
    list.select_row(list.row_at_index(0).as_ref());

    {
        let active = active.clone();
        list.connect_row_selected(move |_, row| {
            if let Some(r) = row {
                let idx = r.index() as usize;
                if idx < ANIMS.len() {
                    *active.borrow_mut() = ANIMS[idx].to_string();
                }
            }
        });
    }
    scroll.set_child(Some(&list));
    side_anims.append(&scroll);
    body.append(&side_anims);
    main_area.append(&body);
    root.append(&main_area);
    window.set_child(Some(&root));

    // Timer d'animation de la prévisualisation
    {
        let preview_area = preview_area.clone();
        let preview_tick = preview_tick.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            *preview_tick.borrow_mut() += 1;
            preview_area.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    // Gestion du clic sur les cellules de la grille
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

    // Changement de Mode
    {
        let control = control.clone();
        mode_drop.connect_selected_notify(move |drop| {
            let idx = drop.selected() as usize;
            if idx < MODES.len() {
                let mut ctrl = control.lock().unwrap();
                ctrl.mode = MODES[idx].to_string();
                ctrl.version += 1;
            }
        });
    }

    // Changement de Taille
    {
        let control = control.clone();
        scale_drop.connect_selected_notify(move |drop| {
            let idx = drop.selected() as usize;
            if idx < SCALES.len() {
                let mut ctrl = control.lock().unwrap();
                ctrl.scale = SCALES[idx].1;
                ctrl.version += 1;
            }
        });
    }

    // Changement de Langue
    {
        let control = control.clone();
        lang_drop.connect_selected_notify(move |drop| {
            let idx = drop.selected() as usize;
            if idx < LANGS.len() {
                let code = LANGS[idx].1;
                crate::i18n::set_lang(code);
                let mut ctrl = control.lock().unwrap();
                ctrl.lang = code.to_string();
                ctrl.version += 1;
            }
        });
    }

    // Changement de Canal Twitch
    {
        let control = control.clone();
        twitch_entry.connect_activate(move |entry| {
            let mut ctrl = control.lock().unwrap();
            ctrl.twitch_channel = entry.text().to_string();
            ctrl.version += 1;
        });
    }

    // Changement de Pelote
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

    // Changement de Skin
    {
        let mapping = mapping.clone();
        let json_path = json_path.clone();
        let title = title.clone();
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

                    title.set_text(&format!("{} : {}", crate::i18n::t("sprite_editor_tab"), new_skin));
                    for da in cells.borrow().iter() { da.queue_draw(); }
                }
            }
        });
    }

    // Bouton Sauvegarder
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

    // Bouton Fermer & Échap
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
