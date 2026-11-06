# Codon-Tabellen

Relative Häufigkeit jedes Codons innerhalb seiner Aminosäure („fraction“ der Spalte aus der Codon Usage Database).
Quelle: Kazusa DNA Research Institute, Codon Usage Database (https://www.kazusa.or.jp/codon/), abgerufen am 26.10.2026,
Format `showcodon.cgi?species=<id>&aa=1&style=N`:

| Datei | Organismus | species |
|---|---|---|
| ecoli.json | Escherichia coli W3110 | 316407 |
| human.json | Homo sapiens | 9606 |
| yeast.json | Saccharomyces cerevisiae | 4932 |
| mouse.json | Mus musculus | 10090 |

Die Tabellen werden im Kern mit `bio::codon_tables` geladen (`include_str!`).
