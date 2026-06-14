# NekoLand

Nativer NekoLand, geschrieben in Rust. Eine animierte Katze jagt ein **Wollknäuel**, das über den Desktop hüpft; wenn sie es fängt, ruht sie sich aus, bevor ein neues Knäuel erscheint. Wenn ein **Twitch Heat**-Kanal aktiv ist, jagt sie vorrangig die Klicks der Zuschauer im Stream.

## Verhalten (Gameplay)

Drei **Modi** (über das Tray-Menü im laufenden Betrieb umschaltbar, werden gespeichert):

- **Spielzeug**: Ein Wollknäuel springt diagonal an den Rändern des globalen Bereichs entlang. Die Katze jagt es, fängt es durch Kollision (< 24 px), das Knäuel versteckt sich, die Katze ruht sich aus (~2,5s), dann erscheint ein neues Knäuel.
- **Autonom**: Die Katze wandert zu zufälligen Zielen und macht nach der Ankunft ein Nickerchen. **Dock-Untermodus** (Hyprland): Wenn ein Fenster > 1 Min. fokussiert bleibt, wandert die Katze horizontal an dessen unterem Rand entlang. Sobald sich der Fokus ändert, nimmt sie ihr freies Umherstreifen wieder auf.
- **Schlaf**: Die Katze bleibt an Ort und Stelle und schläft.

**Twitch Heat** hat in allen Modi Vorrang: Wenn `twitch_channel` konfiguriert ist und ein Zuschauer klickt, jagt die Katze dieses Ziel.

## Sprites

Skins aus dem Oneko-Archiv: <https://bomvel.neocities.org/neko/>.
Format: 8x6-Raster, 32x32 Kacheln mit **1 px Abstand**. 

Die Zuweisung Zelle → Animation erfolgt über den **nativen GTK4-Editor** (`src/mapper.rs`), der über das Tray-Menü (**"Optionen / Dashboard"**) im Vollbildmodus geöffnet wird. Man wählt eine Animation, klickt die Zellen in der Reihenfolge der Frames an und klickt auf **"Mapping speichern"**. Die App **lädt das Mapping im laufenden Betrieb neu**.

## Credits
Credits: The Neko Archive Project & Neko (software).

## Installation & Autostart

```sh
./install.sh
```
Kompiliert im Release-Modus, installiert das Binary nach `~/.local/bin/nekoland`, kopiert die Assets und erstellt `.desktop`-Einträge.
