//! Tableau de bord de configuration (Dashboard), natif GTK4 (plein écran).
//!
//! Permet de configurer le comportement global (mode, taille, pelote) et d'éditer
//! le mapping des sprites du skin actif.

use std::cell::RefCell;

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
    Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, StringList,
};
use crate::config::Control;

const COLS: i32 = 8;
const ROWS: i32 = 6;
const STRIDE: i32 = 33; // tuile 32 + 1px de marge
const TILE: i32 = 32;
const CELL: f64 = 3.0; // agrandissement d'une cellule

const ANIMS: &[&str] = &[
    "idle", "alert", "tired", "sleeping", "scratchSelf", "scratchWallN", "scratchWallS",
    "scratchWallW", "scratchWallE", "N", "NE", "E", "SE", "S", "SW", "W", "NW",
];

const MODES: &[&str] = &["Pelote", "Autonome", "Souris", "Sommeil"];
const SCALES: &[(&str, f64)] = &[("1.0x", 1.0), ("1.5x", 1.5), ("2.0x", 2.0), ("3.0x", 3.0)];

type Mapping = HashMap<String, Vec<(i32, i32)>>;

// Helper pour dessiner un rectangle arrondi
fn rounded_rect(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}

pub fn open(app: &Application, assets: PathBuf, control: Arc<Mutex<Control>>) {
    let (initial_skin, initial_toy, initial_mode, initial_scale, initial_twitch) = {
        let c = control.lock().unwrap();
        (c.skin.clone(), c.toy.clone(), c.mode.clone(), c.scale, c.twitch_channel.clone())
    };

    let json_path = Rc::new(RefCell::new(assets.join("pets").join(format!("{initial_skin}.json"))));
    let mapping: Rc<RefCell<Mapping>> = Rc::new(RefCell::new(load_or_default(&json_path.borrow())));
    let active: Rc<RefCell<String>> = Rc::new(RefCell::new(ANIMS[0].to_string()));

    let sheet = Rc::new(RefCell::new(crate::load_sprite(&assets.join("pets").join(format!("{initial_skin}.png")))));

    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        r#"
        window.dashboard {
            background-color: #0b1019;
            color: #e6edf3;
            font-size: 14px;
        }
        .dashboard .sidebar {
            background-color: #0f1622;
            border-right: 1px solid #1e2a3a;
            padding: 28px 22px;
        }
        .dashboard .sidebar-title {
            font-size: 22px;
            font-weight: 800;
            color: #e6edf3;
            margin-bottom: 26px;
        }
        .dashboard .section-label {
            font-size: 12px;
            font-weight: 600;
            letter-spacing: 1px;
            color: #6b7a8d;
            margin-top: 18px;
            margin-bottom: 8px;
        }

        /* Champs (dropdowns + entry) : surface sombre, accent au survol/focus */
        .dashboard dropdown {
            margin-bottom: 14px;
            border: none;
            background: transparent;
        }
        .dashboard dropdown > button,
        .dashboard entry {
            background-image: none;
            background-color: #16202e;
            color: #e6edf3;
            border-radius: 10px;
            border: 1px solid #263448;
            padding: 11px 14px;
            min-height: 20px;
            transition: border-color 0.15s ease, background-color 0.15s ease, box-shadow 0.15s ease;
        }
        .dashboard dropdown > button:hover,
        .dashboard entry:hover {
            background-color: #1b2738;
            border-color: #34465f;
        }
        .dashboard dropdown > button:focus,
        .dashboard entry:focus-within {
            border-color: #3b82f6;
            box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.18);
        }
        .dashboard entry {
            margin-bottom: 14px;
            caret-color: #3b82f6;
        }
        .dashboard entry text { color: #e6edf3; }
        .dashboard entry > text > placeholder { color: #5b6b7e; }

        .dashboard dropdown popover,
        .dashboard dropdown popover > contents {
            background-color: #16202e;
            color: #e6edf3;
            border: 1px solid #263448;
            border-radius: 12px;
        }
        .dashboard dropdown popover row {
            border-radius: 8px;
            padding: 8px 10px;
        }
        .dashboard dropdown popover row:selected {
            background-color: #3b82f6;
            color: white;
        }

        /* Boutons d'action */
        .dashboard button.primary-btn {
            background-image: none;
            background-color: #3b82f6;
            color: white;
            border-radius: 10px;
            padding: 13px 20px;
            font-weight: 700;
            border: none;
            transition: background-color 0.15s ease, box-shadow 0.15s ease;
        }
        .dashboard button.primary-btn:hover {
            background-color: #2f6fe0;
            box-shadow: 0 6px 18px rgba(59, 130, 246, 0.35);
        }
        .dashboard button.close-btn {
            background-image: none;
            background-color: transparent;
            border: 1px solid #3a2230;
            color: #f87171;
            border-radius: 10px;
            padding: 11px 20px;
            font-weight: 600;
            margin-top: 12px;
            transition: all 0.15s ease;
        }
        .dashboard button.close-btn:hover {
            background-color: #ef4444;
            border-color: #ef4444;
            color: white;
        }

        .dashboard .main-area { padding: 28px; }
        .dashboard .main-title {
            font-size: 18px;
            font-weight: 700;
            color: #e6edf3;
            margin-bottom: 22px;
        }
        .dashboard list {
            background-color: transparent;
            padding: 4px;
        }
        .dashboard row {
            padding: 11px 14px;
            border-radius: 8px;
            margin-bottom: 3px;
            color: #aeb9c7;
            font-weight: 500;
            transition: all 0.12s ease;
        }
        .dashboard row:selected {
            background-color: #3b82f6;
            color: white;
            font-weight: 600;
        }
        .dashboard row:hover:not(:selected) {
            background-color: #1b2738;
            color: #e6edf3;
        }
        .dashboard scrolledwindow {
            border: 1px solid #1e2a3a;
            border-radius: 14px;
            background-color: #0f1622;
        }
        "#,
    );
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Neko Dashboard")
        .build();
    window.add_css_class("dashboard");
    #[cfg(target_os = "windows")]
    {
        // Sur Windows : fenêtre normale avec décorations et redimensionnable
        window.set_decorated(true);
        window.set_resizable(true);
        window.set_default_size(900, 650);
    }
    #[cfg(not(target_os = "windows"))]
    window.fullscreen();

    let root = GtkBox::new(Orientation::Horizontal, 0);

    // ==========================================
    // SIDEBAR (Configuration globale)
    // ==========================================
    let sidebar = GtkBox::new(Orientation::Vertical, 0);
    sidebar.add_css_class("sidebar");
    sidebar.set_size_request(300, -1);

    let app_title = Label::new(Some(&crate::i18n::t("app_title")));
    app_title.add_css_class("sidebar-title");
    app_title.set_halign(gtk::Align::Start);
    sidebar.append(&app_title);

    // -- Mode --
    let l_mode = Label::new(Some(&crate::i18n::t("mode")));
    l_mode.add_css_class("section-label");
    l_mode.set_halign(gtk::Align::Start);
    sidebar.append(&l_mode);
    
    let mode_list = StringList::new(MODES);
    let mode_drop = DropDown::new(Some(mode_list), gtk::Expression::NONE);
    if let Some(pos) = MODES.iter().position(|&m| m == initial_mode) {
        mode_drop.set_selected(pos as u32);
    }
    sidebar.append(&mode_drop);

    // -- Canal Twitch (mode streamer) --
    let l_twitch = Label::new(Some(&crate::i18n::t("twitch_channel")));
    l_twitch.add_css_class("section-label");
    l_twitch.set_halign(gtk::Align::Start);
    sidebar.append(&l_twitch);

    let twitch_entry = Entry::new();
    twitch_entry.set_placeholder_text(Some(&crate::i18n::t("twitch_placeholder")));
    twitch_entry.set_text(&initial_twitch);
    sidebar.append(&twitch_entry);

    // -- Skin --
    let l_skin = Label::new(Some(&crate::i18n::t("skin")));
    l_skin.add_css_class("section-label");
    l_skin.set_halign(gtk::Align::Start);
    sidebar.append(&l_skin);
    
    let skins = crate::list_pngs(&assets.join("pets"));
    let skins_ref: Vec<&str> = skins.iter().map(|s| s.as_str()).collect();
    let skin_list = StringList::new(&skins_ref);
    
    let mut skin_textures = std::collections::HashMap::new();
    for skin in &skins {
        let path = assets.join("pets").join(format!("{skin}.png"));
        if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(&path) {
            if pixbuf.width() >= 32 && pixbuf.height() >= 32 {
                let sub = pixbuf.new_subpixbuf(0, 0, 32, 32);
                skin_textures.insert(skin.clone(), gtk::gdk::Texture::for_pixbuf(&sub));
            }
        }
    }
    let skin_textures = std::rc::Rc::new(skin_textures);

    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, obj| {
        let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
        let hbox = GtkBox::new(Orientation::Horizontal, 8);
        let img = gtk::Image::new();
        img.set_pixel_size(32);
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
    sidebar.append(&skin_drop);

    // -- Toy --
    let l_toy = Label::new(Some(&crate::i18n::t("toy")));
    l_toy.add_css_class("section-label");
    l_toy.set_halign(gtk::Align::Start);
    sidebar.append(&l_toy);
    
    let toys = crate::list_pngs(&assets.join("toys"));
    let toys_ref: Vec<&str> = toys.iter().map(|s| s.as_str()).collect();
    let toy_list = StringList::new(&toys_ref);

    let mut toy_textures = std::collections::HashMap::new();
    for toy in &toys {
        let path = assets.join("toys").join(format!("{toy}.png"));
        if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(&path) {
            if pixbuf.width() >= 32 && pixbuf.height() >= 32 {
                let sub = pixbuf.new_subpixbuf(0, 0, 32, 32);
                toy_textures.insert(toy.clone(), gtk::gdk::Texture::for_pixbuf(&sub));
            }
        }
    }
    let toy_textures = std::rc::Rc::new(toy_textures);

    let toy_factory = gtk::SignalListItemFactory::new();
    toy_factory.connect_setup(|_, obj| {
        let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
        let hbox = GtkBox::new(Orientation::Horizontal, 8);
        let img = gtk::Image::new();
        img.set_pixel_size(32);
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
    sidebar.append(&toy_drop);

    // -- Scale --
    let l_scale = Label::new(Some(&crate::i18n::t("scale")));
    l_scale.add_css_class("section-label");
    l_scale.set_halign(gtk::Align::Start);
    sidebar.append(&l_scale);
    
    let scale_strs: Vec<&str> = SCALES.iter().map(|(s, _)| *s).collect();
    let scale_list = StringList::new(&scale_strs);
    let scale_drop = DropDown::new(Some(scale_list), gtk::Expression::NONE);
    if let Some(pos) = SCALES.iter().position(|(_, v)| (v - initial_scale).abs() < 0.1) {
        scale_drop.set_selected(pos as u32);
    }
    sidebar.append(&scale_drop);

    // -- Démarrage avec Windows (Windows uniquement) --
    #[cfg(target_os = "windows")]
    {
        let autostart_check = gtk::CheckButton::with_label("Lancer au démarrage de Windows");
        autostart_check.set_active(autostart_get());
        autostart_check.set_margin_top(12);
        autostart_check.connect_toggled(|btn| {
            autostart_set(btn.is_active());
        });
        sidebar.append(&autostart_check);
    }

    // Spacer
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    sidebar.append(&spacer);

    let save_btn = Button::with_label(&crate::i18n::t("save_mapping"));
    save_btn.add_css_class("primary-btn");
    sidebar.append(&save_btn);

    let close_btn = Button::with_label(&crate::i18n::t("close"));
    close_btn.add_css_class("close-btn");
    sidebar.append(&close_btn);

    let credits = Label::new(Some(&crate::i18n::t("credits")));
    credits.set_wrap(true);
    // Borne la largeur « naturelle » : sans ça, un label wrap réclame la largeur
    // de tout son texte sur une ligne → la sidebar gonflerait pour le contenir.
    credits.set_max_width_chars(34);
    credits.set_xalign(0.0);
    credits.set_opacity(0.6);
    credits.add_css_class("credits-label");
    credits.set_margin_top(16);
    sidebar.append(&credits);

    root.append(&sidebar);

    // ==========================================
    // MAIN AREA (Éditeur de Sprites)
    // ==========================================
    let main_area = GtkBox::new(Orientation::Vertical, 0);
    main_area.add_css_class("main-area");
    main_area.set_hexpand(true);
    main_area.set_vexpand(true);

    let title = Label::new(Some(&format!("{} : {}", crate::i18n::t("sprite_editor"), initial_skin)));
    title.add_css_class("main-title");
    title.set_halign(gtk::Align::Start);
    main_area.append(&title);

    let body = GtkBox::new(Orientation::Horizontal, 20);
    body.set_vexpand(true);

    let grid = Grid::new();
    grid.set_row_spacing(8);
    grid.set_column_spacing(8);
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
                    rounded_rect(cr, 0.0, 0.0, w as f64, h as f64, 8.0);
                    cr.set_source_rgb(0.12, 0.16, 0.23); 
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
                        rounded_rect(cr, 4.0, h as f64 - badge_h - 4.0, w as f64 - 8.0, badge_h, 6.0);
                        cr.set_source_rgba(0.23, 0.51, 0.96, 0.9);
                        let _ = cr.fill();
                        
                        cr.set_source_rgb(1.0, 1.0, 1.0);
                        cr.set_font_size(10.0);
                        
                        let text = names.join(",");
                        if let Ok(te) = cr.text_extents(&text) {
                            let tx = (w as f64 - te.width()) / 2.0;
                            cr.move_to(tx, h as f64 - 8.0);
                            let _ = cr.show_text(&text);
                        }
                    }
                });
            }
            let click = gtk::GestureClick::new();
            click.set_button(0); // Listen to all mouse buttons
            {
                let mapping = mapping.clone();
                let active = active.clone();
                let cells = cells.clone();
                click.connect_pressed(move |g, _n, _x, _y| {
                    if g.current_button() == 3 {
                        // Right click: unbind ALL animations for this cell
                        let mut m = mapping.borrow_mut();
                        for v in m.values_mut() {
                            v.retain(|p| *p != (c, r));
                        }
                    } else if g.current_button() == 1 {
                        // Left click: toggle the currently active animation for this cell
                        let a = active.borrow().clone();
                        let mut m = mapping.borrow_mut();
                        let v = m.entry(a).or_default();
                        match v.iter().position(|p| *p == (c, r)) {
                            Some(i) => { v.remove(i); }
                            None => v.push((c, r)),
                        }
                    }
                    for da in cells.borrow().iter() { da.queue_draw(); }
                });
            }
            da.add_controller(click);
            grid.attach(&da, c, r, 1, 1);
            cells.borrow_mut().push(da);
        }
    }
    body.append(&grid);

    // Liste des animations à droite.
    let list = ListBox::new();
    for name in ANIMS {
        let row = ListBoxRow::new();
        row.set_child(Some(&Label::new(Some(name))));
        list.append(&row);
    }
    list.select_row(list.row_at_index(0).as_ref());
    {
        let active = active.clone();
        list.connect_row_selected(move |_l, row| {
            if let Some(row) = row {
                if let Some(name) = ANIMS.get(row.index() as usize) {
                    *active.borrow_mut() = name.to_string();
                }
            }
        });
    }
    let scroll = ScrolledWindow::new();
    scroll.set_child(Some(&list));
    scroll.set_min_content_width(200);
    scroll.set_margin_start(10);
    body.append(&scroll);

    main_area.append(&body);
    root.append(&main_area);
    window.set_child(Some(&root));

    // ==========================================
    // ACTIONS & EVENTS
    // ==========================================

    // Changement de Mode
    {
        let control = control.clone();
        mode_drop.connect_selected_notify(move |drop| {
            if let Some(item) = drop.selected_item() {
                if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                    let mut c = control.lock().unwrap();
                    c.mode = string_obj.string().to_string();
                }
            }
        });
    }

    // Changement de canal Twitch — appliqué sur Entrée ou perte de focus, pas à
    // chaque frappe (sinon le superviseur tenterait de se connecter à un nom partiel).
    {
        let control = control.clone();
        twitch_entry.connect_activate(move |e| {
            control.lock().unwrap().twitch_channel = e.text().trim().to_string();
        });
    }
    {
        let control = control.clone();
        let entry = twitch_entry.clone();
        let focus = gtk::EventControllerFocus::new();
        focus.connect_leave(move |_| {
            control.lock().unwrap().twitch_channel = entry.text().trim().to_string();
        });
        twitch_entry.add_controller(focus);
    }

    // Changement de Pelote
    {
        let control = control.clone();
        toy_drop.connect_selected_notify(move |drop| {
            if let Some(item) = drop.selected_item() {
                if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                    let mut c = control.lock().unwrap();
                    c.toy = string_obj.string().to_string();
                    c.version += 1;
                }
            }
        });
    }

    // Changement de Scale
    {
        let control = control.clone();
        scale_drop.connect_selected_notify(move |drop| {
            if let Some(item) = drop.selected_item() {
                if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                    let mut c = control.lock().unwrap();
                    let s_str = string_obj.string();
                    if let Some((_, val)) = SCALES.iter().find(|(s, _)| s == &s_str) {
                        c.scale = *val;
                        c.version += 1;
                    }
                }
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
                    
                    title.set_text(&format!("{} : {}", crate::i18n::t("sprite_editor"), new_skin));
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
            
            // Remettre le label original après 2s
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
