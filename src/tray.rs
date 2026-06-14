//! Icône de barre système (StatusNotifierItem via `ksni`).
//!
//! `ksni` lance son propre thread D-Bus, indépendant de la boucle GTK. Les
//! sous-menus permettent de changer le skin, la pelote et la taille à chaud :
//! chaque choix met à jour [`Control`] (que la boucle GTK relit) et persiste la
//! config sur disque.

use std::sync::{Arc, Mutex};

use gtk::gdk_pixbuf::Pixbuf;
use ksni::{Icon, MenuItem, Tray};

use crate::config::{self, Control};

struct NekoTray {
    icon: Vec<Icon>,
    skins: Vec<String>,
    toys: Vec<String>,
    scales: Vec<f64>,
    modes: Vec<String>,
    skin_idx: usize,
    toy_idx: usize,
    scale_idx: usize,
    mode_idx: usize,
    control: Arc<Mutex<Control>>,
}

impl NekoTray {
    /// Pousse l'état courant dans `Control` (bump version) et sauve la config.
    fn commit(&self) {
        let mut c = self.control.lock().unwrap();
        c.skin = self.skins[self.skin_idx].clone();
        c.toy = self.toys[self.toy_idx].clone();
        c.scale = self.scales[self.scale_idx];
        c.mode = self.modes[self.mode_idx].clone();
        c.version += 1;
        config::save(&c.to_config());
    }
}

impl Tray for NekoTray {
    fn id(&self) -> String {
        "neko_rust".into()
    }
    fn title(&self) -> String {
        "Neko".into()
    }
    fn icon_name(&self) -> String {
        "face-smile".into() // fallback si l'hôte ignore le pixmap
    }
    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icon.clone()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::{RadioGroup, RadioItem, StandardItem, SubMenu};

        let radio = |labels: Vec<String>,
                     selected: usize,
                     select: Box<dyn Fn(&mut NekoTray, usize)>|
         -> MenuItem<NekoTray> {
            RadioGroup {
                selected,
                select,
                options: labels
                    .into_iter()
                    .map(|label| RadioItem { label, ..Default::default() })
                    .collect(),
                ..Default::default()
            }
            .into()
        };

        vec![
            SubMenu {
                label: "Mode".into(),
                submenu: vec![radio(
                    self.modes.clone(),
                    self.mode_idx,
                    Box::new(|t, i| {
                        t.mode_idx = i;
                        t.commit();
                    }),
                )],
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Chat".into(),
                submenu: vec![radio(
                    self.skins.clone(),
                    self.skin_idx,
                    Box::new(|t, i| {
                        t.skin_idx = i;
                        t.commit();
                    }),
                )],
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Pelote".into(),
                submenu: vec![radio(
                    self.toys.clone(),
                    self.toy_idx,
                    Box::new(|t, i| {
                        t.toy_idx = i;
                        t.commit();
                    }),
                )],
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Taille".into(),
                submenu: vec![radio(
                    self.scales.iter().map(|s| format!("{s}×")).collect(),
                    self.scale_idx,
                    Box::new(|t, i| {
                        t.scale_idx = i;
                        t.commit();
                    }),
                )],
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quitter".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Lance le tray. `sprite` fabrique l'icône ; les listes alimentent les menus ;
/// `control` reçoit les changements.
pub fn spawn(
    sprite: Option<&Pixbuf>,
    skins: Vec<String>,
    toys: Vec<String>,
    scales: Vec<f64>,
    modes: Vec<String>,
    control: Arc<Mutex<Control>>,
) {
    let (skin_idx, toy_idx, scale_idx, mode_idx) = {
        let c = control.lock().unwrap();
        let skin_idx = skins.iter().position(|s| *s == c.skin).unwrap_or(0);
        let toy_idx = toys.iter().position(|t| *t == c.toy).unwrap_or(0);
        let scale_idx = scales
            .iter()
            .position(|s| (s - c.scale).abs() < 0.01)
            .unwrap_or(0);
        let mode_idx = modes.iter().position(|m| *m == c.mode).unwrap_or(0);
        (skin_idx, toy_idx, scale_idx, mode_idx)
    };

    let icon = sprite.map(icon_from_tile).into_iter().collect();
    let tray = NekoTray {
        icon,
        skins,
        toys,
        scales,
        modes,
        skin_idx,
        toy_idx,
        scale_idx,
        mode_idx,
        control,
    };
    ksni::TrayService::new(tray).spawn();
}

/// Convertit la tuile (0,0) — la pose idle — en icône ARGB32 attendue par SNI.
fn icon_from_tile(sheet: &Pixbuf) -> Icon {
    let tile = sheet.new_subpixbuf(0, 0, 32, 32);
    let w = tile.width();
    let h = tile.height();
    let nch = tile.n_channels() as usize;
    let rowstride = tile.rowstride() as usize;
    let pixels = unsafe { tile.pixels() };

    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * rowstride + x * nch;
            let r = pixels[i];
            let g = pixels[i + 1];
            let b = pixels[i + 2];
            let a = if nch == 4 { pixels[i + 3] } else { 255 };
            // SNI attend de l'ARGB32 en ordre réseau (gros-boutiste) : A, R, G, B.
            data.extend_from_slice(&[a, r, g, b]);
        }
    }
    Icon { width: w, height: h, data }
}
