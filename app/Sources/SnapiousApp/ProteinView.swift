import SnapiousFFI
import SwiftUI

/// Protein of a feature (usually CDS): amino acid sequence, properties and peptides of a protease digest.
struct ProteinView: View {
    @ObservedObject var model: AppModel
    @State private var featureIndex: Int = -1
    @State private var protease = "trypsin"
    @State private var missed = 0
    @State private var protein = ""
    @State private var props: ProteinPropsDto?
    @State private var peptides: [PeptideRow] = []
    @State private var message: String?
    @State private var liabilities: [LiabilityDto] = []

    private var candidates: [(Int, FeatureInfo)] {
        let all = Array(model.features.enumerated()).map { ($0.offset, $0.element) }
        let cds = all.filter { $0.1.featureType.uppercased() == "CDS" }
        return cds.isEmpty ? all : cds
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Picker(tr("Feature"), selection: $featureIndex) {
                    Text(tr("Please choose")).tag(-1)
                    ForEach(candidates, id: \.0) { Text("\($0.1.name) (\($0.1.featureType), \($0.1.start + 1)–\($0.1.end))").tag($0.0) }
                }
                .frame(width: 380)
                .onChange(of: featureIndex) { _, _ in load() }
                Spacer()
            }
            if let message { Text(message).foregroundStyle(Theme.danger) }
            if !protein.isEmpty {
                GroupBox(tr("Amino acid sequence")) {
                    ScrollView {
                        Text(protein).font(Theme.mono(13)).textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .frame(height: 90)
                }
                if let p = props {
                    Grid(alignment: .leading, horizontalSpacing: 24, verticalSpacing: 4) {
                        GridRow { Text(tr("Length")).foregroundStyle(Theme.inkSoft); Text("\(p.length) aa") }
                        GridRow { Text(tr("Mass")).foregroundStyle(Theme.inkSoft); Text(String(format: "%.2f Da", p.molecularWeight)) }
                        GridRow { Text("pI").foregroundStyle(Theme.inkSoft); Text(String(format: "%.2f", p.isoelectricPoint)) }
                        GridRow { Text(tr("Charge at pH 7")).foregroundStyle(Theme.inkSoft); Text(String(format: "%.2f", p.chargePh7)) }
                        GridRow { Text("ε280").foregroundStyle(Theme.inkSoft); Text(String(format: "%.0f M⁻¹cm⁻¹", p.extinction280)) }
                        GridRow { Text(tr("Aromaticity")).foregroundStyle(Theme.inkSoft); Text(String(format: "%.1f %%", p.aromaticity * 100)) }
                        GridRow { Text("Aliphatic Index").foregroundStyle(Theme.inkSoft); Text(String(format: "%.1f", p.aliphaticIndex)) }
                        GridRow { Text(tr("GRAVY (hydrophobicity)")).foregroundStyle(Theme.inkSoft); Text(String(format: "%.2f", p.gravy)) }
                    }
                }
                if !liabilities.isEmpty {
                    GroupBox(tr("Liabilities (%@)", "\(liabilities.count)")) {
                        VStack(alignment: .leading, spacing: 2) {
                            ForEach(Array(liabilities.prefix(40).enumerated()), id: \.offset) { _, l in
                                Text("\(l.position + 1): \(l.motif) – \(l.kind)").font(.callout)
                            }
                            if liabilities.count > 40 { Text(tr("… and %@ more", "\(liabilities.count - 40)")).font(.caption).foregroundStyle(.secondary) }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
                HStack {
                    Picker(tr("Protease"), selection: $protease) {
                        Text("Trypsin").tag("trypsin")
                        Text("Lys-C").tag("lysc")
                        Text("Arg-C").tag("argc")
                        Text("Glu-C").tag("gluc")
                        Text("Chymotrypsin").tag("chymotrypsin")
                        Text(tr("Cyanogen bromide")).tag("cnbr")
                    }
                    .frame(width: 220)
                    .onChange(of: protease) { _, _ in digest() }
                    Stepper(tr("Missed cleavages: %@", "\(missed)"), value: $missed, in: 0...3)
                        .onChange(of: missed) { _, _ in digest() }
                }
                Table(peptides) {
                    TableColumn(tr("Range")) { r in Text("\(r.dto.start + 1)–\(r.dto.end)") }.width(90)
                    TableColumn(tr("Peptide")) { r in Text(r.dto.sequence).font(Theme.mono(13)) }
                    TableColumn(tr("Mass")) { r in Text(String(format: "%.2f", r.dto.mass)) }.width(90)
                    TableColumn(tr("Missed")) { r in Text("\(r.dto.missedCleavages)") }.width(60)
                }
            } else {
                Spacer()
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear { loadWholeProtein() }
        .onChange(of: model.selection) { _, _ in
            featureIndex = -1
            protein = ""
            props = nil
            peptides = []
            liabilities = []
            message = nil
            loadWholeProtein()
        }
    }

    /// For a protein document the whole sequence counts as the protein.
    private func loadWholeProtein() {
        guard let project = model.project, model.summary?.seqType == "PROTEIN" else { return }
        protein = model.fullBases
        props = project.proteinProperties(protein: protein)
        liabilities = proteinLiabilitiesText(protein: protein)
        digest()
    }

    private func load() {
        message = nil
        guard let project = model.project, let id = model.selection, featureIndex >= 0 else { return }
        do {
            protein = try project.proteinOfFeature(id: id, featureIndex: UInt64(featureIndex))
            props = project.proteinProperties(protein: protein)
            liabilities = proteinLiabilitiesText(protein: protein)
            digest()
        } catch {
            protein = ""
            props = nil
            peptides = []
            message = error.localizedDescription
        }
    }

    private func digest() {
        guard let project = model.project, !protein.isEmpty else { return }
        peptides = ((try? project.proteinDigest(protein: protein, protease: protease, missedCleavages: UInt64(missed))) ?? [])
            .enumerated().map { PeptideRow(index: $0.offset, dto: $0.element) }
    }
}

struct PeptideRow: Identifiable {
    let index: Int
    let dto: PeptideDto
    var id: Int { index }
}
