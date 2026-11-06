//! Operationen mit Netzwerkzugriff: Datensatz von NCBI oder UniProt laden. Der Abruf ist austauschbar (`HttpGet`), damit die Tests ohne Netz laufen.

use crate::db::ncbi::HttpGet;
use crate::db::uniprot_entry::{entry_url, parse_entry};
use crate::error::{Error, Result};
use crate::io::genbank;
use crate::model::document::Document;
use crate::ops::operation::{required, Operation, OperationContext, OperationResult, Params};

/// `db.ncbi.fetch`: lädt einen NCBI-Datensatz (`nuccore`) als GenBank. Keine Eingaben nötig (werden ignoriert).
/// Parameter: `id` (Pflicht, Kennung oder Akzession). Ergebnis: alle Sequenzdokumente aus `genbank::parse` des Textes von
/// `NcbiClient::fetch_genbank`; liefert der Text kein Dokument, `Error::Operation("NCBI lieferte keine Sequenz für {id}")`.
/// `summary` = `"{n} Sequenz(en) von NCBI"`. Fehler der Kernfunktionen als `Error::Operation` mit deren Text.
pub struct NcbiFetchOperation<H: HttpGet> {
    pub http: H,
}

impl<H: HttpGet> Operation for NcbiFetchOperation<H> {
    fn id(&self) -> &str {
        "db.ncbi.fetch"
    }

    fn label(&self) -> &str {
        "Von NCBI laden"
    }

    fn run(
        &self,
        _inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let id = required(params, "id")?;

        // Replikation von NcbiClient::fetch_genbank Logik, da http nicht geklont werden kann/konnte.
        let url = crate::db::ncbi::efetch_url("nuccore", id, "gb");
        let text = self
            .http
            .get(&url)
            .map_err(|e| Error::Operation(e.to_string()))?;

        let docs = genbank::parse(&text).map_err(|e| Error::Operation(e.to_string()))?;

        if docs.is_empty() {
            return Err(Error::Operation(format!(
                "NCBI lieferte keine Sequenz für {}",
                id
            )));
        }

        let n = docs.len();
        Ok(OperationResult {
            outputs: docs.into_iter().map(Document::Sequence).collect(),
            summary: format!("{} Sequenz(en) von NCBI", n),
        })
    }
}

/// `db.uniprot.fetch`: lädt einen UniProt-Eintrag. Parameter: `accession` (Pflicht). Ruft `entry_url(accession)` ab,
/// liest mit `parse_entry` und wandelt mit `to_document` um (Domänen usw. werden Features). Ergebnis: ein Protein-Dokument;
/// `summary` = `"{accession}: {n} aa, {k} Feature(s)"` (`n` Sequenzlänge, `k` Zahl der Features des erzeugten Dokuments).
/// Fehler als `Error::Operation` mit dem Text der Kernfunktion.
pub struct UniprotFetchOperation<H: HttpGet> {
    pub http: H,
}

impl<H: HttpGet> Operation for UniprotFetchOperation<H> {
    fn id(&self) -> &str {
        "db.uniprot.fetch"
    }

    fn label(&self) -> &str {
        "Von UniProt laden"
    }

    fn run(
        &self,
        _inputs: &[Document],
        params: &Params,
        _ctx: &mut dyn OperationContext,
    ) -> Result<OperationResult> {
        let accession = required(params, "accession")?;
        let url = entry_url(accession);
        let json = self
            .http
            .get(&url)
            .map_err(|e| Error::Operation(e.to_string()))?;
        let entry = parse_entry(&json).map_err(|e| Error::Operation(e.to_string()))?;
        let doc = entry
            .to_document()
            .map_err(|e| Error::Operation(e.to_string()))?;
        let n = doc.sequence().len();
        let k = doc.features().len();
        Ok(OperationResult {
            outputs: vec![Document::Sequence(doc)],
            summary: format!("{}: {} aa, {} Feature(s)", accession, n, k),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::operation::NullContext;
    use std::cell::RefCell;

    struct Fake {
        answer: Result<String>,
        urls: RefCell<Vec<String>>,
    }

    impl HttpGet for Fake {
        fn get(&self, url: &str) -> Result<String> {
            self.urls.borrow_mut().push(url.to_string());
            match &self.answer {
                Ok(s) => Ok(s.clone()),
                Err(e) => Err(Error::Io(e.to_string())),
            }
        }
    }

    fn params(items: &[(&str, &str)]) -> Params {
        items
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    const MINI: &str =
        "LOCUS       mini                      60 bp    DNA     circular   SYN 01-JAN-2024
DEFINITION  Testplasmid.
ACCESSION   TEST0001
FEATURES             Location/Qualifiers
     gene            5..20
                     /gene=\"abc\"
ORIGIN
        1 atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc atgcatgcat gcatgcatgc
//
";

    const ENTRY: &str = r#"{"primaryAccession":"P12345","uniProtkbId":"T_HUMAN",
      "sequence":{"value":"MKVLAAGIVGLLLAQPSTHEWK"},
      "features":[{"type":"Domain","description":"K","location":{"start":{"value":2},"end":{"value":9}}}]}"#;

    #[test]
    fn ncbi_laden() {
        let op = NcbiFetchOperation {
            http: Fake {
                answer: Ok(MINI.to_string()),
                urls: RefCell::new(Vec::new()),
            },
        };
        let r = op
            .run(&[], &params(&[("id", "L09137")]), &mut NullContext)
            .unwrap();
        assert_eq!(r.outputs.len(), 1);
        assert_eq!(r.summary, "1 Sequenz(en) von NCBI");
        assert!(op.http.urls.borrow()[0].contains("L09137"));
        match &r.outputs[0] {
            Document::Sequence(s) => assert_eq!(s.len(), 60),
            _ => panic!("keine Sequenz"),
        }
    }

    #[test]
    fn ncbi_fehler() {
        let op = NcbiFetchOperation {
            http: Fake {
                answer: Ok("kein genbank".to_string()),
                urls: RefCell::new(Vec::new()),
            },
        };
        assert!(op.run(&[], &Params::new(), &mut NullContext).is_err());
        let e = op
            .run(&[], &params(&[("id", "X1")]), &mut NullContext)
            .unwrap_err();
        assert!(e.to_string().contains("X1"));
        let down = NcbiFetchOperation {
            http: Fake {
                answer: Err(Error::Io("offline".to_string())),
                urls: RefCell::new(Vec::new()),
            },
        };
        assert!(matches!(
            down.run(&[], &params(&[("id", "X1")]), &mut NullContext),
            Err(Error::Operation(_))
        ));
    }

    #[test]
    fn uniprot_laden() {
        let op = UniprotFetchOperation {
            http: Fake {
                answer: Ok(ENTRY.to_string()),
                urls: RefCell::new(Vec::new()),
            },
        };
        let r = op
            .run(&[], &params(&[("accession", "P12345")]), &mut NullContext)
            .unwrap();
        assert_eq!(r.summary, "P12345: 22 aa, 1 Feature(s)");
        assert_eq!(
            op.http.urls.borrow()[0],
            "https://rest.uniprot.org/uniprotkb/P12345.json"
        );
        assert!(op.run(&[], &Params::new(), &mut NullContext).is_err());
    }

    #[test]
    fn uniprot_fehler() {
        let op = UniprotFetchOperation {
            http: Fake {
                answer: Ok("{}".to_string()),
                urls: RefCell::new(Vec::new()),
            },
        };
        assert!(matches!(
            op.run(&[], &params(&[("accession", "P1")]), &mut NullContext),
            Err(Error::Operation(_))
        ));
    }
}
