//! Éditeur de mapping des sprites, natif GTK4 (plein écran, même process).
//!
//! Affiche la grille 8×6 du sheet du skin courant ; on choisit une animation à
//! droite puis on clique les cellules (dans l'ordre des frames). « Enregistrer »
//! écrit `<assets>/pets/<skin>.json` — la boucle principale recharge le mapping à
//! chaud (surveillance du mtime).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use gtk::gdk::prelude::*;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, Grid, Label, ListBox,
    ListBoxRow, Orientation, ScrolledWindow,
};

const COLS: i32 = 8;
const ROWS: i32 = 6;
const STRIDE: i32 = 33; // tuile 32 + 1px de marge
const TILE: i32 = 32;
const CELL: f64 = 3.0; // agrandissement d'une cellule

/// Animations attendues par le moteur (mêmes clés que le JSON).
const ANIMS: &[&str] = &[
    "idle", "alert", "tired", "sleeping", "scratchSelf", "scratchWallN", "scratchWallS",
    "scratchWallW", "scratchWallE", "N", "NE", "E", "SE", "S", "SW", "W", "NW",
];

type Mapping = HashMap<String, Vec<(i32, i32)>>;

pub fn open(app: &Application, assets: &Path, skin: &str, sheet: Option<Pixbuf>) {
    let Some(sheet) = sheet else {
        eprintln!("[neko] mapper : sheet du skin « {skin} » introuvable");
        return;
    };
    let json_path = assets.join("pets").join(format!("{skin}.json"));
    let mapping: Rc<RefCell<Mapping>> = Rc::new(RefCell::new(load_or_default(&json_path)));
    let active: Rc<RefCell<String>> = Rc::new(RefCell::new(ANIMS[0].to_string()));

    let window = ApplicationWindow::builder()
        .application(app)
        .title(format!("Neko — sprites : {skin}"))
        .build();
    window.fullscreen();

    let root = GtkBox::new(Orientation::Vertical, 8);
    root.set_margin_top(10);
    root.set_margin_bottom(10);
    root.set_margin_start(10);
    root.set_margin_end(10);

    // ---- Barre du haut ----
    let bar = GtkBox::new(Orientation::Horizontal, 8);
    let title = Label::new(Some(&format!(
        "Skin « {skin} » — choisis une animation à droite, puis clique les cellules (dans l'ordre)"
    )));
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    let save_btn = Button::with_label("💾 Enregistrer");
    let close_btn = Button::with_label("Fermer (Échap)");
    bar.append(&title);
    bar.append(&spacer);
    bar.append(&save_btn);
    bar.append(&close_btn);
    root.append(&bar);

    // ---- Corps : grille de cellules + liste d'animations ----
    let body = GtkBox::new(Orientation::Horizontal, 12);
    body.set_vexpand(true);

    let grid = Grid::new();
    grid.set_row_spacing(4);
    grid.set_column_spacing(4);
    grid.set_halign(gtk::Align::Center);
    grid.set_valign(gtk::Align::Center);
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
                    cr.set_source_rgb(0.13, 0.13, 0.13);
                    let _ = cr.paint();
                    let sub = sheet.new_subpixbuf(c * STRIDE, r * STRIDE, TILE, TILE);
                    let _ = cr.save();
                    cr.scale(CELL, CELL);
                    cr.set_source_pixbuf(&sub, 0.0, 0.0);
                    let _ = cr.paint();
                    let _ = cr.restore();
                    // Badge : animations assignées à cette cellule.
                    let names: Vec<String> = mapping
                        .borrow()
                        .iter()
                        .filter(|(_, v)| v.iter().any(|p| *p == (c, r)))
                        .map(|(k, _)| k.clone())
                        .collect();
                    if !names.is_empty() {
                        cr.set_source_rgba(0.0, 0.4, 0.8, 0.85);
                        cr.rectangle(0.0, h as f64 - 13.0, w as f64, 13.0);
                        let _ = cr.fill();
                        cr.set_source_rgb(1.0, 1.0, 1.0);
                        cr.set_font_size(9.0);
                        cr.move_to(2.0, h as f64 - 3.0);
                        let _ = cr.show_text(&names.join(","));
                    }
                });
            }
            let click = gtk::GestureClick::new();
            {
                let mapping = mapping.clone();
                let active = active.clone();
                let cells = cells.clone();
                click.connect_pressed(move |_g, _n, _x, _y| {
                    {
                        let a = active.borrow().clone();
                        let mut m = mapping.borrow_mut();
                        let v = m.entry(a).or_default();
                        match v.iter().position(|p| *p == (c, r)) {
                            Some(i) => {
                                v.remove(i);
                            }
                            None => v.push((c, r)),
                        }
                    }
                    for da in cells.borrow().iter() {
                        da.queue_draw();
                    }
                });
            }
            da.add_controller(click);
            grid.attach(&da, c, r, 1, 1);
            cells.borrow_mut().push(da);
        }
    }
    body.append(&grid);

    // Liste des animations.
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
    scroll.set_min_content_width(160);
    body.append(&scroll);
    root.append(&body);

    window.set_child(Some(&root));

    // ---- Actions ----
    {
        let mapping = mapping.clone();
        let json_path = json_path.clone();
        let title = title.clone();
        save_btn.connect_clicked(move |_| {
            let m = mapping.borrow();
            let out: HashMap<&String, Vec<[i32; 2]>> = m
                .iter()
                .map(|(k, v)| (k, v.iter().map(|p| [p.0, p.1]).collect()))
                .collect();
            match serde_json::to_string_pretty(&out).map(|s| std::fs::write(&json_path, s)) {
                Ok(Ok(())) => title.set_text("Enregistré ✓ — appliqué au chat"),
                _ => title.set_text("Échec de l'enregistrement"),
            }
        });
    }
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

/// Charge le mapping du skin, ou le mapping neko embarqué par défaut.
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
