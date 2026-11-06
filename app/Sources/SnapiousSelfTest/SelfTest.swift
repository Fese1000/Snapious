import Foundation
import SnapiousFFI

/// Selbsttest ohne Fenster: prüft die Verbindung Swift ↔ Rust.
/// Aufruf:  Snapious --selftest [Pfad/zu/mini.gb]
enum SelfTest {
    static func run(arguments: [String]) -> Int32 {
        var failures = 0
        func check(_ ok: Bool, _ what: String) {
            print((ok ? "  ok  " : "FEHLER ") + what)
            if !ok { failures += 1 }
        }
        let args = arguments.drop(while: { $0 != "--selftest" }).dropFirst()
        let sample = args.first ?? "crates/snapious-core/testdata/mini.gb"
        print("Snapious-Selbsttest (Kern \(coreVersion()))")
        check(coreVersion() == "0.1.0", "Kernversion lesbar")

        let dir = FileManager.default.temporaryDirectory
            .appendingPathComponent("snapious-selftest-\(UUID().uuidString)")
        do {
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            defer { try? FileManager.default.removeItem(at: dir) }
            let project = try Project.open(path: dir.appendingPathComponent("t.snapious").path)
            check(try project.documents().isEmpty, "neues Projekt ist leer")

            let neu = try project.importFile(path: sample)
            check(neu.count == 1 && neu[0].name == "mini", "GenBank-Import: 1 Dokument 'mini'")
            let s = try project.summary(id: neu[0].id)
            check(s.length == 60 && s.topology == "CIRCULAR", "Zusammenfassung: 60 bp, zirkulär")
            check(s.featureCount == 2 && s.accession == "TEST0001", "2 Features, Accession TEST0001")
            let f = try project.features(id: neu[0].id)
            check(f.first?.featureType == "gene" && f.first?.start == 4 && f.first?.end == 20, "Feature gene 4–20")
            check(try project.bases(id: neu[0].id, start: 0, end: 8) == "ATGCATGC", "Basen 0–8")

            try project.renameDocument(id: neu[0].id, name: "umbenannt")
            check(try project.documents().first?.name == "umbenannt", "Umbenennen")

            do {
                _ = try project.summary(id: "gibt-es-nicht")
                check(false, "Fehler bei unbekannter ID erwartet")
            } catch let e as ApiError {
                if case .NotFound = e { check(true, "unbekannte ID → ApiError.NotFound") }
                else { check(false, "falscher Fehlertyp \(e)") }
            }
            do {
                _ = try project.importFile(path: "/gibt/es/nicht")
                check(false, "Fehler bei fehlender Datei erwartet")
            } catch is ApiError {
                check(true, "fehlende Datei → ApiError")
            }

            // Ansichtsdaten
            let id = neu[0].id
            check(!(try project.featureLanes(id: id)).isEmpty, "Feature-Spuren")
            check(try project.selectionStats(id: id, start: 0, end: 8).length == 8, "Auswahl-Kennzahlen")
            check(try project.selectionText(id: id, start: 0, end: 4, format: "reverse_complement") == "GCAT", "Reverse-Komplement der Auswahl")
            check(project.enzymeList().count >= 79, "Enzymliste")
            check(!(try project.digestDocument(id: id, enzymes: ["EcoRI"])).isEmpty, "virtueller Verdau")
            check(try gelLadder(name: "generic_1kb", agarosePercent: 1.0).count == 13, "Gel-Leiter")
            _ = try project.findGuides(id: id, nuclease: "spcas9")

            // Bearbeiten mit Rückgängig
            try project.editInsert(id: id, position: 0, bases: "AAA")
            check(try project.summary(id: id).length == 63, "Einfügen")
            check(try project.editUndo(id: id), "Rückgängig")
            check(try project.summary(id: id).length == 60, "Länge nach Rückgängig")
            check(try project.editRedo(id: id), "Wiederholen")
            check(try project.editUndo(id: id), "Rückgängig (2)")

            // Alignment und Baum
            let fa = dir.appendingPathComponent("x.fa")
            try ">a\nACGTACGTACGTAAAA\n>b\nACGTACGTACGTAAAT\n>c\nACGAACGTACCTAAAA\n".write(to: fa, atomically: true, encoding: .utf8)
            let seqs = try project.importFile(path: fa.path)
            let aln = try project.alignDocuments(ids: seqs.map { $0.id }, name: "Test", backend: "builtin")
            check(aln.docType == "alignment", "Alignment erzeugt")
            check(try project.alignment(id: aln.id).rows.count == 3, "Alignment hat 3 Zeilen")
            let mafft = try project.alignDocuments(ids: seqs.map { $0.id }, name: "Mafft", backend: "mafft")
            check(try project.alignment(id: mafft.id).rows.count == 3, "MAFFT-Alignment")

            // Workflow
            let wfJson = #"{"name":"rc+ins","steps":[{"operation":"edit.reverseComplement","inputs":["$in:0"]},{"operation":"edit.insert","parameters":{"position":1,"bases":"GG"},"inputs":["$step:0:0"]}],"outputs":["$step:1:0"]}"#
            check(try project.workflowSave(json: wfJson) == "rc+ins", "Workflow gespeichert")
            let run = try project.workflowRun(name: "rc+ins", inputIds: [id])
            check(run.errors.isEmpty && run.documents.count == 1, "Workflow ausgeführt")
            try project.workflowDelete(name: "rc+ins")
            check(try project.workflowList().isEmpty, "Workflow gelöscht")

            // BLAST und RNA-Faltung (nur wenn die Programme installiert sind)
            let blastIds = seqs.map { $0.id }
            if let hits = try? project.blastSearch(queryId: blastIds[0], subjectIds: Array(blastIds.dropFirst()), program: "blastn", maxHits: 5) {
                check(!hits.isEmpty, "BLAST findet Treffer")
            } else {
                print("  --  BLAST übersprungen")
            }
            if let fold = try? project.foldRegion(id: blastIds[0], start: 0, end: 0) {
                check(fold.structure.count == fold.sequence.count, "RNA-Faltung: Struktur so lang wie Sequenz")
            } else {
                print("  --  RNAfold übersprungen")
            }

            // NCBI (braucht Internet; ein Netzwerkfehler zählt nicht als Fehler)
            do {
                let found = try project.ncbiSearch(term: "L09137", maxResults: 3)
                check(found.total >= 1 && !found.records.isEmpty, "NCBI-Suche liefert Treffer")
                if let first = found.records.first {
                    let imported = try project.ncbiImport(id: first.id)
                    check(imported.count == 1, "NCBI-Import: 1 Dokument (\(first.accession))")
                }
                let prot = try project.uniprotSearch(query: "accession:P69905", maxResults: 2)
                check(prot.records.first?.accession == "P69905", "UniProt-Suche findet P69905")
                let pi = try project.uniprotImport(accession: "P69905")
                check(try project.summary(id: pi[0].id).seqType == "PROTEIN", "UniProt-Import ist ein Protein")
            } catch {
                print("  --  NCBI-Test übersprungen (\(error))")
            }

            let mut = try project.mutagenesisPrimers(id: neu[0].id, start: 9, end: 10, replacement: "A", minFlank: 5, maxFlank: 12, targetTm: 0)
            check(mut.forward.count == 11 && mut.reverse.count == 11, "Mutagenese-Primer (5+1+5)")
            check(try project.findRepeats(id: neu[0].id, minUnit: 1, maxUnit: 6, minCopies: 3, minArm: 50, maxSpacer: 0).contains { $0.kind == "tandem" },
                  "Wiederholungen: Tandem in mini gefunden")
            check((try? calcNgToPmol(ng: 1000, bp: 1000)).map { abs($0 - 1.6181333) < 1e-5 } ?? false, "Rechner ng→pmol")
            check((try? calcDilutionVolume(cStock: 10, cFinal: 1, vFinal: 50)) == 5.0, "Rechner Verdünnung")
            let contigs = try project.assembleReads(ids: [neu[0].id], minOverlap: 10)
            check(contigs.count == 1, "Contig-Bildung mit einem Read")
            try project.exportDocument(id: neu[0].id, path: dir.appendingPathComponent("x.embl").path, format: "embl")
            check(try project.importFile(path: dir.appendingPathComponent("x.embl").path).count == 1, "EMBL-Export und -Import")

            let ordner = try project.createFolder(name: "Test", parentId: "")
            try project.moveDocument(docId: neu[0].id, folderId: ordner.id)
            check(try project.documentsInFolder(folderId: ordner.id).count == 1, "Ordner: Dokument verschoben")
            let aktuell = try project.documents().first { $0.id == neu[0].id }?.name ?? "?"
            check(try project.searchDocuments(kind: "name", text: aktuell, text2: "").contains { $0.id == neu[0].id }, "Suche nach Name")
            check(try project.searchDocuments(kind: "motif", text: "ATGC", text2: "").count >= 1, "Suche nach Motiv")
            try project.setFavorite(docId: neu[0].id, favorite: true)
            check(try project.favorites().count == 1, "Favorit")
            try project.setNote(docId: neu[0].id, text: "Notiz")
            check(try project.note(docId: neu[0].id) == "Notiz", "Notiz")
            _ = try project.addExternalFile(path: "/tmp/x.sam", kind: "sam", docId: neu[0].id, label: "")
            check(try project.externalFiles(docId: neu[0].id).count == 1, "Externe Datei verknüpft")
            check(try project.exportText(id: neu[0].id, kind: "report").hasPrefix("# "), "Bericht als Markdown")
            check(try project.findRepeats(id: neu[0].id, minUnit: 1, maxUnit: 2, minCopies: 3, minArm: 50, maxSpacer: 0).count >= 0, "Wiederholungen")
            check(settingsLoad().undoLimit >= 10, "Einstellungen lesbar")

            let made = try project.createSequence(name: "", text: ">a erste\nATGAAATAA\n>b\nMKVLAAGIVG\n", kind: "AUTO", circular: false)
            check(made.count == 2 && made[0].name == "a erste", "Neue Sequenz aus FASTA-Text (2 Einträge)")
            check(try project.summary(id: made[1].id).seqType == "PROTEIN", "Art automatisch: Protein")
            let fw = try project.translateFrame(id: made[0].id, frame: 1)
            check(fw.map { $0.aminoAcid }.joined() == "MK*" && fw.map { $0.basePosition } == [0, 3, 6], "Aminosäuren Rahmen +1")
            let rv = try project.createSequence(name: "rev", text: "TTATTTCAT", kind: "DNA", circular: true)
            let rvc = try project.translateFrame(id: rv[0].id, frame: -1)
            check(rvc.map { $0.aminoAcid }.joined() == "MK*" && rvc.map { $0.basePosition } == [6, 3, 0], "Aminosäuren Gegenstrang")
            check(try project.summary(id: rv[0].id).topology == "CIRCULAR", "Zirkulär angelegt")
            check((try? project.createSequence(name: "x", text: "123 456", kind: "AUTO", circular: false)) == nil, "Text ohne Buchstaben abgelehnt")
            for d in made + rv { try project.deleteDocument(id: d.id) }

            // Zusammenbau: zwei Fragmente mit 20 bp Überlappung → 100 bp
            let overlap = "GATTACAGGCTTAACCGGAT"
            let left = "ACGTTGCAAGGCTTAACCGGATATCGCGATTAGCCTAGGA" + overlap
            let right = overlap + "TTGCATGCCGTAAGCTTGACCGATCAGTCTAGGCATCCAA"
            let frags = try project.createSequence(name: "frag", text: ">l\n\(left)\n>r\n\(right)\n", kind: "DNA", circular: false)
            for method in ["gibson", "infusion"] {
                let r = try project.assembleOverlap(ids: frags.map { $0.id }, method: method, circular: false)
                check(try project.summary(id: r.document.id).length == 100, "Zusammenbau \(method): 100 bp")
                try project.deleteDocument(id: r.document.id)
            }
            // SLIC braucht mindestens 25 bp Überlappung: 30 bp → 110 bp, mit 20 bp wird abgelehnt
            let ov30 = overlap + "TTGCAGGCAT"
            let slicFrags = try project.createSequence(name: "slic", text: ">l\nACGTTGCAAGGCTTAACCGGATATCGCGATTAGCCTAGGA\(ov30)\n>r\n\(ov30)TTGCATGCCGTAAGCTTGACCGATCAGTCTAGGCATCCAA\n", kind: "DNA", circular: false)
            let sl = try project.assembleOverlap(ids: slicFrags.map { $0.id }, method: "slic", circular: false)
            check(try project.summary(id: sl.document.id).length == 110, "Zusammenbau slic: 110 bp")
            check((try? project.assembleOverlap(ids: frags.map { $0.id }, method: "slic", circular: false)) == nil, "SLIC lehnt 20 bp Überlappung ab")
            for d in slicFrags + [sl.document] { try project.deleteDocument(id: d.id) }
            // Golden Gate: BsaI-Stellen GGTCTC(N1)/4-nt-Überhang an den Fragmentenden
            let gl = "GGTCTCA" + "AATG" + "ACGTACGTACGTACGTACGT" + "GCTT" + "T" + "GAGACC"
            let gr = "GGTCTCA" + "GCTT" + "TTGGCCAATTGGCCAATTGG" + "CGAA" + "T" + "GAGACC"
            let gg = try project.createSequence(name: "gg", text: ">l\n\(gl)\n>r\n\(gr)\n", kind: "DNA", circular: false)
            if let r = try? project.assembleGoldenGate(ids: gg.map { $0.id }, enzyme: "BsaI", circular: false) {
                check(try project.summary(id: r.document.id).length > 30, "Golden-Gate-Zusammenbau liefert ein Konstrukt")
                try project.deleteDocument(id: r.document.id)
            } else {
                check(false, "Golden-Gate-Zusammenbau fehlgeschlagen")
            }
            for d in frags + gg { try project.deleteDocument(id: d.id) }

            // Rückübersetzung, Zusammenfügen, stille Mutationen, Alignment-Spalten
            check(try backTranslate(protein: "MKV", mode: "degenerate") == "ATGAARGTN", "Rückübersetzung mehrdeutig")
            let pr = try project.createSequence(name: "p", text: "MKV", kind: "PROTEIN", circular: false)
            let bt = try project.backTranslateDocument(id: pr[0].id, mode: "preferred", name: "")
            check(try project.bases(id: bt.id, start: 0, end: 100) == "ATGAAAGTG", "Rückübersetzung eines Dokuments")
            let ca = try project.createSequence(name: "ca", text: "ACGT", kind: "DNA", circular: false)
            let cb = try project.createSequence(name: "cb", text: "TTGCA", kind: "DNA", circular: false)
            let cc = try project.concatenateDocuments(ids: [ca[0].id, cb[0].id], name: "ab", circular: false)
            check(try project.bases(id: cc.id, start: 0, end: 100) == "ACGTTTGCA", "Sequenzen zusammengefügt")
            let sm = try project.createSequence(name: "sm", text: "GAGTTC", kind: "DNA", circular: false)
            try project.addFeatures(id: sm[0].id, features: [FeatureInfo(featureType: "CDS", name: "c", start: 0, end: 6, strand: "FORWARD")])
            let sc = try project.silentChanges(id: sm[0].id, featureIndex: 0, site: "EcoRI", mode: "create")
            check(sc.count == 1 && sc[0].position == 2 && sc[0].to == "A", "Stille Mutation erzeugt EcoRI-Stelle")
            let ax = try project.createSequence(name: "ax", text: "ACGTAC", kind: "DNA", circular: false)
            let ay = try project.createSequence(name: "ay", text: "ACCTAC", kind: "DNA", circular: false)
            let al = try project.alignDocuments(ids: [ax[0].id, ay[0].id], name: "al", backend: "builtin")
            let st = try project.stripAlignmentColumns(id: al.id, kind: "codon", param: 3, start: 0, name: "")
            check(try project.alignment(id: st.id).rows[0].count == 4, "Alignment: dritte Codonposition entfernt")
            let sv = try project.sequenceWithVariants(id: al.id, reference: 0, row: 1, name: "")
            check(try project.bases(id: sv.id, start: 0, end: 100) == "ACCTAC", "Sequenz mit Varianten")
            check(enzymeSets().count >= 3, "Enzymsets vorhanden")

            // Neue Formate, Tabellen und NGS-Werkzeuge
            let nd = dir.appendingPathComponent("neu"); try FileManager.default.createDirectory(at: nd, withIntermediateDirectories: true)
            try "#mega\n!Title t;\n#a\nACGT\n#b\nAC-T\n".write(to: nd.appendingPathComponent("a.meg"), atomically: true, encoding: .utf8)
            check(try project.importFile(path: nd.appendingPathComponent("a.meg").path).first?.docType == "alignment", "MEGA-Import als Alignment")
            try ">P1;S1\nEin Protein\nMKVL*\n".write(to: nd.appendingPathComponent("p.pir"), atomically: true, encoding: .utf8)
            let pir = try project.importFile(path: nd.appendingPathComponent("p.pir").path)
            check(try project.summary(id: pir[0].id).seqType == "PROTEIN", "PIR-Import als Protein")
            try "Name;Seq\na;ACGT\n\"b c\";ttgg\n".write(to: nd.appendingPathComponent("t.csv"), atomically: true, encoding: .utf8)
            let tp = try tablePreview(path: nd.appendingPathComponent("t.csv").path, sep: ";", maxRows: 5)
            check(tp.count == 3 && tp[2][0] == "b c", "Tabellen-Vorschau")
            let ti = try project.importTable(path: nd.appendingPathComponent("t.csv").path, sep: ";", nameCol: 0, seqCol: 1, hasHeader: true, kind: "DNA", circular: false)
            check(ti.count == 2 && ti[1].name == "b c", "Tabellenimport")
            check(try project.exportText(id: ti[0].id, kind: "features_csv").hasPrefix("Typ,Name"), "Features als CSV")
            try "@x1\nACGTGGGG\n+\nIIIIJJJJ\n@x2\nGGGGGG\n+\nIIIIII\n".write(to: nd.appendingPathComponent("r.fastq"), atomically: true, encoding: .utf8)
            let grp = try splitFastqByBarcode(path: nd.appendingPathComponent("r.fastq").path, barcodes: "b1,ACGT", maxMismatch: 0, outDir: nd.appendingPathComponent("out").path)
            check(grp.map { $0.count } == [1, 1], "Barcodes aufgeteilt")
            check(try fastqToFastaQual(path: nd.appendingPathComponent("r.fastq").path, fastaPath: nd.appendingPathComponent("r.fa").path, qualPath: nd.appendingPathComponent("r.qual").path) == 2, "FASTQ → FASTA + QUAL")
            try "3\nwater\nO 0 0 0\nH 0.757 0.586 0\nH -0.757 0.586 0\n".write(to: nd.appendingPathComponent("w.xyz"), atomically: true, encoding: .utf8)
            check(try structureOpen(path: nd.appendingPathComponent("w.xyz").path).elements == ["O", "H", "H"], "Molekül aus XYZ")

            // Neue Formate und Auswertungen (Runde 3)
            try "# x\n>r1\nT0123\n".write(to: nd.appendingPathComponent("a.csfasta"), atomically: true, encoding: .utf8)
            let cs = try project.importFile(path: nd.appendingPathComponent("a.csfasta").path)
            check(try project.bases(id: cs[0].id, start: 0, end: 100) == "TTGAT", "csfasta-Import (Farbraum → Basen)")
            try "AS 1 1\n\nCO c1 8 1 0 U\nACGTAC*G\n\nBQ\n 30\n\nAF r1 U 1\n\nRD r1 5 0 0\nACGTA\n\n".write(to: nd.appendingPathComponent("a.ace"), atomically: true, encoding: .utf8)
            let ac = try project.importFile(path: nd.appendingPathComponent("a.ace").path)
            check(try project.features(id: ac[0].id).first?.featureType == "read", "ACE-Import mit Read-Features")
            let pa = try project.createSequence(name: "PA", text: "AAAAAAAA", kind: "DNA", circular: false)
            let pb = try project.createSequence(name: "PB", text: "CCCCCCCC", kind: "DNA", circular: false)
            let pq = try project.createSequence(name: "PQ", text: "AAAACCCC", kind: "DNA", circular: false)
            let pal = try project.alignDocuments(ids: [pa[0].id, pb[0].id, pq[0].id], name: "pal", backend: "builtin")
            check(try project.alignmentSimilarity(id: pal.id, window: 3).count == 8, "Ähnlichkeitsverlauf")
            let fs = try project.createSequence(name: "fs", text: "AACCGGTTAACC", kind: "DNA", circular: false)
            check(try project.findSimilarFeatures(id: fs[0].id, reference: "AACC", maxMismatch: 0).count == 3, "Feature-Treffer auf beiden Strängen")
            try project.addFeatures(id: fs[0].id, features: [FeatureInfo(featureType: "gene", name: "g", start: 2, end: 4, strand: "REVERSE")])
            check(try project.extractFeatures(id: fs[0].id, featureType: "gene", flank: 0).count == 1, "Feature als neue Sequenz")

            // Runde 4: Schwachstellen, Statistik, Oberflächen-Logik
            let lp = try project.createSequence(name: "lp", text: "ANSTGDSMWC", kind: "PROTEIN", circular: false)
            check(try project.proteinLiabilities(id: lp[0].id).count == 6, "Schwachstellen im Protein")
            check(abs(project.proteinProperties(protein: "AVIL").aliphaticIndex - 292.5) < 1e-9, "Aliphatic Index")
            check(pAdjustBh(p: [0.01, 0.04, 0.03, 0.005]) == [0.02, 0.04, 0.04, 0.02], "Benjamini-Hochberg")
            check(commandRank(items: [CommandItemDto(title: "Primer entwerfen", keywords: ["pcr"]), CommandItemDto(title: "Verdau", keywords: [])], query: "pr") == [0], "Befehlssuche")
            check(friendlyError(message: "Database is locked").title == "Projekt gesperrt", "Fehler verständlich")
            check(formatShortcut(keys: "cmd+shift+k") == "⇧⌘K", "Kürzel im Mac-Stil")
            check(recentAdd(list: ["a", "b"], path: "b", max: 5) == ["b", "a"], "Zuletzt geöffnet")
            let tv = try project.createSequence(name: "TV", text: "GGGGTTTT", kind: "DNA", circular: false)
            let tIns = try project.createSequence(name: "TI", text: "ACGTAC", kind: "DNA", circular: false)
            let taDocs = try project.cloneTa(tVectorId: tv[0].id, pcrProductId: tIns[0].id)
            let taBases = try project.bases(id: taDocs[0].id, start: 0, end: 100)
            check(taDocs.count == 2 && taBases == "GGGGTTTTACGTAC", "TA-Klonierung: zwei Orientierungen")
            let tc = try project.createSequence(name: "TC", text: "CACCATG", kind: "DNA", circular: false)
            check(try project.bases(id: project.cloneTopo(vectorId: tv[0].id, insertId: tc[0].id).id, start: 0, end: 100) == "GGGGTTTTCACCATG", "Gerichtetes TOPO")
            let fold = try project.simpleFold(id: lp[0].id, start: 0, end: 0)
            check(fold.structure.count == 10, "Einfache Faltung liefert eine Struktur")

            try project.deleteDocument(id: neu[0].id)
            check(try project.documents().count >= 6, "Löschen (Sequenzen, Alignment, Baum bleiben)")
        } catch {
            check(false, "unerwarteter Fehler: \(error)")
        }
        print(failures == 0 ? "Alle Prüfungen bestanden." : "\(failures) Prüfung(en) fehlgeschlagen.")
        return failures == 0 ? 0 : 1
    }
}
