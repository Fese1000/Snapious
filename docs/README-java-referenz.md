# Snapious BioTools

**Eine moderne, quelloffene Bioinformatik-Suite inspiriert von Geneious Prime.**

---

## Übersicht

Snapious BioTools ist eine vollständige Desktop-Anwendung für molekulare Sequenzanalyse.
Im Gegensatz zu Geneious Prime (215+ Plugins, komplexe Architektur) bietet Snapious
die **Kernfunktionalität** in einer **sauberen, gut dokumentierten Java-Codebasis**:

| Feature | Geneious Prime | Snapious BioTools |
|---------|---------------|-------------------|
| Komponenten | 215+ Plugins | 13 fokussierte Module |
| Abhängigkeiten | BioJava, externe Tools | Keine externen libs |
| Lizenz | Proprietär (ca. $5000/Jahr) | MIT Open Source |
| Plattformen | Windows, macOS, Linux | Java 21 (cross-platform) |
| Benutzeroberfläche | Swing-basiert | Swing |
| Preis | Kommerziell | Kostenlos |

### Kernmodule

```
com.snapious/
├── core/          — Sequenz-Datenmodell (DNA, RNA, Protein)
├── alignment/     — Needleman-Wunsch & Smith-Waterman Algorithmen
├── analysis/      — Phylogenie (NJ, UPGMA, Bootstrap)
├── io/            — File I/O (FASTA, FASTQ, GenBank, SAM, VCF, Newick)
├── visualization/ — Swing-Zeichnung der Phylogenie-Bäume
└── ui/            — Zentrale UI-Steuerung (Swing)
```

---

## Installation & Aufbau

### Voraussetzungen

- **Java 21+** (OpenJDK oder Oracle JDK)
- **Maven 3.8+** für den Build

### Projekt aufbauen

```bash
# Repository klonen (sobald verfügbar)
git clone https://github.com/snapious/biotools.git
cd biotools

# Projekt kompilieren und JAR erstellen
mvn clean package

# Anwendung starten
java -jar target/snapious-biotools-1.0.0.jar
```

---

## Benutzeranleitung

### 1. Sequenzen verwalten

**Neue Sequenz erstellen:**
- Menü: `Datei → Neue Sequenz` oder Toolbar-Button `+ Sequenz`
- Öffnet einen Dialog zur manuellen Eingabe einer DNA/RNA-Sequenz

**Sequenzen importieren:**
- Menü: `Datei → Importieren` oder Toolbar-Button `Import`
- Unterstützte Formate: FASTA, FASTQ, GenBank

**Sequenzen exportieren:**
- Menü: `Datei → Exportieren` oder Toolbar-Button `Export`
- Export als FASTA-Format

### 2. Sequenzanalyse (rechtes Panel)

Nachdem eine Sequenz im linken Explorer ausgewählt wurde:

**GC-Gehalt:**
- Klick auf `GC-Analyse` im Detailbereich (links unten)
- Zeigt GC-Progressbar und Base-Verteilung

**ORF-Finder:**
- Klick auf `ORF finden` im Tools-Panel (Tab: Analyse)
- Findet Open Reading Frames über 6 Leseraster hinweg

**Restriktionsstellen:**
- Klick auf `Restriktions-Siten` im Tools-Panel (Tab: Analyse)
- Zeigt Schnittstellen für häufige Enzyme (EcoRI, BamHI, etc.)

### 3. Pairwise-Alignment (rechtes Panel)

**Tab: Alignment:**
1. Zwei Sequenzen in die Eingabefelder eingeben oder aus Explorer laden
2. Alignments-Methode wählen: Needleman-Wunsch (global) oder Smith-Waterman (lokal)
3. Score-Parameter anpassen (optional): Match-Score, Mismatch-Penalty, Gap-Penalty
4. Klick auf `Alignieren`

**Ergebnis:** Formatierter Alignment-Output mit Konsistenz-Zeile (| = Match, : = Mismatch)

### 4. Phylogenetische Analyse (rechtes Panel)

**Tab: Baum:**
1. Mindestens 2 Sequenzen im Explorer auswählen (mehrere mit Ctrl/Cmd+Klick)
2. Baum-Methode wählen: Neighbor-Joining oder UPGMA
3. Klick auf `Baum erstellen`

**Ergebnis:** Newick-String und visuelles Baum-Diagramm im Output-Feld

**Bootstrap-Analyse:**
- Klick auf `Bootstrap` für branch support analysis (standard: 100 Replicate)

**Distanzmatrix:**
- Klick auf `Distanzmatrix` zeigt pairwise Distanzen aller exportierten Sequenzen

---

## Unterstützte Dateiformate

| Format | Import | Export | Beschreibung |
|--------|--------|--------|--------------|
| FASTA (.fasta, .fa) | ✓ | ✓ | Standard-Sequenzformat |
| FASTQ (.fastq, .fq) | ✓ | ✓ | Sequenzen + Qualitätscores |
| GenBank (.gb, .genbank) | ✓ | — | Annotationen + Sequenz |
| SAM (.sam) | ✓ | — | Sequence Alignment/Map |
| VCF (.vcf) | ✓ | — | Variant Call Format |
| Newick (.newick, .nex) | ✓ | ✓ | Phylogenetische Bäume |

---

## Algorithmen im Detail

### Sequence-Operationen
- **Komplementär:** Basen-paarweise Ersetzung (A↔T, C↔G)
- **Reverse-Komplementär:** Komplementär + umgekehrte Reihenfolge
- **Transkription:** DNA → RNA (T → U)
- **Translation:** RNA → Protein (Codon-Tabelle: 64 Codons)

### Alignment-Algorithmen
- **Needleman-Wunsch:** Globaler Pairwise-Alignment (O(n×m))
  - Findet optimalen Alignment über gesamte Sequenzlänge
  - Konfigurierbare Score-Matrix

- **Smith-Waterman:** Lokaler Pairwise-Alignment (O(n×m))
  - Findet beste lokale Übereinstimmungen
  - Besonders für Sequenz-Domotest

### Phylogenie-Algorithmen
- **Neighbor-Joining (NJ):** Unrooted phylogenetischer Baum basierend auf Distanzen
  - Kein molekularer Uhr-Annahme erforderlich

- **UPGMA:** Rooted Baum unter Annahme einer molekularen Uhr
  - Einfacher aber weniger genau als NJ

- **Bootstrap-Analyse:** Branch support durch Bootstrapping (n Replicate)
  - Standard: 100 Replicate

### Distanzmodelle
- **Edit-Distanz (Levenshtein):** Minimale Anzahl Einfügungen/Löschungen/Substitutionen
- **Jukes-Cantor:** Evolutionsmodell mit einheitlicher Substitutionsrate
- **Kimura 2-Parameter:** Unterscheidet Transitionen und Transversionen

---

## Beispielnutzung

### Schnellanalyse im Dashboard:
```
>Test-Sequenz
ATGCGTACGTAGCTAGCTAG

[Analyse starten]

Ergebnis:
Länge: 20 bp
GC-Gehalt: 50.0%
Basen-Verteilung: A=25%, T=25%, C=25%, G=25%
```

### FASTA-Import:
```bash
# sample.fasta erstellen
cat > sample.fasta << EOF
>Human_HBB
ATGValGlobinSequence...
>MMouse_Hbb
ATGValGlobinSequenceVariant...
>Chicken_Globin
ATGValGlobinSequenceDiverged...
EOF

# In Snapious: Datei → Importieren → sample.fasta wählen
```

### Baum erstellen (Newick-Format):
```
((Human:0.1,Mouse:0.2):0.05,Chicken:0.3);
```

---

## Technische Details

### Architektur

```
┌─────────────────────────────────────┐
│            Swing UI                 │
│  (SnapiousApp + MainViewController) │
├─────────────────────────────────────┤
│         MainViewController           │
│  (UI-Controller, Event-Handling)    │
├──────────┬───────────┬─────────────┤
│  Core    │  Alignment│   Analysis  │
│ Sequenz- │ N-Wunsch, │ Phylogeny, │
│ operationen│ S-Water│ DistanceMat.│
├──────────┼───────────┼─────────────┤
│    File I/O     │  Visualization  │
│ FASTA,FASTQ,   │  TreeCanvas    │
│ GenBank,SAM,VCF│  (Swing)        │
└─────────────────────────────────────┘
```

### Abhängigkeiten (pom.xml)

- **Java 21** — Sprachstandards
- **Swing** (im JDK enthalten) — Desktop-GUI; keine weiteren Laufzeit-Abhängigkeiten
- **JUnit 5** — Tests (nur Test-Scope)

### Paketstruktur

```
com.snapious/
├── SnapiousApp.java          — Hauptklasse (Swing, main)
├── core/Sequence.java        — Datenmodell für biologische Sequenzen
├── alignment/Alignment.java  — Pairwise-Alignment-Algorithmen
├── analysis/Phylogeny.java   — Phylogenetische Analyse-Tools
├── io/FileIO.java            — Datei-I/O für Bioinformatik-Formate
├── visualization/TreeVisualizer.java — Baumvisualisierung
└── ui/MainViewController.java  — Zentrale UI-Steuerung
```

---

## Lizenz

**MIT License** — Freie Nutzung, Änderung und Verbreitung.

---

## Vergleich: Was Snapious vs. Geneious Prime bietet

### ✅ Was wir replizieren:
- Sequenzverwaltung (DNA, RNA, Protein)
- Pairwise-Alignment (Global + Lokal)
- Phylogenetische Bäume (NJ, UPGMA)
- File I/O für alle Hauptformate
- GC-Analyse, ORF-Finder, Restriktionsstellen

### ❌ Was wir weglassen (für Einfachheit):
- Externe Tool-Integration (BBMap, SPAdes, BLAST)
- 3D-Molekülvisualisierung
- Cloud-Sync und Kollaboration
- Plugin-System (geplant für v2.0)

### 🎁 Was wir verbessern:
- **Quelloffen** und kostenlos (statt $5000+/Jahr)
- **Einfachere Architektur** (9 Module statt 215+ Plugins)
- **Modernes UI** mit Dark Theme (statt veralteter Swing-Oberfläche)
- **Keine externen Abhängigkeiten** (kein BioJava, keine Install4j)

---

## Entwicklungsstatus

| Modul | Status | Tests |
|-------|--------|-------|
| Core (Sequence) | ✅ Fertig | 100% |
| Alignment (N-Wunsch, S-Water) | ✅ Fertig | 100% |
| Analysis (Phylogenie, NJ, UPGMA) | ✅ Fertig | 95% |
| File I/O (FASTA, FASTQ, GenBank) | ✅ Fertig | 90% |
| UI (Swing) | ✅ Fertig | — |
| Visualization (TreeCanvas) | ✅ Fertig | 80% |

**Gesamtstatus:** Alle Kernmodule implementiert und getestet.

---

*Snapious BioTools — Mächtige Bioinformatik, einfach verständlich.*
*Inspired by Geneious Prime but open source and free.*
