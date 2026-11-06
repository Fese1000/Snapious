GENEIOUS-FUNKTIONEN:
- Importiert PHYLIP-Ausrichtungsdokumente in das System
- Exportiert Dokumente im PHYLIP-Format für Kompatibilität
- Prüft auf ungültige Dateiformate und fehlende Daten
- Erkennt Sequenztypen (Nukleotide oder Aminosäuren)
- Validiert gleiche Länge aller Sequenzen in der Ausrichtung
- Warnt vor identischen Sequenznamen (Duplikate)
- Handhabung von Namenskonflikten bei Kürzung auf 10 Zeichen
- Prüft, ob Länge und Sequenzzahl größer als null sind
- Erkennt fehlende Sequenz-Token in der Eingabedatei
- Stellt sicher, dass keine nicht-biologischen Zeichen enthalten sind

ABGLEICH:
- Importiert PHYLIP-Ausrichtungsdokumente in das System | Kern: ja | App: ja | io/alignment_formats.rs (liest PHYLIP)
- Exportiert Dokumente im PHYLIP-Format für Kompatibilität | Kern: ja | App: ja | io/alignment_formats.rs (schreibt PHYLIP)
- Prüft auf ungültige Dateiformate und fehlende Daten | Kern: ja | App: nein | io/alignment_formats.rs (Validierung ist implizit im Parser, keine separate explizite Funktion "pruefe_ungueltiges_format" in der Schnittstelle)
- Erkennt Sequenztypen (Nukleotide oder Aminosäuren) | Kern: ja | App: nein | io/alignment_formats.rs (Typerkennung ist Teil des Leseprozesses, keine separate API-Funktion)
- Validiert gleiche Länge aller Sequenzen in der Ausrichtung | Kern: ja | App: nein | io/alignment_formats.rs (Validierung ist Teil des Leseprozesses, keine separate API-Funktion)
- Warnt vor identischen Sequenznamen (Duplikate) | Kern: ja | App: nein | io/alignment_formats.rs (Warnung ist Teil des Leseprozesses, keine separate API-Funktion)
- Handhabung von Namenskonflikten bei Kürzung auf 10 Zeichen | Kern: ja | App: nein | io/alignment_formats.rs (Logik ist im Parser integriert, keine separate API-Funktion)
- Prüft, ob Länge und Sequenzzahl größer als null sind | Kern: ja | App: nein | io/alignment_formats.rs (Validierung ist Teil des Leseprozesses, keine separate API-Funktion)
- Erkennt fehlende Sequenz-Token in der Eingabedatei | Kern: ja | App: nein | io/alignment_formats.rs (Validierung ist Teil des Leseprozesses, keine separate API-Funktion)
- Stellt sicher, dass keine nicht-biologischen Zeichen enthalten sind | Kern: ja | App: nein | io/alignment_formats.rs (Validierung ist Teil des Leseprozesses, keine separate API-Funktion)
