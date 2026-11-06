GENEIOUS-FUNKTIONEN:
- Sequenzalignierung an Referenzsequenzen durchführen
- De-novo-Assemblierung von Reads zu Contigs erstellen
- Wiederholungssequenzen (Repeats) in Sequenzen finden und annotieren
- Primer-Sequenzen in DNA-Daten identifizieren und markieren
- Vektorsequenzen (z.B. UniVec) in Sequenzen screenen
- Sequenzenden vor der Assemblierung abschneiden (Trimmen)
- Sequenzen basierend auf Binning-Parametern gruppieren
- Ausgerichtete Reads in einer Liste speichern oder anzeigen
- Assemblierungsberichte als HTML-Dokument exportieren und anzeigen
- Richtung der Reads festlegen oder umkehren
- Konsensussequenzen aus Contigs generieren und speichern

ABGLEICH:
- Sequenzalignierung an Referenzsequenzen durchführen | Kern: ja | App: ja | bio/align.rs (align_documents, alignment), ngs/mapper.rs (map_reads_to_reference)
- De-novo-Assemblierung von Reads zu Contigs erstellen | Kern: ja | App: ja | bio/assembly.rs (assemble_reads), ngs/spades.rs
- Wiederholungssequenzen (Repeats) in Sequenzen finden und annotieren | Kern: ja | App: ja | bio/repeats.rs (find_repeats)
- Primer-Sequenzen in DNA-Daten identifizieren und markieren | Kern: ja | App: ja | primer/annotate.rs (Primerbindestellen als Features eintragen)
- Vektorsequenzen (z.B. UniVec) in Sequenzen screenen | Kern: nein | App: nein | Keine spezifische Datenbank oder Funktion für Vektorscreening (wie UniVec) vorhanden; site_check prüft nur Enzymschnittstellen
- Sequenzenden vor der Assemblierung abschneiden (Trimmen) | Kern: ja | App: ja | ngs/trim.rs (trim_fastq)
- Sequenzen basierend auf Binning-Parametern gruppieren | Kern: nein | App: nein | Keine Funktion zum automatischen Gruppieren/Binnen von Sequenzen basierend auf Parametern vorhanden
- Ausgerichtete Reads in einer Liste speichern oder anzeigen | Kern: ja | App: ja | bio/align.rs (alignment), ngs/pileup.rs, view/seq_layout.rs
- Assemblierungsberichte als HTML-Dokument exportieren und anzeigen | Kern: ja | App: nein | export/text.rs bietet Markdown/Text, aber keine HTML-Generierung für Berichte
- Richtung der Reads festlegen oder umkehren | Kern: ja | App: teilweise | model/sequence.rs (Reverse-Komplement), ops/builtin.rs; keine explizite "Read-Richtung setzen"-Funktion in der App-Schnittstelle, aber Reverse-Komplement ist verfügbar
- Konsensussequenzen aus Contigs generieren und speichern | Kern: ja | App: ja | bio/align_stats.rs (alignment_consensus), bio/read_map.rs (Konsens aus Abdeckung bilden)
