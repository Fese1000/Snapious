//! NCBI Entrez: build addresses, read JSON responses, download records.

use crate::error::{Error, Result};
use serde_json::Value;
use std::process::Command;

/// A simple GET fetch that returns the response text. Replaceable by a stub in tests.
pub trait HttpGet {
    fn get(&self, url: &str) -> Result<String>;
}

/// Fetch via the program `curl` (available on macOS and most Linux systems).
#[derive(Clone, Debug, Default)]
pub struct CurlClient;

impl HttpGet for CurlClient {
    fn get(&self, url: &str) -> Result<String> {
        let out = Command::new("curl")
            .args(["-sS", "-L", "--max-time", "60", "--fail", url])
            .output()
            .map_err(|e| Error::Io(format!("curl could not be started: {}", e)))?;
        if !out.status.success() {
            return Err(Error::Io(format!(
                "Fetch failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        String::from_utf8(out.stdout)
            .map_err(|_| Error::Io("Response is not UTF-8 text".to_string()))
    }
}

/// Percent encoding for address parts: letters, digits and `-`, `_`, `.`, `~` stay; all other bytes (UTF-8) become `%XX`
/// with uppercase hex (so a space becomes `%20`).
pub fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        if (byte.is_ascii_alphanumeric()) || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{:02X}", byte));
        }
    }
    out
}

const BASE: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/";

/// `{BASE}esearch.fcgi?db={db}&term={term encoded}&retmax={retmax}&retmode=json`
pub fn esearch_url(db: &str, term: &str, retmax: usize) -> String {
    format!(
        "{}esearch.fcgi?db={}&term={}&retmax={}&retmode=json",
        BASE,
        db,
        percent_encode(term),
        retmax
    )
}

/// `{BASE}esummary.fcgi?db={db}&id={ids joined by comma}&retmode=json` (the comma separator stays unencoded).
pub fn esummary_url(db: &str, ids: &[String]) -> String {
    let id_str = ids.join(",");
    format!("{}esummary.fcgi?db={}&id={}&retmode=json", BASE, db, id_str)
}

/// `{BASE}efetch.fcgi?db={db}&id={id}&rettype={rettype}&retmode=text`
pub fn efetch_url(db: &str, id: &str, rettype: &str) -> String {
    format!(
        "{}efetch.fcgi?db={}&id={}&rettype={}&retmode=text",
        BASE, db, id, rettype
    )
}

/// Result of a search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    /// Total number of hits (`count` as a number).
    pub count: usize,
    /// The returned identifiers in the order of `idlist`.
    pub ids: Vec<String>,
}

/// Reads the esearch response: `{"esearchresult": {"count": "2", "idlist": ["11", "22"]}}`.
/// `count` arrives as text and is converted to a number. If `esearchresult` is missing, the response is not JSON, or `count`
/// is not a number, the result is `Error::InvalidInput`. If `idlist` is missing, the list is empty. If the response has `esearchresult.ERROR`
/// (text), that is likewise `Error::InvalidInput` with this text.
pub fn parse_esearch(json: &str) -> Result<SearchResult> {
    let root = serde_json::from_str::<Value>(json)
        .map_err(|_| Error::InvalidInput("Invalid JSON".to_string()))?;

    let res = root
        .get("esearchresult")
        .ok_or_else(|| Error::InvalidInput("Missing esearchresult".to_string()))?;

    if let Some(err) = res.get("ERROR").and_then(|v| v.as_str()) {
        return Err(Error::InvalidInput(err.to_string()));
    }

    let count = res
        .get("count")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::InvalidInput("Missing or invalid count".to_string()))?
        .parse::<usize>()
        .map_err(|_| Error::InvalidInput("count is not a number".to_string()))?;

    let ids = res
        .get("idlist")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchResult { count, ids })
}

/// Summary of a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryRecord {
    pub id: String,
    /// `accessionversion`, otherwise `caption`, otherwise empty.
    pub accession: String,
    pub title: String,
    pub organism: String,
    /// `slen` (number or text containing a number), otherwise `None`.
    pub length: Option<u64>,
}

/// Reads the esummary response: `{"result": {"uids": ["1", "2"], "1": {...}, "2": {...}}}`. The order follows `uids`;
/// `uids` entries without an object are skipped. Missing fields become empty (`length`: `None`). The object's `uid` is
/// authoritative for `id`; if it is missing, the key is used. Errors as for `parse_esearch` (`result` missing or not JSON → `Error::InvalidInput`).
pub fn parse_esummary(json: &str) -> Result<Vec<SummaryRecord>> {
    let root = serde_json::from_str::<Value>(json)
        .map_err(|_| Error::InvalidInput("Invalid JSON".to_string()))?;

    let result = root
        .get("result")
        .ok_or_else(|| Error::InvalidInput("Missing result".to_string()))?;
    let uids = result
        .get("uids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::InvalidInput("Missing uids".to_string()))?;

    let mut records = Vec::new();
    for uid_val in uids {
        let uid = uid_val
            .as_str()
            .ok_or_else(|| Error::InvalidInput("uid is not a string".to_string()))?;
        let obj = result.get(uid).cloned();

        let obj = match obj {
            Some(o) => o,
            None => continue,
        };

        let id = obj
            .get("uid")
            .and_then(|v| v.as_str())
            .unwrap_or(uid)
            .to_string();
        let accession = obj
            .get("accessionversion")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("caption").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let organism = obj
            .get("organism")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let length = obj.get("slen").and_then(|v| v.as_u64()).or_else(|| {
            obj.get("slen")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
        });

        records.push(SummaryRecord {
            id,
            accession,
            title,
            organism,
            length,
        });
    }
    Ok(records)
}

/// Client with a replaceable fetch.
pub struct NcbiClient<H: HttpGet> {
    pub http: H,
}

impl<H: HttpGet> NcbiClient<H> {
    pub fn new(http: H) -> Self {
        Self { http }
    }

    /// Searches `db` for `term` (at most `retmax` hits) and fetches the summaries of the hits:
    /// first `esearch_url`, then (only with at least one identifier) `esummary_url`. Result: total number of hits and the summaries.
    pub fn search(
        &self,
        db: &str,
        term: &str,
        retmax: usize,
    ) -> Result<(usize, Vec<SummaryRecord>)> {
        let url = esearch_url(db, term, retmax);
        let json = self.http.get(&url)?;
        let res = parse_esearch(&json)?;

        if res.count == 0 || res.ids.is_empty() {
            return Ok((res.count, Vec::new()));
        }

        let summary_url = esummary_url(db, &res.ids);
        let json2 = self.http.get(&summary_url)?;
        let records = parse_esummary(&json2)?;

        Ok((res.count, records))
    }

    /// Downloads a record in GenBank flatfile format (`rettype=gb`) from `nuccore`. If the response is empty (whitespace only),
    /// `Error::InvalidInput("Leere Antwort für {id}")`.
    pub fn fetch_genbank(&self, id: &str) -> Result<String> {
        let url = efetch_url("nuccore", id, "gb");
        let text = self.http.get(&url)?;
        if text.trim().is_empty() {
            return Err(Error::InvalidInput(format!("Empty response for {}", id)));
        }
        Ok(text)
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

    impl Mock {
        fn new(replies: &[&str]) -> Mock {
            Mock {
                replies: replies.iter().map(|s| s.to_string()).collect(),
                calls: RefCell::new(Vec::new()),
            }
        }
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

    #[test]
    fn kodierung() {
        assert_eq!(percent_encode("abc-XYZ_0.9~"), "abc-XYZ_0.9~");
        assert_eq!(
            percent_encode("E. coli [Organism]"),
            "E.%20coli%20%5BOrganism%5D"
        );
        assert_eq!(percent_encode("ä"), "%C3%A4");
        assert_eq!(percent_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(percent_encode(""), "");
    }

    #[test]
    fn adressen() {
        assert_eq!(
            esearch_url("nuccore", "pUC19 AND plasmid", 5),
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=nuccore&term=pUC19%20AND%20plasmid&retmax=5&retmode=json"
        );
        assert_eq!(
            esummary_url("nuccore", &["1".to_string(), "22".to_string()]),
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=nuccore&id=1,22&retmode=json"
        );
        assert_eq!(
            efetch_url("nuccore", "L09137.2", "gb"),
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?db=nuccore&id=L09137.2&rettype=gb&retmode=text"
        );
    }

    #[test]
    fn esearch_lesen() {
        let r = parse_esearch(r#"{"header":{"type":"esearch"},"esearchresult":{"count":"120","retmax":"2","idlist":["11","22"]}}"#).unwrap();
        assert_eq!(
            r,
            SearchResult {
                count: 120,
                ids: vec!["11".to_string(), "22".to_string()]
            }
        );
        let leer = parse_esearch(r#"{"esearchresult":{"count":"0"}}"#).unwrap();
        assert_eq!(leer.count, 0);
        assert!(leer.ids.is_empty());
        assert!(matches!(
            parse_esearch("kein json"),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_esearch(r#"{"x":1}"#),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            parse_esearch(r#"{"esearchresult":{"count":"abc"}}"#),
            Err(Error::InvalidInput(_))
        ));
        match parse_esearch(r#"{"esearchresult":{"ERROR":"Invalid db name","count":"0"}}"#) {
            Err(Error::InvalidInput(m)) => assert!(m.contains("Invalid db name")),
            other => panic!("unerwartet: {:?}", other),
        }
    }

    const SUMMARY: &str = r#"{"result":{"uids":["11","22","33"],
        "11":{"uid":"11","caption":"L09137","accessionversion":"L09137.2","title":"Cloning vector pUC19","organism":"Cloning vector pUC19","slen":2686},
        "22":{"uid":"22","caption":"X1","title":"ohne Organismus","slen":"1500"}}}"#;

    #[test]
    fn esummary_lesen() {
        let r = parse_esummary(SUMMARY).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].id, "11");
        assert_eq!(r[0].accession, "L09137.2");
        assert_eq!(r[0].title, "Cloning vector pUC19");
        assert_eq!(r[0].organism, "Cloning vector pUC19");
        assert_eq!(r[0].length, Some(2686));
        assert_eq!(r[1].accession, "X1");
        assert_eq!(r[1].organism, "");
        assert_eq!(r[1].length, Some(1500));
        assert!(matches!(parse_esummary("[]"), Err(Error::InvalidInput(_))));
        assert!(matches!(parse_esummary("{}"), Err(Error::InvalidInput(_))));
        let ohne_laenge = parse_esummary(r#"{"result":{"uids":["5"],"5":{"title":"t"}}}"#).unwrap();
        assert_eq!(ohne_laenge[0].id, "5");
        assert_eq!(ohne_laenge[0].length, None);
        assert_eq!(ohne_laenge[0].accession, "");
    }

    #[test]
    fn suche_mit_attrappe() {
        let mock = Mock::new(&[
            r#"{"esearchresult":{"count":"57","idlist":["11","22"]}}"#,
            SUMMARY,
        ]);
        let client = NcbiClient::new(mock);
        let (count, recs) = client.search("nuccore", "pUC19", 2).unwrap();
        assert_eq!(count, 57);
        assert_eq!(recs.len(), 2);
        let calls = client.http.calls.borrow();
        assert_eq!(calls.len(), 2);
        assert!(
            calls[0].contains("esearch.fcgi")
                && calls[0].contains("term=pUC19")
                && calls[0].contains("retmax=2")
        );
        assert!(calls[1].contains("esummary.fcgi") && calls[1].contains("id=11,22"));
    }

    #[test]
    fn suche_ohne_treffer_fragt_nicht_nach() {
        let client = NcbiClient::new(Mock::new(&[
            r#"{"esearchresult":{"count":"0","idlist":[]}}"#,
        ]));
        let (count, recs) = client.search("nuccore", "nichts", 5).unwrap();
        assert_eq!(count, 0);
        assert!(recs.is_empty());
        assert_eq!(client.http.calls.borrow().len(), 1);
    }

    #[test]
    fn genbank_laden() {
        let client = NcbiClient::new(Mock::new(&["LOCUS       TEST\n//\n", "  \n"]));
        assert!(client
            .fetch_genbank("L09137.2")
            .unwrap()
            .starts_with("LOCUS"));
        assert!(
            client.http.calls.borrow()[0].contains("id=L09137.2")
                && client.http.calls.borrow()[0].contains("rettype=gb")
        );
        match client.fetch_genbank("X") {
            Err(Error::InvalidInput(m)) => assert_eq!(m, "Empty response for X"),
            other => panic!("unerwartet: {:?}", other),
        }
        let kaputt = NcbiClient::new(Mock::new(&[]));
        assert!(matches!(kaputt.fetch_genbank("Y"), Err(Error::Io(_))));
    }
}
