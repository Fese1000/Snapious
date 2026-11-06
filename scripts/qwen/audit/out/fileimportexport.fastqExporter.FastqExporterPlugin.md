GENEIOUS-FUNKTIONEN:
- Exportiert Sequenzen im FastQ-Format (Sanger-Qualitätswerte)
- Komprimiert FastQ-Dateien zur Speicherplatzersparnis
- Exportiert gepaarte Forward/Reverse-Dateien für paired-end Reads
- Ersetzt Leerzeichen in Sequenznamen durch Unterstriche
- Stellt sicher, dass nur Nukleotidsequenzen exportiert werden
- Zeigt Fortschritt des Exportvorgangs an (Exported %,d of %,d)
- Bietet Option „Sequence is its own mate“ für unpaired Reads
- Unterstützt das Exportieren von FastA-Paired-Dateien (komprimiert)
- Integriert sich als Plugin in Geneious Prime für Dateihandling

ABGLEICH:
- Exportiert Sequenzen im FastQ-Format (Sanger-Qualitätswerte) | Kern: ja | App: ja | io/fastq.rs
- Komprimiert FastQ-Dateien zur Speicherplatzersparnis | Kern: nein | App: nein | Keine Komprimierungsfunktion im Export-Modul genannt (nur Lesen/Schreiben)
- Exportiert gepaarte Forward/Reverse-Dateien für paired-end Reads | Kern: ja | App: nein | Keine spezifische Funktion für gepaarte Export-Dateien vorhanden (nur einzelnes FASTQ)
- Ersetzt Leerzeichen in Sequenznamen durch Unterstriche | Kern: ja | App: nein | Keine automatische Namensbereinigung im Export-Modul erwähnt
- Stellt sicher, dass nur Nukleotidsequenzen exportiert werden | Kern: ja | App: nein | Keine explizite Filterlogik für Nukleotid-Typen im Export-Modul genannt
- Zeigt Fortschritt des Exportvorgangs an (Exported %,d of %,d) | Kern: ja | App: ja | view/progress_format.rs (allgemeine Fortschrittsanzeige)
- Bietet Option „Sequence is its own mate“ für unpaired Reads | Kern: ja | App: nein | Keine spezifische UI-Option oder Logik für "self-mate" im Export erwähnt
- Unterstützt das Exportieren von FastA-Paired-Dateien (komprimiert) | Kern: ja | App: ja | io/fasta.rs (Lesen/Schreiben), jedoch keine explizite "Paired-File"-Export-Funktion im App-Interface genannt
- Integriert sich als Plugin in Geneious Prime für Dateihandling | Kern: ja | App: nein | Snapious nutzt `plugin.rs` für externe Programme, aber keine spezifische Integration als "Geneious-Plugin" oder Dateihandler-Interface wie beschrieben
