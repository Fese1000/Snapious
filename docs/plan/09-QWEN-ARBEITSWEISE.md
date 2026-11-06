# Arbeitsweise mit Qwen3.6-35B-A3B (lokal)

Ziel dieses Dokuments: Aufgaben so zuschneiden, dass ein Modell mit **35 Mrd. Parametern insgesamt, aber nur ca. 3 Mrd. aktiv pro Token** sie zuverlässig umsetzt.
*Einschätzung (ohne eigene Messung an diesem Modell):* Solche Mixture-of-Experts-Modelle sind breit gebildet und schnell, aber bei langen mehrstufigen Herleitungen,
feinen Details in langen Texten und strenger Typlogik (Rust-Borrow-Checker) anfälliger als dichte Modelle gleicher Größe. Deshalb gilt:
**wenig Kontext, kleine Schritte, fertige Tests, mechanische Prüfung.** Wenn sich in der Praxis zeigt, dass das Modell mehr schafft, können die Schritte größer werden.

## Grundprinzipien

1. **Test zuerst, von Claude geschrieben.** Für jede Aufgabe legt Claude (oder du) das Gerüst an: Datei mit Typen, Funktionssignaturen mit `todo!()` im Rumpf und **fertigen Tests**. Qwen füllt nur die Funktionsrümpfe,
   bis `cargo test` grün ist. Das ist die wirksamste Hilfe für kleine Modelle, weil „grün/rot" ein eindeutiges Ziel ist.
2. **Ein Prompt = eine Aufgabenkarte** (höchstens ca. 200 Zeilen neuer Code, eine Datei, ein Thema). Größere Aufgaben aus `01`–`07` werden vorher in Karten zerlegt (Regeln unten).
3. **Kleiner Kontext.** Im Prompt stehen nur: Systemtext (`00-ARBEITSANWEISUNG.md`), die Aufgabenkarte, die betroffene Datei, höchstens zwei Hilfsdateien (z. B. `error.rs`, ein Typ, den die Karte nutzt).
   Nicht die ganze Phasen-Datei, nicht das Java-Projekt (außer die Karte nennt genau eine Java-Methode als Referenz und fügt sie ein).
4. **Mechanische Prüfung statt Urteilskraft.** Qwen führt `cargo check`/`test`/`clippy`/`fmt` selbst aus und liest die Fehlermeldungen. Kein „sieht gut aus".
5. **Drei Versuche, dann Schluss.** Bei hängenbleibenden Fehlern nicht weiterraten, sondern in `OFFENE_FRAGEN.md` eintragen (siehe Systemtext). Claude schaut dann nach.
6. **Review durch Claude** nach jeder Aufgabe (Checkliste unten), bevor die nächste Karte vergeben wird.
7. Empfohlene Einstellungen (anpassen, falls das Modell anders reagiert): niedrige Temperatur (ca. 0,2–0,3), Antworten nicht abschneiden (ausreichend `max_tokens`),
   Denkmodus – falls vorhanden – für Zerlegung und Fehlersuche an, fürs reine Ausfüllen von Rümpfen aus.

## Aufgabenkarte (Vorlage)

```
# Karte <Nr>: <Titel>
Datei(en): crates/snapious-core/src/<pfad>.rs      (bereits mit Gerüst und Tests vorhanden)
Voraussetzung: <welche Karten fertig sein müssen>
Ziel: <ein Satz>
Funktionen: <Liste mit Signatur und je einem Satz Verhalten>
Regeln/Formeln: <genau die nötigen, mit Zahlenbeispiel>
Randfälle: <Liste>
Referenz: <optional: genau eine Java-Methode, eingefügt>
Fertig, wenn: `cargo test -p snapious-core <modul>` grün, fmt/clippy sauber.
Nicht tun: <was typischerweise schiefgeht: andere Dateien ändern, Tests ändern, neue Crates>
```

### Beispiel: Karte 1.1 „Zeilenlayout" (geprüft: Gerüst kompiliert, mit Referenzlösung sind alle 6 Tests grün)

```
# Karte 1.1: Zeilenlayout der Sequenzansicht
Datei: crates/snapious-core/src/view/seq_layout.rs (Gerüst und Tests vorhanden)
Voraussetzung: keine (nur `error.rs`)
Ziel: Eine Sequenz wird in Zeilen fester Länge aufgeteilt; Positionen und sichtbare Zeilen lassen sich berechnen.
Funktionen:
  SequenceLayout::new(length, bases_per_line) -> Result<Self>   // bases_per_line == 0 → Err(InvalidInput)
  line_count() -> usize            // ceil(length / bases_per_line), bei length 0: 0
  line_of(position) -> usize       // position / bases_per_line
  column_of(position) -> usize     // position % bases_per_line
  line_start(line) -> usize        // line * bases_per_line
  line_end(line) -> usize          // min(line_start + bases_per_line, length)
  first_visible_line(scroll_y, line_height) -> usize      // floor(scroll_y / line_height), bei scroll_y <= 0 oder line_height <= 0: 0
  last_visible_line(&self, scroll_y, view_height, line_height) -> usize
                                   // floor((scroll_y + view_height) / line_height), begrenzt auf line_count() - 1 (bei 0 Zeilen: 0)
  bases_per_line_for(view_width, char_width, margin_width, group_size) -> usize
                                   // Gruppenbreite = (group_size + 1) * char_width; verfügbar = view_width - margin_width (mind. 0)
                                   // Gruppen = floor(verfügbar / Gruppenbreite), mindestens 1; Ergebnis = Gruppen * group_size
Randfälle: Länge 0, Position genau am Zeilenanfang, Letzte Zeile kürzer, group_size 0 → wie 1 behandeln.
Fertig, wenn: `cargo test -p snapious-core seq_layout` grün.
Nicht tun: Tests ändern; andere Dateien ändern; `unwrap()`.
```

**Gerüst, das Claude vorher anlegt** (Rümpfe leer, Tests fertig):

```rust
//! Zeilenlayout der Sequenzansicht (reine Rechenlogik, ohne Oberfläche).
use crate::error::{Error, Result};

/// Aufteilung einer Sequenz in Zeilen fester Länge.
#[derive(Clone, Debug, PartialEq)]
pub struct SequenceLayout { length: usize, bases_per_line: usize }

impl SequenceLayout {
    pub fn new(length: usize, bases_per_line: usize) -> Result<Self> { todo!() }
    pub fn line_count(&self) -> usize { todo!() }
    pub fn line_of(&self, position: usize) -> usize { todo!() }
    pub fn column_of(&self, position: usize) -> usize { todo!() }
    pub fn line_start(&self, line: usize) -> usize { todo!() }
    pub fn line_end(&self, line: usize) -> usize { todo!() }
    pub fn first_visible_line(scroll_y: f64, line_height: f64) -> usize { todo!() }
    pub fn last_visible_line(&self, scroll_y: f64, view_height: f64, line_height: f64) -> usize { todo!() }
}

pub fn bases_per_line_for(view_width: f64, char_width: f64, margin_width: f64, group_size: usize) -> usize { todo!() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeilen_bei_laenge_125_und_60_pro_zeile() {
        let l = SequenceLayout::new(125, 60).unwrap();
        assert_eq!(l.line_count(), 3);
        assert_eq!(l.line_of(59), 0);
        assert_eq!(l.line_of(60), 1);
        assert_eq!(l.column_of(61), 1);
        assert_eq!(l.line_start(2), 120);
        assert_eq!(l.line_end(2), 125);
    }

    #[test]
    fn leere_sequenz_hat_keine_zeilen() {
        assert_eq!(SequenceLayout::new(0, 60).unwrap().line_count(), 0);
    }

    #[test]
    fn null_basen_pro_zeile_ist_fehler() {
        assert!(SequenceLayout::new(10, 0).is_err());
    }

    #[test]
    fn sichtbare_zeilen_bei_scrollposition_250() {
        let l = SequenceLayout::new(6000, 60).unwrap(); // genau 100 Zeilen
        assert_eq!(SequenceLayout::first_visible_line(250.0, 20.0), 12);
        assert_eq!(l.last_visible_line(250.0, 100.0, 20.0), 17);
    }

    #[test]
    fn letzte_zeile_wird_auf_vorhandene_zeilen_begrenzt() {
        let l = SequenceLayout::new(125, 60).unwrap(); // 3 Zeilen
        assert_eq!(l.last_visible_line(0.0, 10_000.0, 20.0), 2);
    }

    #[test]
    fn basen_pro_zeile_aus_fensterbreite() {
        assert_eq!(bases_per_line_for(800.0, 10.0, 80.0, 10), 60);
        assert_eq!(bases_per_line_for(50.0, 10.0, 0.0, 10), 10);
    }
}
```

*(`error.rs` mit `Error::InvalidInput(String)` und `type Result<T>` legt Phase 0 an.)*

## So zerlegt Claude die Aufgaben aus `01`–`07` in Karten

| Regel | Beispiel |
|---|---|
| Eine Karte pro **Datei** und **Thema**, höchstens ca. 200 Zeilen Code | 2.1 Virtueller Verdau → 2.1a `End`/`Fragment`-Typen, 2.1b `cut_positions` (nur Treffer und Schnitte), 2.1c `digest_linear`, 2.1d `digest_circular`, 2.1e Features in Fragmenten |
| Typen und Konstruktoren zuerst, Algorithmen danach | 2.3 → 2.3a Datentypen, 2.3b `find_overlap`, 2.3c `assemble_linear`, 2.3d zirkulär, 2.3e Warnungen, 2.3f Primerdesign |
| Reine Funktionen vor Zustand | `EditSession` erst nach `SequenceDocument::insert/delete` |
| Jede Karte hat Tests mit **konkreten Zahlen** (aus den Phasen-Dateien oder Java-Tests) | siehe Beispiel oben |
| Komplexe Formeln stehen **ausgeschrieben** auf der Karte | Nearest-Neighbor-Tm: Tabelle und Formel in der Karte, nicht „siehe Paper" |
| Fließkommazahlen im Test immer mit Toleranz | `assert!((a - b).abs() < 1e-4)` |
| Aufgaben, die mehrere Module berühren, werden getrennt: erst Kern-API, dann Verbraucher | 2.2 Restriktionsklonierung nach 2.1 |

**Reihenfolge der Karten** folgt `08-RUST-KERN-SWIFT-APP.md`, Abschnitt „Reihenfolge". Claude legt Gerüst + Tests **eine Karte im Voraus** an (so liegt immer die nächste Karte bereit).

## Spickzettel: typische Rust-Fehler und was zu tun ist

| Fehler | Ursache | Lösung |
|---|---|---|
| `E0382 borrow of moved value` | Wert wurde verschoben | `.clone()` an der ersten Verwendung, oder Funktion nimmt `&T` statt `T` |
| `E0502/E0499 cannot borrow as mutable` | gleichzeitige Leih-Zugriffe | Wert vorher in lokale Variable kopieren (`let n = v.len();`), dann mutieren |
| `E0308 mismatched types` | `usize`/`i32`/`f64` vermischt | explizit umwandeln: `x as usize`, `x as f64`; bei Subtraktion `usize`: `checked_sub`/`saturating_sub` |
| `E0599 no method named …` | Trait nicht importiert oder Typ falsch | `use` ergänzen; mit `.iter()` bzw. `.as_bytes()` auf den richtigen Typ gehen |
| `E0106 missing lifetime specifier` | Referenz in Struktur/Rückgabe | **nicht** `'a` einführen: Strukturfelder als `String`/`Vec<u8>` besitzen; Rückgabe als eigener Wert |
| `E0277 the trait bound … is not satisfied` | fehlende `derive`s | `#[derive(Clone, Debug, PartialEq)]` ergänzen (bei `f64`: kein `Eq`/`Hash`) |
| `unused …`-Warnungen mit `-D warnings` | clippy | entfernen oder `_name` |
| `attempt to subtract with overflow` im Test | `usize`-Unterlauf | `saturating_sub` bzw. vorher vergleichen |
| `index out of bounds` | ungeprüfter Index | `get(i)` + `ok_or(...)?`, oder vorher `if i >= len { return Err(...) }` |
| Test hängt (Endlosschleife) | Schleife ohne Fortschritt (z. B. Schnitt an Position 0) | Schleifenvariable muss in **jedem** Durchlauf wachsen; Fall `pos == last` ausdrücklich behandeln |
| `clippy::needless_range_loop` | Schleife über Index | `for (i, x) in v.iter().enumerate()` |
| Sequenz als `String` indiziert | UTF-8 | Sequenzen als `&[u8]` behandeln (`bases[i]`) |

## Typische Fallen bei kleinen Modellen (und Gegenmaßnahmen)

| Falle | Gegenmaßnahme |
|---|---|
| ändert die vorgegebenen Tests, bis sie grün sind | Karte: „Nicht tun: Tests ändern"; Claude prüft per `git diff`/Dateivergleich, dass Testcode unverändert ist (Datei-Hash der `mod tests`) |
| erfindet Funktionen/Crates, die es nicht gibt | nur Standardbibliothek + erlaubte Crates (`08`, Tabelle); `cargo check` entlarvt es sofort |
| löst ein einfacheres Problem als gefragt | Karte enthält Zahlenbeispiele; Tests prüfen exakte Werte |
| bricht bei langen Antworten ab oder wiederholt sich | kleine Karten; Rumpf für Rumpf ausfüllen lassen; ggf. Funktion einzeln als Prompt |
| ändert Formatierung/benennt um | `cargo fmt` am Ende; Karte nennt die erlaubten Dateien |
| hängt in Fehlerschleifen | Drei-Versuche-Regel (Systemtext Nr. 12) |
| vergisst Randfälle | Karte listet Randfälle; Tests decken sie ab |
| Mischt Java-Syntax in Rust | Karten enthalten **nur Rust-Signaturen**; Java nur als Referenzstück, ausdrücklich so markiert |

## Prüfliste für Claude nach jeder Qwen-Karte

1. Wurden nur die erlaubten Dateien geändert? (Dateiliste vergleichen)
2. Ist der Testcode unverändert? (`mod tests` mit Vorlage vergleichen)
3. `cargo test -p snapious-core`, `cargo clippy -p snapious-core -- -D warnings`, `cargo fmt --check` ohne Fehler?
4. Stichprobe: ein zusätzlicher eigener Test mit einem anderen Zahlenbeispiel (Randfall). Falls rot → Karte zurück.
5. Keine `unwrap()`/`expect()`/`panic!`/`unsafe` im Bibliothekscode? (`grep`)
6. Keine neuen Abhängigkeiten in `Cargo.toml`? Keine macOS-spezifischen Aufrufe?
7. Dokumentationskommentare auf Deutsch, Quelle bei Algorithmen genannt?
8. Fortschritt in `00-ARBEITSANWEISUNG.md` bzw. Kartenliste aktualisieren.

## Vorschlag für die Zusammenarbeit

| Wer | Aufgabe |
|---|---|
| Claude | Phase 0 und 0.9, FFI (F.0), alle Swift-Aufgaben (S.x), Gerüste + Tests der Qwen-Karten, Review |
| Qwen | Funktionsrümpfe der Rust-Karten ab Phase 1 (Layout, Formate, Algorithmen, Klonierung, Primer, Bäume, Netz, …) |
| Felix | Xcode/Programme installieren, Qwen starten und Ausgaben weiterreichen, visuelles Feedback zur App |

Wenn das Modell sich als zuverlässiger erweist als angenommen: Karten zusammenfassen (z. B. eine ganze Aufgabe `2.x` pro Karte). Wenn es oft scheitert: Karten weiter verkleinern (eine Funktion pro Prompt).

## Pilotlauf Karte 1.1 (25.10.2026) – Ergebnis

- **Zugang:** LM Studio läuft auf `http://localhost:1234/v1` (OpenAI-kompatibel), Modell-ID `qwen3.6-35b-a3b`. Claude ruft es direkt per `chat/completions` auf (Skript `scripts/qwen/run_card_1.1.py`).
- **Arbeitsweise im Lauf:** Qwen hat keinen Dateizugriff; es bekommt Systemtext + Karte + Dateien und liefert die **vollständige Datei in einem Codeblock**. Das Skript prüft, dass der Testblock **buchstäblich unverändert** ist,
  schreibt die Datei, führt `cargo test` aus und reicht Fehlermeldungen (bis zu 3 Versuche) zurück.
- **Ergebnis:** Alle 6 Tests grün nach **2 Versuchen**. Versuch 1: Tests grün, aber `clippy` bemängelte eine von Hand geschriebene Aufrufsumme (`manual_div_ceil`); Versuch 2 korrigierte das nach Rückmeldung.
  Der Testblock blieb unverändert. Meine zusätzlichen Randfalltests (negative Scrollwerte, Zeichenbreite 0, `group_size` 0, Fenster kleiner als Rand, Grenzfälle bei der Gruppenzahl) bestanden alle.
- **Beobachtungen:** Antwort 1 dauerte 148 s und nutzte ca. 5 100 Denk-Token (Denkmodus an) – für eine so einfache Karte viel; Antwort 2 dauerte 34 s. Die Karte hätte etwas größer sein können (z. B. zwei Dateien).
  Qwen hat die `///`-Kommentare für die Funktionen weggelassen (Regel 9). Das prüft die Prüfliste künftig mit `grep -L "///"`.
- **Folgerung:** Kartengröße ca. 1,5- bis 2-fach erhöhen; Denkmodus für reines Ausfüllen möglichst abschalten; Regel „Dokumentationskommentare für alle `pub`-Elemente" in der Karte ausdrücklich wiederholen.
