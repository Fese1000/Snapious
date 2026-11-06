GENEIOUS-FUNKTIONEN:
- Simuliert den Verlauf von DNA-Sequenzen auf einem Elektrophorese-Gel
- Zeigt virtuelle Gelergebnisse für ausgewählte Sequenzdokumente an
- Analysiert Bandenintensitäten in den simulierten Gel-Lanes
- Unterstützt die Verdauung von DNA durch einzelne Restriktionsenzyme
- Bietet verschiedene Farbschemata zur visuellen Darstellung der Bands
- Lädt und rendert vordefinierte oder benutzerdefinierte DNA-Leitern
- Zeigt Supercoiled-DNA als lineare Fragmente zur besseren Analyse an
- Speichert und lädt benutzerdefinierte Leitern über Textdateien
- Zeigt Bandennamen an, wenn man mit der Maus darüber fährt
- Gruppiert Sequenzen in Lanes basierend auf Restriktionsmarkierungen
- Zeigt Bandenpositionen in Basenpaaren oder Kilobasenpaaren an

ABGLEICH:
- Simuliert den Verlauf von DNA-Sequenzen auf einem Elektrophorese-Gel | Kern: ja | App: gel_ladder, gel_lane | cloning/gel.rs
- Zeigt virtuelle Gelergebnisse für ausgewählte Sequenzdokumente an | Kern: ja | App: gel_ladder, gel_lane | cloning/gel.rs
- Analysiert Bandenintensitäten in den simulierten Gel-Lanes | Kern: nein | App: nein | cloning/gel.rs (nur Positions-/Größenberechnung, keine Intensitätsanalyse)
- Unterstützt die Verdauung von DNA durch einzelne Restriktionsenzyme | Kern: ja | App: digest_document, enzyme_sites | cloning/fragment.rs, cloning/enzymes.rs
- Bietet verschiedene Farbschemata zur visuellen Darstellung der Bands | Kern: ja | App: nein | cloning/gel.rs (keine Farbschemata implementiert, nur geometrische Darstellung)
- Lädt und rendert vordefinierte oder benutzerdefinierte DNA-Leitern | Kern: ja | App: gel_ladder | cloning/gel.rs
- Zeigt Supercoiled-DNA als lineare Fragmente zur besseren Analyse an | Kern: ja | App: nein | cloning/gel.rs (keine spezifische Handhabung von Supercoiling-Topologien für die Migration simuliert)
- Speichert und lädt benutzerdefinierte Leitern über Textdateien | Kern: ja | App: nein | cloning/gel.rs (keine Datei-I/O für Leitern definiert)
- Zeigt Bandennamen an, wenn man mit der Maus darüber fährt | Kern: ja | App: nein | cloning/gel.rs (keine Hover-Interaktion spezifiziert)
- Gruppiert Sequenzen in Lanes basierend auf Restriktionsmarkierungen | Kern: ja | App: gel_lane | cloning/gel.rs (manuelle Zuordnung zu Lanes möglich, keine automatische Gruppierung nach Markierungen)
- Zeigt Bandenpositionen in Basenpaaren oder Kilobasenpaaren an | Kern: ja | App: gel_ladder, gel_lane | cloning/gel.rs
