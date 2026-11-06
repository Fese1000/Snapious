//! SVG-Ausgabe der Plasmidkarte (Kreis) und der linearen Karte.

use std::f64::consts::PI;

use crate::model::document::SequenceDocument;
use crate::view::feature_layout::assign_lanes;

/// Farbe nach Featuretyp (Groß-/Kleinschreibung egal): `cds` und `gene` `#4b5bd6`, `promoter` `#1a9e8f`, `terminator` `#8b5a2b`,
/// `rep_origin` `#8e44ad`, `primer_bind` `#e0559a`, alles andere `#888888`.
pub fn feature_color(feature_type: &str) -> &'static str {
    match feature_type.to_lowercase().as_str() {
        "cds" | "gene" => "#4b5bd6",
        "promoter" => "#1a9e8f",
        "terminator" => "#8b5a2b",
        "rep_origin" => "#8e44ad",
        "primer_bind" => "#e0559a",
        _ => "#888888",
    }
}

/// Kreiskarte als SVG-Text, quadratisch `size × size`.
///
/// - Kopf `<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">` (Größe als ganze Zahl),
///   Weißer Hintergrund `<rect width="100%" height="100%" fill="white"/>\n`; jede Zeile endet mit `\n`; Schluss `</svg>\n`.
/// - Mittelpunkt `cx = cy = size / 2`, Rückgrat-Radius `R = size · 0.32`: `<circle class="backbone" cx=".." cy=".." r=".." fill="none" stroke="#555555" stroke-width="2"/>`.
/// - Position `p` hat den Winkel `2π · p / Länge` (oben beginnend, im Uhrzeigersinn): Punkt `(cx + r·sin a, cy − r·cos a)`.
/// - Features mit `assign_lanes`: Spur `k` liegt auf dem Radius `R − 14 − 14·k`. Jedes Segment `[s, e)` ist ein
///   `<path class="feature" d="M x0,y0 A r,r 0 {groß} 1 x1,y1" fill="none" stroke="{farbe}" stroke-width="10"/>`
///   mit `groß = 1`, wenn der überstrichene Winkel größer als π ist, sonst 0. Deckt das Segment die ganze Sequenz ab (`s == 0` und `e == Länge`),
///   stattdessen `<circle class="feature" cx cy r fill="none" stroke="{farbe}" stroke-width="10"/>`.
/// - Pro Feature (nicht pro Segment) ein Text `<text class="feature-label" x y text-anchor="middle" font-size="10">{Name}</text>` am Radius der
///   Spur beim Winkel der Mitte seines ersten Segments (Name mit `xml_escape`); Features ohne Namen bekommen kein Label.
/// - Enzymstellen `sites` (`(Name, Position)`, nach Position sortiert gegeben): je eine `<line class="site" x1 y1 x2 y2 stroke="#000000"/>`
///   von Radius `R` bis `R + 10` und ein `<text class="site-label" x y font-size="10">{Name} ({Position + 1})</text>` bei Radius `R + 20`.
/// - Mitte: `<text class="title" x="{cx}" y="{cy − 6}" text-anchor="middle" font-size="16">{Name des Dokuments}</text>` und
///   `<text class="length" x="{cx}" y="{cy + 14}" text-anchor="middle" font-size="12">{Länge} bp</text>`.
/// - Alle Zahlen mit `format!("{:.2}", …)`.
pub fn circular_map_svg(doc: &SequenceDocument, sites: &[(String, usize)], size: f64) -> String {
    let w = size as i64;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        w, w, w, w
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n");

    let len = doc.len();
    let cx = size / 2.0;
    let cy = size / 2.0;
    let r_backbone = size * 0.32;

    svg.push_str(&format!(
        "<circle class=\"backbone\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"none\" stroke=\"#555555\" stroke-width=\"2\"/>\n",
        cx, cy, r_backbone
    ));

    if len == 0 {
        svg.push_str(&format!(
            "<text class=\"title\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"16\">{}</text>\n",
            cx, cy - 6.0, doc.name()
        ));
        svg.push_str(&format!(
            "<text class=\"length\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"12\">{} bp</text>\n",
            cx, cy + 14.0, len
        ));
        svg.push_str("</svg>\n");
        return svg;
    }

    let placed = assign_lanes(doc.features());
    let mut prev_idx: Option<usize> = None;

    for pf in placed.iter() {
        let r_lane = r_backbone - 14.0 - (pf.lane as f64) * 14.0;
        let color = feature_color(doc.features()[pf.feature_index].feature_type());

        for interval in doc.features()[pf.feature_index].location().intervals() {
            let s = interval.start();
            let e = interval.end();

            if s == 0 && e == len {
                svg.push_str(&format!(
                    "<circle class=\"feature\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"10\"/>\n",
                    cx, cy, r_lane, color
                ));
            } else {
                let a_start = 2.0 * PI * s as f64 / len as f64;
                let a_end = 2.0 * PI * e as f64 / len as f64;
                let x0 = cx + r_lane * a_start.sin();
                let y0 = cy - r_lane * a_start.cos();
                let x1 = cx + r_lane * a_end.sin();
                let y1 = cy - r_lane * a_end.cos();

                let diff = if a_end >= a_start {
                    a_end - a_start
                } else {
                    2.0 * PI - a_start + a_end
                };
                let large = if diff > PI { "1" } else { "0" };

                svg.push_str(&format!(
                    "<path class=\"feature\" d=\"M {:.2},{:.2} A {:.2},{:.2} 0 {} 1 {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"10\"/>\n",
                    x0, y0, r_lane, r_lane, large, x1, y1, color
                ));
            }

            if prev_idx != Some(pf.feature_index)
                && !doc.features()[pf.feature_index].name().is_empty()
            {
                let first_interval = doc.features()[pf.feature_index].location().intervals()[0];
                let mid_pos = (first_interval.start() + first_interval.end()) / 2;
                let a_mid = 2.0 * PI * mid_pos as f64 / len as f64;
                let lx = cx + r_lane * a_mid.sin();
                let ly = cy - r_lane * a_mid.cos();
                svg.push_str(&format!(
                    "<text class=\"feature-label\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"10\">{}</text>\n",
                    lx, ly, xml_escape(doc.features()[pf.feature_index].name())
                ));
            }
        }
        prev_idx = Some(pf.feature_index);
    }

    for (name, pos) in sites.iter() {
        let a = 2.0 * PI * *pos as f64 / len as f64;
        let x1 = cx + r_backbone * a.sin();
        let y1 = cy - r_backbone * a.cos();
        let x2 = cx + (r_backbone + 10.0) * a.sin();
        let y2 = cy - (r_backbone + 10.0) * a.cos();
        svg.push_str(&format!(
            "<line class=\"site\" x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#000000\"/>\n",
            x1, y1, x2, y2
        ));

        let lx = cx + (r_backbone + 20.0) * a.sin();
        let ly = cy - (r_backbone + 20.0) * a.cos();
        svg.push_str(&format!(
            "<text class=\"site-label\" x=\"{:.2}\" y=\"{:.2}\" font-size=\"10\">{} ({})</text>\n",
            lx,
            ly,
            name,
            pos + 1
        ));
    }

    svg.push_str(&format!(
        "<text class=\"title\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"16\">{}</text>\n",
        cx, cy - 6.0, doc.name()
    ));
    svg.push_str(&format!(
        "<text class=\"length\" x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\" font-size=\"12\">{} bp</text>\n",
        cx, cy + 14.0, len
    ));

    svg.push_str("</svg>\n");
    svg
}

/// Lineare Karte als SVG-Text `width × height`.
///
/// - Kopf wie oben mit `width`/`height`; Hintergrund wie oben.
/// - Achse bei `y = 60` von `x = 20` bis `x = width − 20`: `<line class="axis" x1 y1 x2 y2 stroke="#555555" stroke-width="2"/>`.
///   Position `p` liegt bei `x = 20 + p / Länge · (width − 40)`.
/// - Features mit `assign_lanes`: Spur `k` liegt bei `y = 80 + 18·k` (Höhe 14): je Segment ein
///   `<rect class="feature" x y width height fill="{farbe}"/>` (Breite mindestens 1) und je Feature ein
///   `<text class="feature-label" x y font-size="10">{Name}</text>` bei `x` des ersten Segments, `y` = Oberkante der Spur + 11 (nur bei Name).
/// - Enzymstellen: `<line class="site" x1 y1 x2 y2 stroke="#000000"/>` von `y = 60` bis `y = 50` und `<text class="site-label" x y font-size="10">{Name}</text>`
///   bei `y = 46`.
/// - Titel `<text class="title" x="20" y="20" font-size="14">{Name} ({Länge} bp)</text>`.
/// - Alle Zahlen mit `format!("{:.2}", …)`.
pub fn linear_map_svg(
    doc: &SequenceDocument,
    sites: &[(String, usize)],
    width: f64,
    height: f64,
) -> String {
    let w = width as i64;
    let h = height as i64;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        w, h, w, h
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n");

    let len = doc.len();
    let axis_x2 = width - 20.0;

    svg.push_str(&format!(
        "<line class=\"axis\" x1=\"20.00\" y1=\"60.00\" x2=\"{:.2}\" y2=\"60.00\" stroke=\"#555555\" stroke-width=\"2\"/>\n",
        axis_x2
    ));

    if len == 0 {
        svg.push_str("</svg>\n");
        return svg;
    }

    let placed = assign_lanes(doc.features());
    let mut prev_idx: Option<usize> = None;

    for pf in placed.iter() {
        let y_lane = 80.0 + (pf.lane as f64) * 18.0;
        let color = feature_color(doc.features()[pf.feature_index].feature_type());

        for interval in doc.features()[pf.feature_index].location().intervals() {
            let s = interval.start();
            let e = interval.end();

            let x = 20.0 + s as f64 / len as f64 * (width - 40.0);
            let w_feat = ((e - s) as f64 / len as f64 * (width - 40.0)).max(1.0);

            svg.push_str(&format!(
                "<rect class=\"feature\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"14.00\" fill=\"{}\"/>\n",
                x, y_lane, w_feat, color
            ));

            if prev_idx != Some(pf.feature_index)
                && !doc.features()[pf.feature_index].name().is_empty()
            {
                svg.push_str(&format!(
                    "<text class=\"feature-label\" x=\"{:.2}\" y=\"{:.2}\" font-size=\"10\">{}</text>\n",
                    x, y_lane + 11.0, xml_escape(doc.features()[pf.feature_index].name())
                ));
            }
        }
        prev_idx = Some(pf.feature_index);
    }

    for (name, pos) in sites.iter() {
        let x = 20.0 + *pos as f64 / len as f64 * (width - 40.0);
        svg.push_str(&format!(
            "<line class=\"site\" x1=\"{:.2}\" y1=\"60.00\" x2=\"{:.2}\" y2=\"50.00\" stroke=\"#000000\"/>\n",
            x, x
        ));
        svg.push_str(&format!(
            "<text class=\"site-label\" x=\"{:.2}\" y=\"46.00\" font-size=\"10\">{}</text>\n",
            x, name
        ));
    }

    svg.push_str(&format!(
        "<text class=\"title\" x=\"20.00\" y=\"20.00\" font-size=\"14\">{} ({} bp)</text>\n",
        doc.name(),
        len
    ));

    svg.push_str("</svg>\n");
    svg
}

/// Ersetzt `&`, `<`, `>`, `"` und `'` durch `&amp;`, `&lt;`, `&gt;`, `&quot;` und `&apos;`.
pub fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::Topology;
    use crate::model::feature::{Feature, Interval, Location, Strand};
    use crate::model::sequence::{Sequence, SequenceType};

    fn doc(len: usize, features: Vec<Feature>) -> SequenceDocument {
        let seq = Sequence::new("s", "pTest", &"A".repeat(len), SequenceType::Dna).unwrap();
        SequenceDocument::of("pTest", seq)
            .with_topology(Topology::Circular)
            .with_features(features)
    }

    fn feat(kind: &str, name: &str, start: usize, end: usize) -> Feature {
        let loc = Location::new(
            vec![Interval::new(start, end).unwrap()],
            Strand::Forward,
            false,
            false,
        )
        .unwrap();
        Feature::new(kind, name, loc, Vec::new())
    }

    #[test]
    fn farben() {
        assert_eq!(feature_color("CDS"), "#4b5bd6");
        assert_eq!(feature_color("gene"), "#4b5bd6");
        assert_eq!(feature_color("Promoter"), "#1a9e8f");
        assert_eq!(feature_color("terminator"), "#8b5a2b");
        assert_eq!(feature_color("rep_origin"), "#8e44ad");
        assert_eq!(feature_color("primer_bind"), "#e0559a");
        assert_eq!(feature_color("irgendwas"), "#888888");
    }

    #[test]
    fn kreis_aufbau() {
        let d = doc(
            1000,
            vec![
                feat("CDS", "gfp", 0, 250),
                feat("promoter", "P&1", 500, 900),
            ],
        );
        let s = circular_map_svg(&d, &[("EcoRI".to_string(), 100)], 400.0);
        assert!(s.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400\" height=\"400\" viewBox=\"0 0 400 400\">\n"));
        assert!(s.ends_with("</svg>\n"));
        assert_eq!(s.matches("class=\"backbone\"").count(), 1);
        assert_eq!(s.matches("<path class=\"feature\"").count(), 2);
        assert_eq!(s.matches("class=\"feature-label\"").count(), 2);
        assert!(s.contains(">gfp</text>"));
        assert!(s.contains(">P&amp;1</text>"));
        assert_eq!(s.matches("<line class=\"site\"").count(), 1);
        assert!(s.contains(">EcoRI (101)</text>"));
        assert!(s.contains(">pTest</text>"));
        assert!(s.contains(">1000 bp</text>"));
        // Erstes Feature beginnt oben: Radius 128 − 14 = 114 bei Mittelpunkt 200 → y = 86
        assert!(s.contains("d=\"M 200.00,86.00 A 114.00,114.00 0 0 1 314.00,200.00\""));
        assert!(s.contains("stroke=\"#4b5bd6\"") && s.contains("stroke=\"#1a9e8f\""));
    }

    #[test]
    fn grosser_bogen_und_volles_feature() {
        let d = doc(
            1000,
            vec![
                feat("gene", "gross", 0, 600),
                feat("rep_origin", "ganz", 0, 1000),
            ],
        );
        let s = circular_map_svg(&d, &[], 400.0);
        assert!(s.contains(" 0 1 1 "));
        assert_eq!(s.matches("<circle class=\"feature\"").count(), 1);
        assert_eq!(s.matches("<path class=\"feature\"").count(), 1);
        assert_eq!(s.matches("<line class=\"site\"").count(), 0);
    }

    #[test]
    fn spuren_liegen_auf_verschiedenen_radien() {
        let d = doc(
            1000,
            vec![feat("gene", "a", 0, 500), feat("gene", "b", 100, 400)],
        );
        let s = circular_map_svg(&d, &[], 400.0);
        // Spur 0 Radius 114, Spur 1 Radius 100
        assert!(s.contains("114.00,114.00") && s.contains("100.00,100.00"));
    }

    #[test]
    fn lineare_karte() {
        let d = doc(1000, vec![feat("CDS", "gfp", 100, 300)]).with_topology(Topology::Linear);
        let s = linear_map_svg(&d, &[("BamHI".to_string(), 500)], 840.0, 200.0);
        assert!(s.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"840\" height=\"200\" viewBox=\"0 0 840 200\">\n"));
        assert!(s.ends_with("</svg>\n"));
        assert_eq!(s.matches("class=\"axis\"").count(), 1);
        // x = 20 + 100/1000 · 800 = 100
        assert!(s.contains("<rect class=\"feature\" x=\"100.00\" y=\"80.00\" width=\"160.00\" height=\"14.00\" fill=\"#4b5bd6\"/>"));
        assert!(s.contains(">gfp</text>"));
        assert!(s.contains(">BamHI</text>"));
        assert!(s.contains(">pTest (1000 bp)</text>"));
        assert_eq!(s.matches("<line class=\"site\"").count(), 1);
        // BamHI bei x = 20 + 0.5 · 800 = 420
        assert!(s.contains("x1=\"420.00\""));
    }

    #[test]
    fn leeres_dokument() {
        let d = doc(0, vec![]);
        let s = circular_map_svg(&d, &[], 300.0);
        assert!(s.contains("0 bp"));
        let l = linear_map_svg(&d, &[], 300.0, 100.0);
        assert!(l.ends_with("</svg>\n"));
    }
}
