GENEIOUS-FUNKTIONEN:
- Bietet Fasta-formatierte Ansicht von Sequenzen und Alignments
- Zeigt Sequenzdaten im Fasta-Format an
- Warnung bei zu großen Sequenzen für die Anzeige
- Empfehlung zur direkten Fasta-Format-Exportierung bei großen Daten

ABGLEICH:
- Bietet Fasta-formatierte Ansicht von Sequenzen und Alignments | Kern: ja | App: ja | io/fasta.rs (Lesen/Schreiben), model/sequence.rs, view/seq_layout.rs
- Zeigt Sequenzdaten im Fasta-Format an | Kern: ja | App: ja | io/fasta.rs, view/seq_layout.rs
- Warnung bei zu großen Sequenzen für die Anzeige | Kern: nein | App: nein | Keine explizite Größengrenzen-Prüfung oder Warnlogik in der Liste gefunden (view/seq_layout.rs enthält nur Layout-Logik)
- Empfehlung zur direkten Fasta-Format-Exportierung bei großen Daten | Kern: nein | App: ja | export/text.rs (enthält Export-Funktionalität, aber keine spezifische "Empfehlungs"-Logik für große Daten; strikt genommen fehlt die UI-Logik der Empfehlung)
