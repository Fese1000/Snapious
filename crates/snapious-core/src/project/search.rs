//! Suche in den Dokumenten eines Projekts, Stapelumbenennung und Dublettensuche.

use std::collections::BTreeMap;

use crate::bio::motif::find_motif;
use crate::error::Result;
use crate::model::document::{Document, Topology};
use crate::store::project_store::{DocumentInfo, ProjectStore};

/// Dokumente, deren Name `text` enthält (Groß-/Kleinschreibung egal). Leerer Text liefert alle. Reihenfolge wie `ProjectStore::list`.
pub fn by_name(store: &ProjectStore, text: &str) -> Result<Vec<DocumentInfo>> {
    let q = text.to_lowercase();
    Ok(store
        .list()?
        .into_iter()
        .filter(|d| d.name.to_lowercase().contains(&q))
        .collect())
}

/// Dokumente eines Typs (`sequence`, `alignment`, `tree`; genau gleich, Groß-/Kleinschreibung egal).
pub fn by_type(store: &ProjectStore, doc_type: &str) -> Result<Vec<DocumentInfo>> {
    let t = doc_type.to_lowercase();
    Ok(store
        .list()?
        .into_iter()
        .filter(|d| d.doc_type.to_lowercase() == t)
        .collect())
}

/// Sequenzdokumente, bei denen mindestens ein Feature in Typ, Name, einem Qualifier-Schlüssel oder einem Qualifier-Wert `text` enthält
/// (Groß-/Kleinschreibung egal). Andere Dokumenttypen werden übersprungen. Leerer Text → keine Treffer.
pub fn by_feature_text(store: &ProjectStore, text: &str) -> Result<Vec<DocumentInfo>> {
    let q = text.to_lowercase();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for info in store.list()? {
        let Some(Document::Sequence(d)) = store.load(&info.id)? else {
            continue;
        };
        let hit = d.features().iter().any(|f| {
            f.feature_type().to_lowercase().contains(&q)
                || f.name().to_lowercase().contains(&q)
                || f.qualifiers().iter().any(|(k, vals)| {
                    k.to_lowercase().contains(&q)
                        || vals.iter().any(|v| v.to_lowercase().contains(&q))
                })
        });
        if hit {
            out.push(info);
        }
    }
    Ok(out)
}

/// Dokumente, deren Metadaten (Sequenz, Alignment oder Baum) den Schlüssel `key` haben (genau gleich) und deren Wert `value` entspricht
/// (Groß-/Kleinschreibung egal). Ein leerer `value` verlangt nur, dass der Schlüssel vorhanden ist.
pub fn by_metadata(store: &ProjectStore, key: &str, value: &str) -> Result<Vec<DocumentInfo>> {
    let want = value.to_lowercase();
    let mut out = Vec::new();
    for info in store.list()? {
        let Some(doc) = store.load(&info.id)? else {
            continue;
        };
        let meta = match &doc {
            Document::Sequence(d) => d.metadata(),
            Document::Alignment(d) => d.metadata(),
        };
        if let Some(v) = meta.get(key) {
            if want.is_empty() || v.to_lowercase() == want {
                out.push(info);
            }
        }
    }
    Ok(out)
}

/// Sequenzdokumente, die das IUPAC-Motiv auf einem der beiden Stränge enthalten (`find_motif(bases, motif, true, circular)`, zirkulär bei
/// `Topology::Circular`). Ein ungültiges Motiv ergibt den Fehler der Kernfunktion.
pub fn by_sequence_motif(store: &ProjectStore, motif: &str) -> Result<Vec<DocumentInfo>> {
    let mut out = Vec::new();
    for info in store.list()? {
        let Some(Document::Sequence(d)) = store.load(&info.id)? else {
            continue;
        };
        let circular = d.topology() == Topology::Circular;
        if !find_motif(d.bases(), motif, true, circular)?.is_empty() {
            out.push(info);
        }
    }
    Ok(out)
}

/// Plan für die Stapelumbenennung: ID → neuer Name. Das Muster ersetzt `{name}` (bisheriger Name), `{type}` (Dokumenttyp), `{n}` (Zähler ab 1 in der
/// Reihenfolge von `docs`) und `{n:W}` mit einer Zahl `W` (Zähler mit führenden Nullen auf mindestens `W` Stellen, z. B. `{n:03}` → `007`).
/// Andere Texte bleiben. Ein nicht erkennbarer Ausdruck in geschweiften Klammern bleibt unverändert.
pub fn batch_rename_plan(docs: &[DocumentInfo], pattern: &str) -> BTreeMap<String, String> {
    let mut plan = BTreeMap::new();
    for (i, d) in docs.iter().enumerate() {
        let mut out = String::new();
        let mut rest = pattern;
        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            let after = &rest[start..];
            let Some(end) = after.find('}') else {
                out.push_str(after);
                rest = "";
                break;
            };
            let token = &after[1..end];
            let replaced = match token {
                "name" => Some(d.name.clone()),
                "type" => Some(d.doc_type.clone()),
                "n" => Some((i + 1).to_string()),
                t if t.starts_with("n:") => t[2..]
                    .parse::<usize>()
                    .ok()
                    .map(|w| format!("{:0width$}", i + 1, width = w)),
                _ => None,
            };
            match replaced {
                Some(r) => out.push_str(&r),
                None => out.push_str(&after[..=end]),
            }
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        plan.insert(d.id.clone(), out);
    }
    plan
}

/// Gruppen von Sequenzdokumenten mit gleichem Typ der Sequenz (DNA/RNA/Protein) und gleichen Basen (Großbuchstaben); nur Gruppen mit mindestens zwei Dokumenten.
/// Innerhalb einer Gruppe Reihenfolge wie `list`, die Gruppen nach ihrem ersten Dokument geordnet.
pub fn find_sequence_duplicates(store: &ProjectStore) -> Result<Vec<Vec<DocumentInfo>>> {
    let mut groups: Vec<((String, String), Vec<DocumentInfo>)> = Vec::new();
    for info in store.list()? {
        let Some(Document::Sequence(d)) = store.load(&info.id)? else {
            continue;
        };
        let key = (
            format!("{:?}", d.sequence().seq_type()),
            d.bases().to_ascii_uppercase(),
        );
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(info),
            None => groups.push((key, vec![info])),
        }
    }
    Ok(groups
        .into_iter()
        .map(|(_, v)| v)
        .filter(|v| v.len() >= 2)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::SequenceDocument;
    use crate::model::feature::{Feature, Location, Strand};
    use crate::model::sequence::{Sequence, SequenceType};

    fn add(
        s: &mut ProjectStore,
        name: &str,
        bases: &str,
        topology: Topology,
        features: Vec<Feature>,
        meta: &[(&str, &str)],
    ) -> String {
        let seq = Sequence::new(name, name, bases, SequenceType::Dna).unwrap();
        let d = SequenceDocument::new(
            &crate::util::new_id(),
            name,
            seq,
            topology,
            features,
            meta.iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            crate::util::now_secs(),
        );
        let id = d.id().to_string();
        s.save(&Document::Sequence(d)).unwrap();
        id
    }

    fn feat(kind: &str, name: &str, q: &[(&str, &str)]) -> Feature {
        let loc = Location::of(0, 3, Strand::Forward).unwrap();
        Feature::new(
            kind,
            name,
            loc,
            q.iter()
                .map(|(k, v)| (k.to_string(), vec![v.to_string()]))
                .collect(),
        )
    }

    /// Namen sortiert (die Reihenfolge von `list` hängt von der Erstellungssekunde ab).
    fn names(v: &[DocumentInfo]) -> Vec<String> {
        let mut n: Vec<String> = v.iter().map(|d| d.name.clone()).collect();
        n.sort();
        n
    }

    fn store() -> ProjectStore {
        let mut s = ProjectStore::open_in_memory().unwrap();
        add(
            &mut s,
            "pUC19",
            "AAGAATTCAA",
            Topology::Circular,
            vec![feat("gene", "lacZα", &[("note", "Beta-Galactosidase")])],
            &[("organism", "Escherichia coli")],
        );
        add(
            &mut s,
            "Insert A",
            "AAGAATCCAA",
            Topology::Linear,
            vec![feat("CDS", "gfp", &[])],
            &[("source", "Labor")],
        );
        add(
            &mut s,
            "insert b",
            "AAGAATCCAA",
            Topology::Linear,
            vec![],
            &[],
        );
        s
    }

    #[test]
    fn nach_name_und_typ() {
        let s = store();
        assert_eq!(
            names(&by_name(&s, "INSERT").unwrap()),
            vec!["Insert A", "insert b"]
        );
        assert_eq!(by_name(&s, "").unwrap().len(), 3);
        assert!(by_name(&s, "xyz").unwrap().is_empty());
        assert_eq!(by_type(&s, "Sequence").unwrap().len(), 3);
        assert!(by_type(&s, "tree").unwrap().is_empty());
    }

    #[test]
    fn nach_features_und_metadaten() {
        let s = store();
        assert_eq!(names(&by_feature_text(&s, "lacz").unwrap()), vec!["pUC19"]);
        assert_eq!(
            names(&by_feature_text(&s, "galactosidase").unwrap()),
            vec!["pUC19"]
        );
        assert_eq!(
            names(&by_feature_text(&s, "cds").unwrap()),
            vec!["Insert A"]
        );
        assert!(by_feature_text(&s, "").unwrap().is_empty());
        assert_eq!(
            names(&by_metadata(&s, "organism", "escherichia COLI").unwrap()),
            vec!["pUC19"]
        );
        assert_eq!(
            names(&by_metadata(&s, "source", "").unwrap()),
            vec!["Insert A"]
        );
        assert!(by_metadata(&s, "organism", "Hefe").unwrap().is_empty());
    }

    #[test]
    fn nach_motiv() {
        let s = store();
        // GAATTC nur in pUC19; GAATCC enthält Insert A/b, das Reverskomplement GGATTC wird ebenfalls gefunden
        assert_eq!(
            names(&by_sequence_motif(&s, "GAATTC").unwrap()),
            vec!["pUC19"]
        );
        assert_eq!(
            names(&by_sequence_motif(&s, "GGATTC").unwrap()),
            vec!["Insert A", "insert b"]
        );
        assert_eq!(
            names(&by_sequence_motif(&s, "GRATYC").unwrap()),
            vec!["Insert A", "insert b", "pUC19"]
        );
        assert!(by_sequence_motif(&s, "ZZ").is_err());
    }

    #[test]
    fn umbenennen() {
        let docs: Vec<DocumentInfo> = (0..3)
            .map(|i| DocumentInfo {
                id: format!("id{}", i),
                name: format!("alt{}", i),
                doc_type: "sequence".to_string(),
                created: 0,
            })
            .collect();
        let plan = batch_rename_plan(&docs, "Konstrukt_{n:03}");
        assert_eq!(plan["id0"], "Konstrukt_001");
        assert_eq!(plan["id2"], "Konstrukt_003");
        let plan2 = batch_rename_plan(&docs, "{name}-{type}-{n}-{x}");
        assert_eq!(plan2["id1"], "alt1-sequence-2-{x}");
        assert_eq!(batch_rename_plan(&docs, "{n:2}")["id0"], "01");
        assert!(batch_rename_plan(&[], "x").is_empty());
    }

    #[test]
    fn dubletten() {
        let mut s = store();
        add(
            &mut s,
            "ohne Gruppe",
            "GGGGGGGG",
            Topology::Linear,
            vec![],
            &[],
        );
        add(&mut s, "klein", "aagaatccaa", Topology::Linear, vec![], &[]);
        let groups = find_sequence_duplicates(&s).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(names(&groups[0]), vec!["Insert A", "insert b", "klein"]);
    }
}
