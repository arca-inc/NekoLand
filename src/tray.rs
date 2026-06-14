//! Icône de barre système (StatusNotifierItem via `ksni`).
//!
//! `ksni` lance son propre thread D-Bus, indépendant de la boucle GTK.

use std::sync::{Arc, Mutex};

use gtk::gdk_pixbuf::Pixbuf;
use ksni::{Icon, MenuItem, Tray};

use crate::config::Control;

struct NekoTray {
    icon: Vec<Icon>,
    control: Arc<Mutex<Control>>,
}

impl NekoTray {
    /// Demande à la boucle GTK d'ouvrir le dashboard de configuration.
    fn request_config(&self) {
        let mut c = self.control.lock().unwrap();
        c.open_config += 1;
    }
}

impl Tray for NekoTray {
    fn id(&self) -> String {
        "nekoland".into()
    }
    fn title(&self) -> String {
        "NekoLand".into()
    }
    fn icon_name(&self) -> String {
        "face-smile".into() // fallback si l'hôte ignore le pixmap
    }
    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icon.clone()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::StandardItem;

        vec![
            StandardItem {
                label: crate::i18n::t("options").into(),
                icon_name: "preferences-system".into(),
                activate: Box::new(|t: &mut NekoTray| t.request_config()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: crate::i18n::t("quit").into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Lance le tray. `sprite` fabrique l'icône.
pub fn spawn(
    sprite: Option<&Pixbuf>,
    _skins: Vec<String>,
    _toys: Vec<String>,
    _scales: Vec<f64>,
    _modes: Vec<String>,
    control: Arc<Mutex<Control>>,
) {
    let icon = sprite.map(icon_from_tile).into_iter().collect();
    let tray = NekoTray {
        icon,
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
