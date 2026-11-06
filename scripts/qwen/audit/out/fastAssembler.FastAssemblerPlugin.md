GENEIOUS-FUNKTIONEN:
- De-novo-Assemblierung von Sequenzen aus Reads
- Referenzbasierte Assemblierung mit Gaps und Introns
- Chloroplasten-Assemblierung aus plastomischen Daten
- PacBio-Datenanalyse und Konsensus-Erstellung
- Entdeckung von Fusionen und neuartigen Introns
- Kartierung gepaarter Reads zur Verbesserung der Assemblierung
- Erkennung struktureller Varianten und großer Deletionen
- Generierung simulierter Leseergebnisse zur Testung
- Blast-Ergebnisse für Contig-Hits und Gap-Analyse
- Anpassung von Annotationen basierend auf Lückeneinfügungen

ABGLEICH:
- De-novo-Assemblierung von Sequenzen aus Reads | Kern: ja | App: ja | `assembly.rs` (assemble_reads), `ngs/spades.rs`
- Referenzbasierte Assemblierung mit Gaps und Introns | Kern: nein | App: nein | `read_map.rs` unterstützt nur Mapping ohne Lücken; keine Assemblierung mit Gaps/Introns vorhanden
- Chloroplasten-Assemblierung aus plastomischen Daten | Kern: nein | App: nein | Keine spezifische Unterstützung für Plastome oder Organellen-spezifische Assemblierung
- PacBio-Datenanalyse und Konsensus-Erstellung | Kern: ja (teilweise) | App: nein | `io/fastq.rs` und `ngs/fastq_stream.rs` lesen FASTQ (inkl. .gz), aber keine spezifische PacBio-Verarbeitung oder Konsensus-Bildung für lange Reads über `assembly.rs` hinaus
- Entdeckung von Fusionen und neuartigen Introns | Kern: nein | App: nein | Keine Funktion zur Erkennung von Genfusionen oder neuartigen Introns
- Kartierung gepaarter Reads zur Verbesserung der Assemblierung | Kern: ja (teilweise) | App: nein | `ngs/mapper.rs` und `coverage.rs` unterstützen Mapping, aber keine explizite Paired-End-Verbesserung der Assemblierung im `assembly.rs`
- Erkennung struktureller Varianten und großer Deletionen | Kern: ja (teilweise) | App: nein | `variants.rs` und `ngs/pileup.rs` erkennen SNVs/einfache Varianten, aber keine strukturellen Varianten (SVs) oder große Deletionen im Kontext der Assemblierung
- Generierung simulierter Leseergebnisse zur Testung | Kern: ja (teilweise) | App: nein | `cloning/pcr.rs` simuliert PCR, aber keine generelle Simulation von Sequenzier-Leseergebnissen (Reads) aus einer Referenz
- Blast-Ergebnisse für Contig-Hits und Gap-Analyse | Kern: ja (teilweise) | App: ja | `bio/blast.rs` und `bio/blast_remote.rs` (blast_search, blast_remote); Gap-Analyse im Kontext der Assemblierung nicht explizit als Funktion abgebildet
- Anpassung von Annotationen basierend auf Lückeneinfügungen | Kern: ja (teilweise) | App: nein | `model/feature.rs` und `io/gff3_write.rs` unterstützen Annotationen, aber keine automatische Anpassung von Features basierend auf Lücken in einer Assemblierung
