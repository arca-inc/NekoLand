use std::sync::{Arc, Mutex};
use gtk::gdk_pixbuf::Pixbuf;
use tray_icon::{TrayIconBuilder, Icon, menu::{Menu, MenuItem, MenuEvent, PredefinedMenuItem, Submenu}};
use crate::config::Control;

pub fn spawn(
    sprite: Option<&Pixbuf>,
    _skins: Vec<String>,
    _toys: Vec<String>,
    _scales: Vec<f64>,
    _modes: Vec<String>,
    control: Arc<Mutex<Control>>,
) {
    let mut icon = None;
    if let Some(sheet) = sprite {
        let tile = sheet.new_subpixbuf(0, 0, 32, 32);
        let w = tile.width() as u32;
        let h = tile.height() as u32;
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
                data.extend_from_slice(&[r, g, b, a]);
            }
        }
        icon = Icon::from_rgba(data, w, h).ok();
    }

    let menu = Menu::new();
    let opt_item = MenuItem::new(crate::i18n::t("options").to_string(), true, None);

    let mode_sub = Submenu::new(crate::i18n::t("mode").to_string(), true);
    let mode_pelote = MenuItem::new(crate::i18n::t("mode_pelote").to_string(), true, None);
    let mode_auto = MenuItem::new(crate::i18n::t("mode_autonome").to_string(), true, None);
    let mode_mouse = MenuItem::new(crate::i18n::t("mode_souris").to_string(), true, None);
    let mode_sleep = MenuItem::new(crate::i18n::t("mode_sommeil").to_string(), true, None);
    let _ = mode_sub.append(&mode_pelote);
    let _ = mode_sub.append(&mode_auto);
    let _ = mode_sub.append(&mode_mouse);
    let _ = mode_sub.append(&mode_sleep);

    let quit_item = MenuItem::new(crate::i18n::t("quit").to_string(), true, None);

    let _ = menu.append(&opt_item);
    let _ = menu.append(&mode_sub);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&quit_item);

    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("NekoLand");

    if let Some(ic) = icon {
        builder = builder.with_icon(ic);
    }

    let tray_icon = builder.build().expect("Failed to build tray icon");
    Box::leak(Box::new(tray_icon));

    let opt_id = opt_item.id().clone();
    let quit_id = quit_item.id().clone();
    let p_id = mode_pelote.id().clone();
    let a_id = mode_auto.id().clone();
    let m_id = mode_mouse.id().clone();
    let s_id = mode_sleep.id().clone();

    gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == opt_id {
                let mut c = control.lock().unwrap();
                c.open_config += 1;
            } else if event.id == quit_id {
                std::process::exit(0);
            } else if event.id == p_id {
                let mut c = control.lock().unwrap();
                c.mode = "Pelote".to_string();
                c.version += 1;
            } else if event.id == a_id {
                let mut c = control.lock().unwrap();
                c.mode = "Autonome".to_string();
                c.version += 1;
            } else if event.id == m_id {
                let mut c = control.lock().unwrap();
                c.mode = "Souris".to_string();
                c.version += 1;
            } else if event.id == s_id {
                let mut c = control.lock().unwrap();
                c.mode = "Sommeil".to_string();
                c.version += 1;
            }
        }
        gtk::glib::ControlFlow::Continue
    });
}
