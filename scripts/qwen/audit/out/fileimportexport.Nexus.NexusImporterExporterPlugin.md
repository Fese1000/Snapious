GENEIOUS-FUNKTIONEN:
- Importiert Nexus-Dokumente in Geneious Prime
- Exportiert Sequenzen im Nexus-Dateiformat
- Bietet Option für interleave-formatierte Ausgabe
- Ermöglicht Export ohne Metakommentare (Metadata)
- Ersetzt Leerzeichen in Namen durch Unterstriche
- Setzt Anführungszeichen für Namen mit Leerzeichen
- Hilft bei Kompatibilitätsproblemen beim Dateilesevorgang
- Verarbeitet Nexus-spezifische Formatierungsanforderungen

ABGLEICH:
- Importiert Nexus-Dokumente in Geneious Prime | Kern: ja | App: ja | io/nexus.rs
- Exportiert Sequenzen im Nexus-Dateiformat | Kern: ja | App: ja | io/nexus.rs
- Bietet Option für interleave-formatierte Ausgabe | Kern: ja | App: nein | io/nexus.rs (liest/schreibt, aber keine spezifische UI-Option für Interleave genannt)
- Ermöglicht Export ohne Metakommentare (Metadata) | Kern: ja | App: nein | io/nexus.rs (schreibt, aber keine explizite Option zur Metadaten-Unterdrückung in der Schnittstelle)
- Ersetzt Leerzeichen in Namen durch Unterstriche | Kern: ja | App: nein | io/nexus.rs (Implementierungsdetail, nicht als separate Funktion in der Schnittstelle)
- Setzt Anführungszeichen für Namen mit Leerzeichen | Kern: ja | App: nein | io/nexus.rs (Implementierungsdetail, nicht als separate Funktion in der Schnittstelle)
- Hilft bei Kompatibilitätsproblemen beim Dateilesevorgang | Kern: ja | App: nein | io/nexus.rs (Implementierungsdetail/Robustheit, keine separate Funktion)
- Verarbeitet Nexus-spezifische Formatierungsanforderungen | Kern: ja | App: ja | io/nexus.rs
