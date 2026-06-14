//! Icône de barre système (StatusNotifierItem via `ksni`).
//!
//! `ksni` lance son propre thread D-Bus, indépendant de la boucle GTK. Le menu
//! permet (pour l'instant) de quitter l'application.

use gtk::gdk_pixbuf::Pixbuf;
use ksni::{Icon, MenuItem, Tray};

struct NekoTray {
    icon: Vec<Icon>,
}

impl Tray for NekoTray {
    fn id(&self) -> String {
        "neko_rust".into()
    }
    fn title(&self) -> String {
        "Neko".into()
    }
    fn icon_name(&self) -> String {
        // Fallback si l'hôte ignore le pixmap.
        "face-smile".into()
    }
    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icon.clone()
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![StandardItem {
            label: "Quitter".into(),
            icon_name: "application-exit".into(),
            activate: Box::new(|_| std::process::exit(0)),
            ..Default::default()
        }
        .into()]
    }
}

/// Lance le tray dans son thread. `sprite` sert à fabriquer l'icône (tuile idle).
pub fn spawn(sprite: Option<&Pixbuf>) {
    let icon = sprite.map(icon_from_tile).into_iter().collect();
    let service = ksni::TrayService::new(NekoTray { icon });
    service.spawn();
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
