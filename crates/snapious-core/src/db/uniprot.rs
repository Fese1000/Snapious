//! UniProt: search and FASTA retrieval via the REST interface (`https://rest.uniprot.org`).

use crate::db::ncbi::{percent_encode, HttpGet};
use crate::error::{Error, Result};
use serde_json::Value;

/// `https://rest.uniprot.org/uniprotkb/search?query={encoded}&size={size}&format=json&fields=accession,id,protein_name,organism_name,length`
/// (percent encoding with `percent_encode`).
pub fn search_url(query: &str, size: usize) -> String {
    format!(
        "https://rest.uniprot.org/uniprotkb/search?query={}&size={}&format=json&fields=accession,id,protein_name,organism_name,length",
        percent_encode(query),
        size
    )
}

/// `https://rest.uniprot.org/uniprotkb/{accession encoded}.fasta`
pub fn fasta_url(accession: &str) -> String {
    format!(
        "https://rest.uniprot.org/uniprotkb/{}.fasta",
        percent_encode(accession)
    )
}

/// A search hit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProteinRecord {
    /// `primaryAccession`.
    pub accession: String,
    /// `uniProtkbId`.
    pub entry_name: String,
    /// `proteinDescription.recommendedName.fullName.value`, otherwise `proteinDescription.submissionNames[0].fullName.value`, otherwise empty.
    pub name: String,
    /// `organism.scientificName`, otherwise empty.
    pub organism: String,
    /// `sequence.length`, otherwise 0.
    pub length: u64,
}

/// Reads the search response: `{"results": [ {...}, ... ]}`. Not JSON, or `results` missing/not a list → `Error::InvalidInput`.
/// Entries without `primaryAccession` are skipped. Order as in the response.
pub fn parse_search(json: &str) -> Result<Vec<ProteinRecord>> {
    let root = serde_json::from_str::<Value>(json)
        .map_err(|e| Error::InvalidInput(format!("Invalid JSON: {}", e)))?;
    let results = root
        .get("results")
        .ok_or_else(|| Error::InvalidInput("No 'results' field".to_string()))?;
    let arr = results
        .as_array()
        .ok_or_else(|| Error::InvalidInput("'results' is not a list".to_string()))?;

    let mut records = Vec::with_capacity(arr.len());
    for item in arr {
        let acc = item
            .get("primaryAccession")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if acc.is_empty() {
            continue;
        }

        let entry_name = item
            .get("uniProtkbId")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let name = item
            .get("proteinDescription")
            .and_then(|pd| pd.get("recommendedName"))
            .and_then(|rn| rn.get("fullName"))
            .and_then(|fn_val| fn_val.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                item.get("proteinDescription")
                    .and_then(|pd| pd.get("submissionNames"))
                    .and_then(|sn| sn.as_array())
                    .and_then(|arr2| arr2.first())
                    .and_then(|first| first.get("fullName"))
                    .and_then(|fn_val2| fn_val2.get("value"))
                    .and_then(|v2| v2.as_str())
                    .unwrap_or("")
            })
            .to_string();

        let organism = item
            .get("organism")
            .and_then(|org| org.get("scientificName"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let length = item
            .get("sequence")
            .and_then(|seq| seq.get("length"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        records.push(ProteinRecord {
            accession: acc.to_string(),
            entry_name: entry_name.to_string(),
            name,
            organism: organism.to_string(),
            length,
        });
    }

    Ok(records)
}

/// Client with a replaceable fetch (`HttpGet` from `db::ncbi`).
pub struct UniprotClient<H: HttpGet> {
    pub http: H,
}

impl<H: HttpGet> UniprotClient<H> {
    pub fn new(http: H) -> Self {
        UniprotClient { http }
    }

    /// Searches with `search_url` and reads the hits.
    pub fn search(&self, query: &str, size: usize) -> Result<Vec<ProteinRecord>> {
        let url = search_url(query, size);
        let text = self.http.get(&url)?;
        parse_search(&text)
    }

    /// Downloads the entry as FASTA text. If the response (after whitespace) does not start with `>`, `Error::InvalidInput("Kein FASTA für {accession}")`.
    pub fn fetch_fasta(&self, accession: &str) -> Result<String> {
        let url = fasta_url(accession);
        let text = self.http.get(&url)?;
        let trimmed = text.trim_start();
        if !trimmed.starts_with('>') {
            return Err(Error::InvalidInput(format!("No FASTA for {}", accession)));
        }
        Ok(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Mock {
        replies: Vec<String>,
        calls: RefCell<Vec<String>>,
    }

    impl HttpGet for Mock {
        fn get(&self, url: &str) -> Result<String> {
            let n = self.calls.borrow().len();
            self.calls.borrow_mut().push(url.to_string());
            self.replies
                .get(n)
                .cloned()
                .ok_or_else(|| Error::Io("no response".to_string()))
        }
    }

    fn mock(replies: &[&str]) -> Mock {
        Mock {
            replies: replies.iter().map(|s| s.to_string()).collect(),
            calls: RefCell::new(Vec::new()),
        }
    }

    const ANTWORT: &str = r#"{"results":[
        {"entryType":"UniProtKB reviewed (Swiss-Prot)","primaryAccession":"P69892","uniProtkbId":"HBG2_HUMAN",
         "organism":{"scientificName":"Homo sapiens","commonName":"Human","taxonId":9606},
         "proteinDescription":{"recommendedName":{"fullName":{"value":"Hemoglobin subunit gamma-2"}},"alternativeNames":[]},
         "sequence":{"length":147}},
        {"primaryAccession":"A0A001","uniProtkbId":"X_Y",
         "proteinDescription":{"submissionNames":[{"fullName":{"value":"Eingereichter Name"}}]}},
        {"uniProtkbId":"OHNE_ACC"}
    ]}"#;

    #[test]
    fn adressen() {
        assert_eq!(
            search_url("hemoglobin alpha human", 5),
            "https://rest.uniprot.org/uniprotkb/search?query=hemoglobin%20alpha%20human&size=5&format=json&fields=accession,id,protein_name,organism_name,length"
        );
        assert_eq!(
            fasta_url("P69905"),
            "https://rest.uniprot.org/uniprotkb/P69905.fasta"
        );
        assert_eq!(
            fasta_url("a b"),
            "https://rest.uniprot.org/uniprotkb/a%20b.fasta"
        );
    }

    #[test]
    fn suche_lesen() {
        let r = parse_search(ANTWORT).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(
            r[0],
            ProteinRecord {
                accession: "P69892".to_string(),
                entry_name: "HBG2_HUMAN".to_string(),
                name: "Hemoglobin subunit gamma-2".to_string(),
                organism: "Homo sapiens".to_string(),
                length: 147,
            }
        );
        assert_eq!(r[1].accession, "A0A001");
        assert_eq!(r[1].name, "Eingereichter Name");
        assert_eq!(r[1].organism, "");
        assert_eq!(r[1].length, 0);
        assert!(parse_search(r#"{"results":[]}"#).unwrap().is_empty());
        assert!(matches!(parse_search("x"), Err(Error::InvalidInput(_))));
        assert!(matches!(parse_search("{}"), Err(Error::InvalidInput(_))));
        assert!(matches!(
            parse_search(r#"{"results":3}"#),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn client() {
        let c = UniprotClient::new(mock(&[ANTWORT, ">sp|P1|X test\nMKV\n", "kein fasta"]));
        assert_eq!(c.search("hemoglobin", 2).unwrap().len(), 2);
        assert!(
            c.http.calls.borrow()[0].contains("query=hemoglobin")
                && c.http.calls.borrow()[0].contains("size=2")
        );
        assert!(c.fetch_fasta("P1").unwrap().starts_with(">sp|P1|"));
        assert!(c.http.calls.borrow()[1].ends_with("P1.fasta"));
        match c.fetch_fasta("P2") {
            Err(Error::InvalidInput(m)) => assert_eq!(m, "No FASTA for P2"),
            other => panic!("unerwartet: {:?}", other),
        }
        let leer = UniprotClient::new(mock(&[]));
        assert!(matches!(leer.search("x", 1), Err(Error::Io(_))));
    }
}
