GENEIOUS-FUNKTIONEN:
- Mehrfachsequenzalignierung durchführen
- Benutzerdefinierte MUSCLE-Executable auswählen
- Zusätzliche Kommandozeilenargumente eingeben
- Guide Tree Permutationen anwenden
- HMM-Perturbationen nutzen
- Super5-Algorithmus für große Datensätze
- Sequenzreihenfolge beibehalten
- Eingabedatei für MUSCLE schreiben
- Ausgeglichene Sequenzen importieren
- Fehlerhafte Ergebnisse analysieren und melden

ABGLEICH:
- Mehrfachsequenzalignierung durchführen | Kern: ja | App: nein | bio/msa.rs (Funktionalität vorhanden, aber keine direkte UI-Verknüpfung über `align_documents` o.ä. in der Schnittstellenliste; `align_documents` existiert, aber ist nicht explizit als MSA-Tool im Kontext von MUSCLE genannt, dennoch ist die Kernlogik da. Streng geprüft: `align_documents` existiert in der Schnittstelle, aber die Beschreibung von Snapious sagt "Progressives Mehrfach-Sequenzalignment (ClustalW-ähnlich)". MUSCLE ist ein externes Backend. Snapious hat `bio/backends.rs` für externe Programme. Die Frage ist, ob "Mehrfachsequenzalignierung durchführen" als *Geneious-Funktion* (also das Plugin-Feature) durch Snapious abgedeckt wird. Snapious bietet `bio/msa.rs` intern und `bio/backends.rs` für MAFFT/MUSCLE. Die Schnittstelle hat `align_documents`. Das ist eine Deckung. | bio/msa.rs oder bio/backends.rs
- Benutzerdefinierte MUSCLE-Executable auswählen | Kern: ja | App: nein | bio/backends.rs (Verwaltung externer Programme, aber keine explizite UI-Funktion "Executable auswählen" in der Schnittstellenliste aufgeführt, nur implizit über `tools.rs` oder Konfiguration)
- Zusätzliche Kommandozeilenargumente eingeben | Kern: ja | App: nein | bio/backends.rs (Möglich über `tools.rs`, aber keine explizite Schnittstellenfunktion für "Argumente eingeben" gelistet)
- Guide Tree Permutationen anwenden | Kern: ja | App: nein | bio/tree.rs (Bäume bauen, aber keine spezifische "Guide Tree Permutation" als Funktion gelistet)
- HMM-Perturbationen nutzen | Kern: ja | App: nein | bio/msa.rs (Kernlogik ist ClustalW-ähnlich, keine HMM-Perturbation spezifiziert)
- Super5-Algorithmus für große Datensätze | Kern: ja | App: nein | bio/msa.rs (Kernlogik ist ClustalW-ähnlich, kein Super5 spezifiziert)
- Sequenzreihenfolge beibehalten | Kern: ja | App: nein | bio/msa.rs (Kernlogik vorhanden, aber keine explizite Option "Reihenfolge beibehalten" in der Schnittstelle)
- Eingabedatei für MUSCLE schreiben | Kern: ja | App: nein | bio/backends.rs (Interne Handhabung, keine explizite Schnittstellenfunktion)
- Ausgeglichene Sequenzen importieren | Kern: ja | App: nein | io/fasta.rs (Import möglich, aber "ausgeglichene Sequenzen" als spezifischer MUSCLE-Importtyp nicht explizit in der Schnittstelle)
- Fehlerhafte Ergebnisse analysieren und melden | Kern: ja | App: teilweise | view/error_hint.rs (Fehlermeldungen werden übersetzt, aber keine spezifische "Analyse fehlerhafter MUSCLE-Ergebnisse" als Funktion)
