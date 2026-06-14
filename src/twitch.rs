//! Client Twitch « Heat » — porté depuis TwitchListen.ts.
//!
//! Heat (https://heat.j38.net) renvoie, pour chaque clic d'un viewer sur le
//! stream, des coordonnées normalisées (x, y) dans [0,1]. On les convertit en
//! pixels écran et on les pousse dans l'état partagé que la boucle GTK lit.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// Cible courante partagée entre le thread Twitch et la boucle GTK.
pub struct Target {
    pub x: f64,
    pub y: f64,
    /// Vrai dès qu'un clic est reçu → le chat passe en mode CHASE. La boucle GTK
    /// le considère expiré quand `updated` est trop ancien (cf. `TWITCH_TIMEOUT`),
    /// pour que le chat reprenne son comportement normal entre deux clics.
    pub active: bool,
    /// Instant du dernier clic Twitch reçu.
    pub updated: Instant,
}

/// Boucle de connexion (reconnexion automatique). À lancer sur un runtime tokio.
pub async fn run(channel_id: String, shared: Arc<Mutex<Target>>, screen_w: f64, screen_h: f64) {
    let url = format!("wss://heat-api.j38.net/channel/{channel_id}");

    loop {
        match connect_async(&url).await {
            Ok((mut ws, _)) => {
                eprintln!("[twitch] connecté au canal {channel_id}");
                let _ = ws.send(Message::Text("connect".into())).await;

                while let Some(msg) = ws.next().await {
                    let Ok(Message::Text(txt)) = msg else { continue };
                    let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else { continue };

                    // Heat envoie x/y soit en nombre soit en chaîne ("0.42").
                    let parse = |val: &serde_json::Value| -> Option<f64> {
                        val.as_f64().or_else(|| val.as_str().and_then(|s| s.parse().ok()))
                    };
                    if let (Some(x), Some(y)) = (parse(&v["x"]), parse(&v["y"])) {
                        let mut t = shared.lock().unwrap();
                        t.x = x * screen_w;
                        t.y = y * screen_h;
                        t.active = true;
                        t.updated = Instant::now();
                    }
                }
                eprintln!("[twitch] déconnecté, nouvelle tentative…");
            }
            Err(e) => eprintln!("[twitch] échec connexion : {e}"),
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
