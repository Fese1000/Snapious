GENEIOUS-FUNKTIONEN:
- Abruf von Genbank-Dokumentationen und Zusammenfassungen
- Zugriff auf NCBI-Gendatenbankdienstleistungen und Taxonomie
- Durchführung von BLAST-Suchen (Nukleotid und Protein)
- Verwaltung lokaler und Web-BLAST-Datenbanken
- Import und Export von GenBank-Formaten (Text/XML)
- Anzeige von BLAST-Ergebnismengen und Hit-Dokumenten
- Abruf von PopSet-Dokumentationen und Zusammenfassungen
- Suche in PubMed-Publikationsdatenbanken (BibTeX)
- Verarbeitung von Genom-Zusammenfassungsdokumenten
- Verwaltung von SNP-Dokumentationen und Kontig-Intervallen

ABGLEICH:
- Abruf von Genbank-Dokumentationen und Zusammenfassungen | Kern: ja | App: ja | io/genbank.rs, db/ncbi.rs
- Zugriff auf NCBI-Gendatenbankdienstleistungen und Taxonomie | Kern: ja | App: ja | db/ncbi.rs
- Durchführung von BLAST-Suchen (Nukleotid und Protein) | Kern: ja | App: ja | bio/blast.rs, blast_search
- Verwaltung lokaler und Web-BLAST-Datenbanken | Kern: teilweise | App: ja | bio/blast.rs (lokal), db/cache.rs; fehlt: Datenbank-Verwaltungsoberfläche für lokale DBs
- Import und Export von GenBank-Formaten (Text/XML) | Kern: ja | App: ja | io/genbank.rs, export_document
- Anzeige von BLAST-Ergebnismengen und Hit-Dokumenten | Kern: ja | App: ja | bio/blast.rs, blast_search; fehlt: dedizierte Ergebnisansicht für Hit-Dokumente
- Abruf von PopSet-Dokumentationen und Zusammenfassungen | Kern: ja | App: nein | db/ncbi.rs (PopSet nicht spezifiziert)
- Suche in PubMed-Publikationsdatenbanken (BibTeX) | Kern: ja | App: nein | db/ncbi.rs (PubMed nicht spezifiziert)
- Verarbeitung von Genom-Zusammenfassungsdokumenten | Kern: ja | App: ja | db/ncbi.rs, summary
- Verwaltung von SNP-Dokumentationen und Kontig-Intervallen | Kern: ja | App: teilweise | variants.rs, coverage.rs; fehlt: explizite SNP-Datenbankverwaltung
