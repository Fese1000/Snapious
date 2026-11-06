GENEIOUS-FUNKTIONEN:
- Generiert optimierte Nukleotidsequenzen für heterologe Expression
- Tauscht Codonen basierend auf Zielorganismus-Codonnutzungstabelle aus
- Vermeidet spezifische Restriktionsschnittstellen in der Ergebnissequenz
- Schließt benutzerdefinierte Sequenzmotive (Motifs) aus dem Ergebnis aus
- Führt Rückübersetzung von Protein zu Nukleotidsequenz durch
- Generiert mehrere ko-optimierte Kopien mit unterschiedlichen Codonen
- Berechnet den Codon-Anpassungsindex (CAI) für Sequenzen
- Ignoriert annotierte Übersetzungsausnahmen bei der Optimierung
- Verarbeitet DNA-Sequenzen mit ambiguosen Nukleotidcodes
- Optimiert ausgewählte Annotationen oder ganze Sequenzen
- Berücksichtigt verschiedene genetische Codes für Quelle und Ziel
- Vermeidet häufige Codonen basierend einem festgelegten Schwellenwert

ABGLEICH:
- Generiert optimierte Nukleotidsequenzen für heterologe Expression | Kern: ja | App: ja | bio/codon_optimize.rs
- Tauscht Codonen basierend auf Zielorganismus-Codonnutzungstabelle aus | Kern: ja | App: ja | bio/codon_optimize.rs
- Vermeidet spezifische Restriktionsschnittstellen in der Ergebnissequenz | Kern: nein | App: nein | (Funktionalität fehlt im beschriebenen Codon-Optimierer; cloning/enzymes.rs existiert, aber keine Integration in die Optimierung)
- Schließt benutzerdefinierte Sequenzmotive (Motifs) aus dem Ergebnis aus | Kern: nein | App: nein | (Funktionalität fehlt im beschriebenen Codon-Optimierer; bio/motif.rs existiert, aber keine Integration in die Optimierung)
- Führt Rückübersetzung von Protein zu Nukleotidsequenz durch | Kern: ja | App: ja | bio/codon_optimize.rs (implizit in "Protein ... umsetzen")
- Generiert mehrere ko-optimierte Kopien mit unterschiedlichen Codonen | Kern: ja | App: nein | (Snapious-Code beschreibt nur "umsetzen", nicht das Generieren mehrerer alternativer Varianten/Kopien)
- Berechnet den Codon-Anpassungsindex (CAI) für Sequenzen | Kern: ja | App: ja | bio/codon_usage.rs
- Ignoriert annotierte Übersetzungsausnahmen bei der Optimierung | Kern: ja | App: nein | (Snapious-Code erwähnt keine Handhabung von Annotationsausnahmen/Exceptions)
- Verarbeitet DNA-Sequenzen mit ambiguosen Nukleotidcodes | Kern: ja | App: ja | bio/codon_optimize.rs (implizit durch Nutzung von IUPAC/Codes in anderen Modulen wie bio/motif.rs und cloning/enzymes.rs, jedoch nicht explizit als Feature der Optimierung genannt; streng genommen "teilweise" oder "nein" wenn strikt nach Text. Da `bio/codon_optimize.rs` nur "codierende DNA" nennt und keine explizite IUPAC-Verarbeitung für die Optimierung steht, ist es eher "nein" oder "teilweise". Ich setze "teilweise", da `bio/motif.rs` und `cloning/enzymes.rs` IUPAC unterstützen, die Optimierungsroutine selbst aber nicht explizit als IUPAC-fähig beschrieben wird. Streng: "nein" da nicht im Text der Datei genannt.) -> Korrektur: Der Prompt sagt "Sei streng: im Zweifel 'nein'". Die Datei `bio/codon_optimize.rs` sagt nur "codierende DNA". Sie erwähnt keine IUPAC-Codes explizit. Also: nein.
- Optimiert ausgewählte Annotationen oder ganze Sequenzen | Kern: ja | App: ja | bio/codon_optimize.rs (implizit durch "Sequenz ... umsetzen"; Annotationen werden nicht explizit als Ziel der Optimierung genannt, nur die Sequenz. Streng: "teilweise" oder "nein". Da `ops/phase2.rs` Klonierung erwähnt und `bio/codon_optimize.rs` nur die Sequenz, ist die Annotation-Optimierung nicht explizit. Ich setze "teilweise" für die Sequenz, aber für Annotationen "nein". Die Funktion ist eine Kombination. Ich bewerte die Gesamtfunktion: "Optimiert ausgewählte Annotationen" -> nein. "oder ganze Sequenzen" -> ja. Also insgesamt: teilweise.)
- Berücksichtigt verschiedene genetische Codes für Quelle und Ziel | Kern: ja | App: ja | bio/codon_optimize.rs (implizit durch `bio/genetic_code.rs` und `bio/codon_tables.rs`, aber die Optimierungsroutine selbst nennt nicht explizit "verschiedene Codes für Quelle und Ziel" als Parameter, sondern nutzt eine Tabelle. Streng: "teilweise".)
- Vermeidet häufige Codonen basierend einem festgelegten Schwellenwert | Kern: ja | App: nein | (Snapious-Code erwähnt "Codon-Häufigkeitstabelle", aber keinen Schwellenwert zur Vermeidung häufiger Codonen explizit.)

Korrektur der strengen Auswertung basierend auf dem Text:

- Generiert optimierte Nukleotidsequenzen für heterologe Expression | Kern: ja | App: ja | bio/codon_optimize.rs
- Tauscht Codonen basierend auf Zielorganismus-Codonnutzungstabelle aus | Kern: ja | App: ja | bio/codon_optimize.rs
- Vermeidet spezifische Restriktionsschnittstellen in der Ergebnissequenz | Kern: ja | App: nein | (Fehlende Integration von `cloning/enzymes.rs` in die Optimierung)
- Schließt benutzerdefinierte Sequenzmotive (Motifs) aus dem Ergebnis aus | Kern: ja | App: nein | (Fehlende Integration von `bio/motif.rs` in die Optimierung)
- Führt Rückübersetzung von Protein
