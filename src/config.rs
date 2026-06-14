//! Réglages persistants : `~/.config/neko-desktop/config.json`.
//!
//! Chargé au démarrage, réécrit dès qu'un réglage change (via le menu tray).
//! Les variables d'environnement `NEKO_SKIN`, `NEKO_TOY`, `NEKO_SCALE`,
//! `NEKO_TWITCH_CHANNEL` ont priorité sur le fichier (pratique pour tester).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub skin: String,
    pub toy: String,
    pub scale: f64,
    pub twitch_channel: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            skin: "neko".into(),
            toy: "wool".into(),
            scale: 1.5,
            twitch_channel: String::new(),
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into())).join(".config")
        });
    base.join("neko-desktop").join("config.json")
}

/// Charge la config (défaut si absente/invalide), puis applique les surcharges
/// d'environnement.
pub fn load() -> Config {
    let mut cfg = match std::fs::read_to_string(config_path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Config::default(),
    };
    if let Ok(v) = std::env::var("NEKO_SKIN") {
        cfg.skin = v;
    }
    if let Ok(v) = std::env::var("NEKO_TOY") {
        cfg.toy = v;
    }
    if let Ok(v) = std::env::var("NEKO_SCALE") {
        if let Ok(s) = v.parse() {
            cfg.scale = s;
        }
    }
    if let Ok(v) = std::env::var("NEKO_TWITCH_CHANNEL") {
        cfg.twitch_channel = v;
    }
    cfg
}

#[allow(dead_code)] // utilisé par le menu tray (lot suivant)
pub fn save(cfg: &Config) {
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(&path, json);
    }
}
