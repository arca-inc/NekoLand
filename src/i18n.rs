use std::sync::atomic::{AtomicU8, Ordering};

// 0 = Auto/Detect, 1 = English, 2 = French, 3 = German
static ACTIVE_LANG: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang_code: &str) {
    let lower = lang_code.to_lowercase();
    let val = if lower.starts_with("fr") {
        2
    } else if lower.starts_with("de") {
        3
    } else if lower.starts_with("en") {
        1
    } else {
        0
    };
    ACTIVE_LANG.store(val, Ordering::SeqCst);
}

pub fn current_lang() -> &'static str {
    match get_effective_lang_code() {
        2 => "fr",
        3 => "de",
        _ => "en",
    }
}

fn get_effective_lang_code() -> u8 {
    let configured = ACTIVE_LANG.load(Ordering::SeqCst);
    if configured != 0 {
        return configured;
    }

    // Auto-détection système
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
        let mut buffer = [0u16; 85];
        let len = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
        if len > 0 {
            let locale = String::from_utf16_lossy(&buffer[..(len as usize).saturating_sub(1)]).to_lowercase();
            if locale.starts_with("fr") {
                return 2;
            } else if locale.starts_with("de") {
                return 3;
            }
        }
    }

    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let l = val.to_lowercase();
            if l.starts_with("fr") {
                return 2;
            } else if l.starts_with("de") {
                return 3;
            } else if l.starts_with("en") {
                return 1;
            }
        }
    }

    1 // Default to English
}

pub fn t(key: &str) -> String {
    let lang = get_effective_lang_code();
    let text = match key {
        // App & Dashboard
        "dashboard_title" => match lang {
            2 => "Paramètres NekoLand",
            3 => "NekoLand Einstellungen",
            _ => "NekoLand Settings",
        },
        "app_title" => match lang {
            2 => "Panneau de Contrôle",
            3 => "NekoLand Steuerung",
            _ => "Control Panel",
        },
        "general_tab" => match lang {
            2 => "Général",
            3 => "Allgemein",
            _ => "General",
        },
        "sprite_editor_tab" => match lang {
            2 => "Éditeur de Sprites",
            3 => "Sprite-Editor",
            _ => "Sprite Editor",
        },
        "advanced_mode" => match lang {
            2 => "Éditeur de sprites (Avancé)",
            3 => "Sprite-Editor (Erweitert)",
            _ => "Sprite Editor (Advanced)",
        },
        "back_to_settings" => match lang {
            2 => "← Paramètres",
            3 => "← Einstellungen",
            _ => "← Settings",
        },
        "section_appearance" => match lang {
            2 => "Apparence",
            3 => "Aussehen",
            _ => "Appearance",
        },
        "section_behavior" => match lang {
            2 => "Comportement",
            3 => "Verhalten",
            _ => "Behavior",
        },
        "section_system" => match lang {
            2 => "Système & Intégration",
            3 => "System & Integration",
            _ => "System & Integration",
        },
        "mode" => match lang {
            2 => "Comportement",
            3 => "Verhalten",
            _ => "Behavior",
        },
        "skin" => match lang {
            2 => "Apparence (Skin)",
            3 => "Aussehen (Skin)",
            _ => "Appearance (Skin)",
        },
        "toy" => match lang {
            2 => "Jouet (Pelote)",
            3 => "Spielzeug",
            _ => "Toy (Item)",
        },
        "basket" => match lang {
            2 => "Panier douillet",
            3 => "Gemütliches Körbchen",
            _ => "Cozy Basket",
        },
        "basket_desc" => match lang {
            2 => "Un endroit douillet où le chat adore faire la sieste !",
            3 => "Ein gemütlicher Ort, an dem die Katze gerne schläft!",
            _ => "A cozy spot where the cat loves to nap!",
        },
        "place_basket" => match lang {
            2 => "Poser le panier",
            3 => "Körbchen platzieren",
            _ => "Place Basket",
        },
        "remove_basket" => match lang {
            2 => "Retirer le panier",
            3 => "Körbchen entfernen",
            _ => "Remove Basket",
        },
        "center_basket" => match lang {
            2 => "Repositionner au centre",
            3 => "Im Zentrum platzieren",
            _ => "Center on Screen",
        },
        "scale" => match lang {
            2 => "Taille du chat",
            3 => "Katzen-Größe",
            _ => "Cat Size",
        },
        "twitch_channel" => match lang {
            2 => "Canal Twitch (Interactif)",
            3 => "Twitch-Kanal (Streamer)",
            _ => "Twitch Channel (Interactive)",
        },
        "twitch_placeholder" => match lang {
            2 => "Nom de la chaîne Twitch...",
            3 => "Kanalname eingeben...",
            _ => "Twitch channel name...",
        },
        "autostart" => match lang {
            2 => "Lancer au démarrage du système",
            3 => "Mit Windows/System starten",
            _ => "Start automatically on system boot",
        },
        "language" => match lang {
            2 => "Langue de l'interface",
            3 => "Sprache",
            _ => "Interface Language",
        },
        "save_mapping" => match lang {
            2 => "Enregistrer le mapping",
            3 => "Mapping speichern",
            _ => "Save mapping",
        },
        "close" => match lang {
            2 => "Fermer (Échap)",
            3 => "Schließen (Esc)",
            _ => "Close (Esc)",
        },
        "saved" => match lang {
            2 => "Enregistré avec succès !",
            3 => "Erfolgreich gespeichert!",
            _ => "Successfully saved!",
        },
        "failed" => match lang {
            2 => "Échec de l'enregistrement !",
            3 => "Fehlgeschlagen!",
            _ => "Save failed!",
        },
        "options" => match lang {
            2 => "Paramètres & Options",
            3 => "Optionen / Dashboard",
            _ => "Settings & Options",
        },
        "quit" => match lang {
            2 => "Quitter NekoLand",
            3 => "NekoLand beenden",
            _ => "Quit NekoLand",
        },
        // Modes
        "mode_pelote" => match lang {
            2 => "Pelote (Jeu)",
            3 => "Wollknäuel (Spiel)",
            _ => "Yarn Ball (Play)",
        },
        "mode_autonome" => match lang {
            2 => "Autonome (Errance)",
            3 => "Autonom (Streunen)",
            _ => "Autonomous (Wander)",
        },
        "mode_souris" => match lang {
            2 => "Souris (Chasseur)",
            3 => "Mauszeiger (Jäger)",
            _ => "Mouse (Chase pointer)",
        },
        "mode_sommeil" => match lang {
            2 => "Sommeil (Sieste)",
            3 => "Schlafen (Nickerchen)",
            _ => "Sleep (Nap)",
        },
        // Comportements détaillés (Behaviors)
        "behavior_drag" => match lang {
            2 => "Déplacement",
            3 => "Verschieben",
            _ => "Dragging",
        },
        "behavior_twitch" => match lang {
            2 => "Clic Twitch",
            3 => "Twitch-Klick",
            _ => "Twitch Click",
        },
        "behavior_sommeil" => match lang {
            2 => "Sommeil",
            3 => "Schlafend",
            _ => "Sleeping",
        },
        "behavior_souris" => match lang {
            2 => "Chasse souris",
            3 => "Maus-Jagd",
            _ => "Chasing Mouse",
        },
        "behavior_perimetre" => match lang {
            2 => "Tour de fenêtre",
            3 => "Fensterumfang",
            _ => "Window Perimeter",
        },
        "behavior_dock" => match lang {
            2 => "Bordure de fenêtre",
            3 => "Fenster-Dock",
            _ => "Window Dock",
        },
        "behavior_sieste" => match lang {
            2 => "Sieste au panier",
            3 => "Körbchen-Schlaf",
            _ => "Basket Nap",
        },
        "behavior_zoomies" => match lang {
            2 => "Coup de folie (Zoomies)",
            3 => "Zoomies (Tollerei)",
            _ => "Zoomies",
        },
        "behavior_errance" => match lang {
            2 => "Balade libre",
            3 => "Umherwandern",
            _ => "Wandering",
        },
        "behavior_chasse" => match lang {
            2 => "Chasse au jouet",
            3 => "Spielzeug-Jagd",
            _ => "Toy Chase",
        },
        "behavior_repos" => match lang {
            2 => "Pause / Repos",
            3 => "Ausruhen",
            _ => "Resting",
        },
        "behavior_attente" => match lang {
            2 => "Attente",
            3 => "Warten",
            _ => "Idle / Waiting",
        },
        // Animations et postures (Clips)
        "clip_idle" => match lang {
            2 => "Immobile",
            3 => "Stillstehend",
            _ => "Idle",
        },
        "clip_alert" => match lang {
            2 => "Alerte",
            3 => "Aufmerksam",
            _ => "Alert",
        },
        "clip_tired" => match lang {
            2 => "Fatigué",
            3 => "Müde",
            _ => "Tired",
        },
        "clip_sleeping" => match lang {
            2 => "Endormi",
            3 => "Schlafend",
            _ => "Sleeping",
        },
        "clip_scratchSelf" => match lang {
            2 => "Toilette",
            3 => "Putzend",
            _ => "Grooming",
        },
        "clip_scratchWallN" => match lang {
            2 => "Grattage (Haut)",
            3 => "Kratzen (Oben)",
            _ => "Scratching (Up)",
        },
        "clip_scratchWallS" => match lang {
            2 => "Grattage (Bas)",
            3 => "Kratzen (Unten)",
            _ => "Scratching (Down)",
        },
        "clip_scratchWallW" => match lang {
            2 => "Grattage (Gauche)",
            3 => "Kratzen (Links)",
            _ => "Scratching (Left)",
        },
        "clip_scratchWallE" => match lang {
            2 => "Grattage (Droite)",
            3 => "Kratzen (Rechts)",
            _ => "Scratching (Right)",
        },
        "dock_in" => match lang {
            2 => "Accrochage dans",
            3 => "Andocken in",
            _ => "Docking in",
        },
        "docked" => match lang {
            2 => "● ACCROCHÉ",
            3 => "● ANGEDOCKT",
            _ => "● DOCKED",
        },
        // Descriptions & hints
        "drag_hint" => match lang {
            2 => "Astuce : vous pouvez attraper et déplacer le chat avec la souris !",
            3 => "Tipp: Sie können die Katze mit der Maus greifen und bewegen!",
            _ => "Tip: You can grab and drag the cat anywhere with your mouse!",
        },
        "credits" => match lang {
            2 => "Crédits : The Neko Archive Project & Neko (software). Merci à m4keuse (twitch.tv/m4keuse) pour l'idée, inspirée de NekoV2 d'Aqueuse (github.com/Aqueuse/NekoV2).",
            3 => "Credits: The Neko Archive Project & Neko (software). Dank an m4keuse (twitch.tv/m4keuse) für die Idee, inspiriert von Aqueuses NekoV2 (github.com/Aqueuse/NekoV2).",
            _ => "Credits: The Neko Archive Project & Neko (software). Thanks to m4keuse (twitch.tv/m4keuse) for the idea, inspired by Aqueuse's NekoV2 (github.com/Aqueuse/NekoV2).",
        },
        // CLI Help & Messages
        "cli_usage" => match lang {
            2 => "Usage : nekoland [OPTIONS]",
            3 => "Verwendung: nekoland [OPTIONEN]",
            _ => "Usage: nekoland [OPTIONS]",
        },
        "cli_desc" => match lang {
            2 => "NekoLand - Compagnon animé félin pour votre bureau, interactif et personnalisable.",
            3 => "NekoLand - Interaktiver Katzenbegleiter für Ihren Desktop.",
            _ => "NekoLand - Animated cat desktop companion, interactive and customizable.",
        },
        "cli_opt_help" => match lang {
            2 => "Affiche ce message d'aide et quitte",
            3 => "Hilfe anzeigen und beenden",
            _ => "Show this help message and exit",
        },
        "cli_opt_version" => match lang {
            2 => "Affiche la version et quitte",
            3 => "Version anzeigen und beenden",
            _ => "Show version and exit",
        },
        "cli_opt_skin" => match lang {
            2 => "Définit le skin de départ (ex: neko, black, calico, dog)",
            3 => "Start-Skin festlegen",
            _ => "Set initial skin (e.g. neko, black, calico, dog)",
        },
        "cli_opt_toy" => match lang {
            2 => "Définit le jouet (ex: wool, darkWool, boomerang)",
            3 => "Spielzeug festlegen",
            _ => "Set initial toy (e.g. wool, darkWool, boomerang)",
        },
        "cli_opt_basket" => match lang {
            2 => "Active et pose le panier douillet pour le chat",
            3 => "Gemütliches Körbchen für die Katze aktivieren und platzieren",
            _ => "Enable and place the cozy basket for the cat",
        },
        "cli_opt_mode" => match lang {
            2 => "Mode de comportement : Pelote, Autonome, Souris, Sommeil",
            3 => "Verhaltensmodus festlegen",
            _ => "Behavior mode: Pelote/Yarn, Autonome/Auto, Souris/Mouse, Sommeil/Sleep",
        },
        "cli_opt_scale" => match lang {
            2 => "Facteur d'échelle (1.0, 1.5, 2.0, 3.0)",
            3 => "Skalierungsfaktor (1.0, 1.5, 2.0, 3.0)",
            _ => "Scale factor (1.0, 1.5, 2.0, 3.0)",
        },
        "cli_opt_twitch" => match lang {
            2 => "Canal Twitch pour interagir avec les clics du stream",
            3 => "Twitch-Kanal für Heat-API-Klicks",
            _ => "Twitch channel name for stream click interaction",
        },
        "cli_opt_lang" => match lang {
            2 => "Force la langue (fr, en, de)",
            3 => "Sprache erzwingen (fr, en, de)",
            _ => "Force language (fr, en, de)",
        },
        "cli_opt_list_skins" => match lang {
            2 => "Liste tous les skins disponibles et quitte",
            3 => "Alle verfügbaren Skins auflisten",
            _ => "List all available skins and exit",
        },
        "cli_opt_list_toys" => match lang {
            2 => "Liste tous les jouets disponibles et quitte",
            3 => "Alle verfügbaren Spielzeuge auflisten",
            _ => "List all available toys and exit",
        },
        "cli_opt_debug" => match lang {
            2 => "Active le mode débogage avec repères visuels à l'écran",
            3 => "Debug-Modus mit visuellen Overlays aktivieren",
            _ => "Enable debug logging and on-screen visual overlays",
        },
        "cli_available_skins" => match lang {
            2 => "Skins disponibles :",
            3 => "Verfügbare Skins:",
            _ => "Available skins:",
        },
        "cli_available_toys" => match lang {
            2 => "Jouets disponibles :",
            3 => "Verfügbare Spielzeuge:",
            _ => "Available toys:",
        },
        _ => key,
    };
    text.to_string()
}

/// Traduit un identifiant de comportement interne pour l'affichage utilisateur / debug.
pub fn translate_behavior(behavior: &str) -> String {
    let key = match behavior {
        "drag" => "behavior_drag",
        "twitch" => "behavior_twitch",
        "sommeil" => "behavior_sommeil",
        "souris" => "behavior_souris",
        "périmètre" => "behavior_perimetre",
        "dock" => "behavior_dock",
        "sieste" => "behavior_sieste",
        "zoomies" => "behavior_zoomies",
        "errance" => "behavior_errance",
        "chasse" => "behavior_chasse",
        "repos" => "behavior_repos",
        "attente" => "behavior_attente",
        _ => return behavior.to_string(),
    };
    t(key)
}

/// Traduit un nom de clip d'animation pour l'affichage utilisateur.
pub fn translate_clip(clip: &str) -> String {
    let key = match clip {
        "idle" => "clip_idle",
        "alert" => "clip_alert",
        "tired" => "clip_tired",
        "sleeping" => "clip_sleeping",
        "scratchSelf" => "clip_scratchSelf",
        "scratchWallN" => "clip_scratchWallN",
        "scratchWallS" => "clip_scratchWallS",
        "scratchWallW" => "clip_scratchWallW",
        "scratchWallE" => "clip_scratchWallE",
        _ => return clip.to_string(),
    };
    t(key)
}
