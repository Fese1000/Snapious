import SnapiousFFI
import SwiftUI

/// Werkzeuge für die gewählte Sequenz: Mutagenese-Primer, Wiederholungen und Laborrechner.
struct ToolsView: View {
    @ObservedObject var model: AppModel
    @State private var start = ""
    @State private var end = ""
    @State private var replacement = ""
    @State private var targetTm = 60.0
    @State private var primers: MutagenesisDto?
    @State private var repeats: [RepeatDto] = []
    @State private var message: String?
    @State private var ng = "100"
    @State private var bp = "3000"
    @State private var insertBp = "1000"
    @State private var ratio = "3"
    @State private var stock = "10"
    @State private var final = "1"
    @State private var volume = "50"
    @State private var blastProgram = "blastn"
    @State private var blastDb = "nt"
    @State private var blastRunning = false
    @State private var blastHits: [BlastHitDto] = []

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if let message { Text(message).foregroundStyle(Theme.danger) }
                if model.summary?.seqType == "PROTEIN" {
                    BackTranslateSection(model: model)
                } else {
                    SilentMutationsSection(model: model)
                }
                GroupBox(tr("Gezielte Mutagenese (Positionen 1-basiert, Ende einschließlich)")) {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            TextField("von", text: $start).frame(width: 70)
                            TextField("bis", text: $end).frame(width: 70)
                            TextField(tr("neue Basen (leer = Deletion)"), text: $replacement).frame(width: 220)
                            Stepper(tr("Ziel-Tm %@ °C", "\(Int(targetTm))"), value: $targetTm, in: 50...75).frame(width: 170)
                            Button(tr("Primer berechnen")) { design() }
                        }
                        .textFieldStyle(.roundedBorder)
                        if let p = primers {
                            Text(tr("Vorwärts: %@", "\(p.forward)")).font(Theme.mono(13)).textSelection(.enabled)
                            Text(tr("Rückwärts: %@", "\(p.reverse)")).font(Theme.mono(13)).textSelection(.enabled)
                            Text(tr("Flanken %@/%@ · Tm %@ °C", "\(p.leftFlank)", "\(p.rightFlank)", "\(String(format: "%.1f", p.tm))")).foregroundStyle(Theme.inkSoft)
                        }
                    }
                    .padding(6)
                }
                ExternalResultsSection(model: model)
                StatsSection()
                GroupBox(tr("BLAST bei NCBI (Sequenz wird an NCBI gesendet, dauert Minuten)")) {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Picker(tr("Programm"), selection: $blastProgram) {
                                Text("blastn").tag("blastn")
                                Text("blastp").tag("blastp")
                            }.frame(width: 170)
                            Picker(tr("Datenbank"), selection: $blastDb) {
                                Text("nt").tag("nt")
                                Text("nr").tag("nr")
                                Text("refseq_rna").tag("refseq_rna")
                            }.frame(width: 190)
                            Button(blastRunning ? tr("läuft …") : tr("Suchen")) { remoteBlast() }
                                .disabled(blastRunning)
                        }
                        ForEach(Array(blastHits.prefix(15).enumerated()), id: \.offset) { _, h in
                            Text("\(h.subject) · \(String(format: "%.1f", h.identityPercent)) % · \(h.alignmentLength) bp · E \(String(format: "%.1e", h.evalue))")
                                .font(Theme.label(13))
                        }
                    }
                    .padding(6)
                }
                GroupBox(tr("Wiederholungen")) {
                    VStack(alignment: .leading, spacing: 8) {
                        Button(tr("Tandem- und invertierte Wiederholungen suchen")) { findRepeats() }
                        ForEach(Array(repeats.enumerated()), id: \.offset) { _, r in
                            Text("\(r.kind == "tandem" ? "Tandem" : "Invertiert") \(r.start + 1)–\(r.end): \(r.detail)")
                                .font(Theme.label(13))
                        }
                    }
                    .padding(6)
                }
                GroupBox(tr("Laborrechner (dsDNA)")) {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            field("ng", $ng); field("bp", $bp)
                            Text(calc { try "= " + fmt(calcNgToPmol(ng: num(ng), bp: UInt64(num(bp)))) + tr(" pmol, ")
                                + fmt(calcCopyNumber(ng: num(ng), bp: UInt64(num(bp)))) + tr(" Kopien") })
                        }
                        HStack {
                            field(tr("Insert bp"), $insertBp); field(tr("Verhältnis"), $ratio)
                            Text(calc { try tr("Insert: ") + fmt(calcLigationInsertNg(vectorNg: num(ng), vectorBp: UInt64(num(bp)),
                                insertBp: UInt64(num(insertBp)), molarRatio: num(ratio))) + tr(" ng (Vektor wie oben)") })
                        }
                        HStack {
                            field(tr("Stamm"), $stock); field(tr("Ziel"), $final); field(tr("Volumen µl"), $volume)
                            Text(calc { try tr("Stammlösung: ") + fmt(calcDilutionVolume(cStock: num(stock), cFinal: num(final), vFinal: num(volume))) + " µl" })
                        }
                    }
                    .padding(6)
                }
            }
            .padding(12)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onChange(of: model.selection) { _, _ in
            primers = nil
            repeats = []
            message = nil
        }
    }

    private func field(_ title: String, _ text: Binding<String>) -> some View {
        TextField(title, text: text).textFieldStyle(.roundedBorder).frame(width: 90)
    }

    private func num(_ s: String) -> Double { Double(s.replacingOccurrences(of: ",", with: ".")) ?? 0 }

    private func fmt(_ v: Double) -> String {
        v >= 1e6 ? String(format: "%.3g", v) : String(format: "%.3f", v)
    }

    private func calc(_ f: () throws -> String) -> String {
        (try? f()) ?? "–"
    }

    private func design() {
        guard let project = model.project, let id = model.selection,
              let s = Int(start), let e = Int(end), s >= 1, e >= s - 1 else {
            message = tr("Bitte gültige Positionen eingeben.")
            return
        }
        do {
            primers = try project.mutagenesisPrimers(id: id, start: UInt64(s - 1), end: UInt64(e), replacement: replacement,
                                                    minFlank: 10, maxFlank: 30, targetTm: targetTm)
            message = nil
        } catch {
            primers = nil
            message = error.localizedDescription
        }
    }

    private func remoteBlast() {
        guard let project = model.project, let id = model.selection else { return }
        blastRunning = true
        message = nil
        let program = blastProgram, db = blastDb
        Task.detached {
            do {
                let hits = try project.blastRemote(queryId: id, program: program, database: db)
                await MainActor.run { blastHits = hits; blastRunning = false; if hits.isEmpty { message = tr("Keine Treffer.") } }
            } catch {
                await MainActor.run { blastRunning = false; message = error.localizedDescription }
            }
        }
    }

    private func findRepeats() {
        guard let project = model.project, let id = model.selection else { return }
        do {
            repeats = try project.findRepeats(id: id, minUnit: 1, maxUnit: 6, minCopies: 4, minArm: 8, maxSpacer: 12)
            message = nil
        } catch {
            message = error.localizedDescription
        }
    }
}
