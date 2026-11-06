GENEIOUS-FUNKTIONEN:
- Zugriff auf UniProt-Datenbank bei EBI
- Suche nach Proteinsequenzen und -funktionen
- Abfrage von Informationen aus Swiss-Prot, TrEMBL und PIR
- Nutzung der UniProt Search-Funktion innerhalb von Geneious
- Anwendung von Query-Optionen und Unteroptionen der Website-Suche
- Ausführung äquivalenter Suchen wie im UniProt-Webinterface
- Kombination von Suchoptionen über das Plus-Symbol
- Exakte Suche mit „is“ für eingegebenen Text
- Teilweise Suche mit „contains“ (Text + Wildcard)
- Verknüpfung von Suchzeilen durch AND oder OR
- Anzeige der UniProtKB Advanced Search-Optionen

ABGLEICH:
- Zugriff auf UniProt-Datenbank bei EBI | Kern: ja | App: ja | db/uniprot.rs
- Suche nach Proteinsequenzen und -funktionen | Kern: ja | App: ja | db/uniprot.rs (Suche), model/sequence.rs (Sequenz)
- Abfrage von Informationen aus Swiss-Prot, TrEMBL und PIR | Kern: ja | App: ja | db/uniprot.rs (REST-Schnittstelle liefert diese Daten)
- Nutzung der UniProt Search-Funktion innerhalb von Geneious | Kern: ja | App: ja | db/uniprot.rs
- Anwendung von Query-Optionen und Unteroptionen der Website-Suche | Kern: ja | App: teilweise | db/uniprot.rs (Parameter werden übergeben, aber komplexe UI-Optionen fehlen)
- Ausführung äquivalenter Suchen wie im UniProt-Webinterface | Kern: ja | App: ja | db/uniprot.rs
- Kombination von Suchoptionen über das Plus-Symbol | Kern: ja | App: nein | db/uniprot.rs (fehlende UI für komplexe Query-Konstruktion)
- Exakte Suche mit „is“ für eingegebenen Text | Kern: ja | App: teilweise | db/uniprot.rs (Syntax wird unterstützt, aber keine dedizierte UI-Steuerung)
- Teilweise Suche mit „contains“ (Text + Wildcard) | Kern: ja | App: teilweise | db/uniprot.rs (Syntax wird unterstützt, aber keine dedizierte UI-Steuerung)
- Verknüpfung von Suchzeilen durch AND oder OR | Kern: ja | App: nein | db/uniprot.rs (fehlende UI für logische Verknüpfung von Suchbegriffen)
