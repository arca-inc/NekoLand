//! Suivi de la position globale du curseur et de l'état de la souris.
//!
//! Permet au chat de chasser le pointeur (mode « Souris ») et de détecter
//! la saisie / le drag sous Windows et les compositeurs compatibles.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    pub x: f64,
    pub y: f64,
    pub left_down: bool,
    pub valid: bool,
}

impl Cursor {
    fn empty() -> Self {
        Cursor {
            x: 0.0,
            y: 0.0,
            left_down: false,
            valid: false,
        }
    }
}

/// Lance le thread de sondage (~30 Hz quand actif) et renvoie l'état partagé.
pub fn spawn(enabled: Arc<AtomicBool>) -> Arc<Mutex<Cursor>> {
    let shared = Arc::new(Mutex::new(Cursor::empty()));
    let out = shared.clone();
    std::thread::spawn(move || loop {
        if !enabled.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }

        match query() {
            Some((x, y, left_down)) => {
                let mut c = out.lock().unwrap();
                c.x = x;
                c.y = y;
                c.left_down = left_down;
                c.valid = true;
            }
            None => {
                let mut c = out.lock().unwrap();
                c.valid = false;
                c.left_down = false;
                drop(c);
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
        }
        std::thread::sleep(Duration::from_millis(33));
    });
    shared
}

/// Interroge la position du curseur et l'état du clic gauche.
fn query() -> Option<(f64, f64, bool)> {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn GetAsyncKeyState(vKey: i32) -> i16;
        }
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut point = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
        unsafe {
            if GetCursorPos(&mut point) != 0 {
                // VK_LBUTTON = 0x01. Bit de poids fort = bouton actuellement pressé.
                let left_down = (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0;
                return Some((point.x as f64, point.y as f64, left_down));
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(pos) = query_hyprland_socket() {
            return Some((pos.0, pos.1, false));
        }

        let out = std::process::Command::new("hyprctl")
            .args(["cursorpos", "-j"])
            .output()
            .ok()?;
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
        let x = v.get("x")?.as_f64()?;
        let y = v.get("y")?.as_f64()?;
        Some((x, y, false))
    }
}

#[cfg(not(target_os = "windows"))]
fn query_hyprland_socket() -> Option<(f64, f64)> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let xdg = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let sock_path = format!("{}/hypr/{}/.socket.sock", xdg, his);
    let mut stream = UnixStream::connect(sock_path).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(50))).ok();
    stream.set_write_timeout(Some(Duration::from_millis(50))).ok();
    stream.write_all(b"j/cursorpos").ok()?;
    let mut resp = Vec::with_capacity(256);
    stream.read_to_end(&mut resp).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&resp).ok()?;
    let x = v.get("x")?.as_f64()?;
    let y = v.get("y")?.as_f64()?;
    Some((x, y))
}
