//! NCBI Entrez: Adressen bauen, JSON-Antworten lesen, Datensätze herunterladen.

use crate::error::{Error, Result};
use serde_json::Value;
use std::process::Command;

/// Ein einfacher GET-Abruf, der den Antworttext liefert. In Tests durch eine Attrappe ersetzbar.
pub trait HttpGet {
    fn get(&self, url: &str) -> Result<String>;
}

/// Abruf über das Programm `curl` (auf macOS und den meisten Linux-Systemen vorhanden).
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

/// Prozentkodierung für Adressteile: Buchstaben, Ziffern und `-`, `_`, `.`, `~` bleiben; alle anderen Bytes (UTF-8) werden zu `%XX`
/// mit Uppercase-Hex (Leerzeichen also `%20`).
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

/// `{BASE}esearch.fcgi?db={db}&term={term kodiert}&retmax={retmax}&retmode=json`
pub fn esearch_url(db: &str, term: &str, retmax: usize) -> String {
    format!(
        "{}esearch.fcgi?db={}&term={}&retmax={}&retmode=json",
        BASE,
        db,
        percent_encode(term),
        retmax
    )
}

/// `{BASE}esummary.fcgi?db={db}&id={ids mit Komma}&retmode=json` (die Komma-Trennung bleibt unkodiert).
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

/// Ergebnis einer Suche.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    /// Gesamtzahl der Treffer (`count` als Zahl).
    pub count: usize,
    /// Die zurückgegebenen Kennungen in Reihenfolge von `idlist`.
    pub ids: Vec<String>,
}

/// Liest die esearch-Antwort: `{"esearchresult": {"count": "2", "idlist": ["11", "22"]}}`.
/// `count` kommt als Text und wird in eine Zahl umgewandelt. Fehlt `esearchresult`, ist die Antwort kein JSON oder ist `count`
/// keine Zahl, gibt es `Error::InvalidInput`. Fehlt `idlist`, ist die Liste leer. Hat die Antwort `esearchresult.ERROR`
/// (Text), ist das ebenfalls `Error::InvalidInput` mit diesem Text.
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

/// Kurzinfo zu einem Datensatz.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryRecord {
    pub id: String,
    /// `accessionversion`, sonst `caption`, sonst leer.
    pub accession: String,
    pub title: String,
    pub organism: String,
    /// `slen` (Zahl oder Text mit Zahl), sonst `None`.
    pub length: Option<u64>,
}

/// Liest die esummary-Antwort: `{"result": {"uids": ["1", "2"], "1": {...}, "2": {...}}}`. Die Reihenfolge folgt `uids`;
/// `uids`-Einträge ohne Objekt werden übersprungen. Fehlende Felder werden leer (`length`: `None`). `uid` des Objekts ist
/// maßgeblich für `id`, fehlt es, gilt der Schlüssel. Fehler wie bei `parse_esearch` (`result` fehlt oder kein JSON → `Error::InvalidInput`).
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

/// Client mit austauschbarem Abruf.
pub struct NcbiClient<H: HttpGet> {
    pub http: H,
}

impl<H: HttpGet> NcbiClient<H> {
    pub fn new(http: H) -> Self {
        Self { http }
    }

    /// Sucht in `db` nach `term` (höchstens `retmax` Treffer) und holt die Kurzinfos der Treffer:
    /// erst `esearch_url`, dann (nur bei mindestens einer Kennung) `esummary_url`. Ergebnis: Gesamtzahl der Treffer und die Kurzinfos.
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

    /// Lädt einen Datensatz im GenBank-Flatfile-Format (`rettype=gb`) aus `nuccore`. Ist die Antwort leer (nur Leerraum),
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
