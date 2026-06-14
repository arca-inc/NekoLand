use std::env;

pub fn t(key: &str) -> String {
    let lang = env::var("LANG").unwrap_or_else(|_| "en".to_string());
    
    let is_fr = lang.starts_with("fr");
    let is_de = lang.starts_with("de");

    let text = match key {
        "dashboard_title" => if is_fr { "Dashboard NekoLand" } else if is_de { "NekoLand Dashboard" } else { "NekoLand Dashboard" },
        "app_title" => if is_fr { "Contrôle NekoLand" } else if is_de { "NekoLand Steuerung" } else { "NekoLand Control" },
        "mode" => if is_fr { "Comportement (Mode)" } else if is_de { "Verhalten (Modus)" } else { "Behavior (Mode)" },
        "skin" => if is_fr { "Apparence (Skin)" } else if is_de { "Aussehen (Skin)" } else { "Appearance (Skin)" },
        "toy" => if is_fr { "Pelote (Jouet)" } else if is_de { "Spielzeug (Item)" } else { "Toy (Item)" },
        "scale" => if is_fr { "Taille (Scale)" } else if is_de { "Größe (Scale)" } else { "Size (Scale)" },
        "twitch_channel" => if is_fr { "Canal Twitch (Streamer)" } else if is_de { "Twitch-Kanal (Streamer)" } else { "Twitch Channel (Streamer)" },
        "twitch_placeholder" => if is_fr { "nom de la chaîne, puis Entrée" } else if is_de { "Kanalname, dann Enter" } else { "channel name, then Enter" },
        "save_mapping" => if is_fr { "💾 Enregistrer mapping" } else if is_de { "💾 Mapping speichern" } else { "💾 Save mapping" },
        "close" => if is_fr { "Fermer (Échap)" } else if is_de { "Schließen (Esc)" } else { "Close (Esc)" },
        "sprite_editor" => if is_fr { "Éditeur de sprites" } else if is_de { "Sprite-Editor" } else { "Sprite Editor" },
        "saved" => if is_fr { "Enregistré ✓" } else if is_de { "Gespeichert ✓" } else { "Saved ✓" },
        "failed" => if is_fr { "Échec !" } else if is_de { "Fehlgeschlagen!" } else { "Failed!" },
        "options" => if is_fr { "Options / Dashboard" } else if is_de { "Optionen / Dashboard" } else { "Options / Dashboard" },
        "quit" => if is_fr { "Quitter" } else if is_de { "Beenden" } else { "Quit" },
        "credits" => if is_fr {
            "Crédits : The Neko Archive Project & Neko (software). Merci à m4keuse (twitch.tv/m4keuse) pour l'idée, inspirée de NekoV2 d'Aqueuse (github.com/Aqueuse/NekoV2)."
        } else if is_de {
            "Credits: The Neko Archive Project & Neko (software). Dank an m4keuse (twitch.tv/m4keuse) für die Idee, inspiriert von Aqueuses NekoV2 (github.com/Aqueuse/NekoV2)."
        } else {
            "Credits: The Neko Archive Project & Neko (software). Thanks to m4keuse (twitch.tv/m4keuse) for the idea, inspired by Aqueuse's NekoV2 (github.com/Aqueuse/NekoV2)."
        },
        _ => key,
    };
    text.to_string()
}
