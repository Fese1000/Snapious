//! BLAST über die NCBI-URL-API: Anfrage absenden, Status abfragen, Ergebnis lesen. Netzwerk und Wartezeiten sind austauschbar.

use crate::bio::blast::{parse_tabular, BlastHit};
use crate::db::ncbi::percent_encode;
use crate::error::{Error, Result};

/// Basisadresse der NCBI-BLASTAPI.
pub const BLAST_URL: &str = "https://blast.ncbi.nlm.nih.gov/Blast.cgi";

/// Abruf und Absenden (in Tests ersetzbar).
pub trait BlastTransport {
    fn get(&self, url: &str) -> Result<String>;
    /// Sendet einen Formularkörper (`application/x-www-form-urlencoded`) an `url` und liefert den Antworttext.
    fn post(&self, url: &str, body: &str) -> Result<String>;
}

/// Wartet die angegebene Zeit (in Tests ersetzbar).
pub trait Sleeper {
    fn sleep_secs(&self, secs: u64);
}

/// Zustand einer Anfrage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlastStatus {
    Waiting,
    Ready,
    Failed,
    Unknown,
}

/// Formularkörper zum Absenden: `CMD=Put&PROGRAM=<program>&DATABASE=<database>&QUERY=<query>`; `query` wird mit
/// `percent_encode` kodiert, `program` und `database` bleiben unverändert.
pub fn submit_body(program: &str, database: &str, query: &str) -> String {
    format!(
        "CMD=Put&PROGRAM={}&DATABASE={}&QUERY={}",
        program,
        database,
        percent_encode(query)
    )
}

/// Adresse der Statusabfrage: `{BLAST_URL}?CMD=Get&FORMAT_OBJECT=SearchInfo&RID=<rid>`.
pub fn status_url(rid: &str) -> String {
    format!("{}?CMD=Get&FORMAT_OBJECT=SearchInfo&RID={}", BLAST_URL, rid)
}

/// Adresse des Ergebnisses: `{BLAST_URL}?CMD=Get&FORMAT_TYPE=Tabular&RID=<rid>`.
pub fn results_url(rid: &str) -> String {
    format!("{}?CMD=Get&FORMAT_TYPE=Tabular&RID={}", BLAST_URL, rid)
}

/// Liest `(RID, RTOE)` aus der Antwort auf das Absenden: Zeilen der Form `    RID = ABC123` und `    RTOE = 17`
/// (Leerraum um Zeilenanfang und `=` beliebig; die Zeilen dürfen in Text oder HTML stehen, es zählt jede Zeile, die nach dem Kürzen
/// mit `RID` bzw. `RTOE` beginnt, gefolgt von optionalem Leerraum und `=`). Fehlt die RID oder ist sie leer → `Error::Operation`;
/// fehlt RTOE oder ist keine ganze Zahl, ist sie 0.
pub fn parse_submit(text: &str) -> Result<(String, u64)> {
    let mut rid = None;
    let mut rtoe: Option<u64> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("RID") {
            if let Some(eq_pos) = trimmed.find('=') {
                let val = trimmed[eq_pos + 1..].trim();
                if !val.is_empty() {
                    rid = Some(val.to_string());
                } else {
                    return Err(Error::Operation("Missing RID".to_string()));
                }
            } else {
                return Err(Error::Operation("Missing RID".to_string()));
            }
        } else if trimmed.starts_with("RTOE") {
            if let Some(eq_pos) = trimmed.find('=') {
                let val = trimmed[eq_pos + 1..].trim();
                if let Ok(n) = val.parse::<u64>() {
                    rtoe = Some(n);
                }
            }
        }
    }

    let rid = match rid {
        Some(r) => r,
        None => return Err(Error::Operation("Missing RID".to_string())),
    };

    Ok((rid, rtoe.unwrap_or(0)))
}

/// Liest den Status aus `Status=WAITING|READY|FAILED|UNKNOWN` (irgendwo im Text, Groß-/Kleinschreibung der Werte egal);
/// ohne `Status=` → `BlastStatus::Unknown`.
pub fn parse_status(text: &str) -> BlastStatus {
    if let Some(pos) = text.find("Status=") {
        let after = &text[pos + 7..];
        let end = after
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(after.len());
        let status_val = after[..end].to_lowercase();
        match status_val.as_str() {
            "waiting" => BlastStatus::Waiting,
            "ready" => BlastStatus::Ready,
            "failed" => BlastStatus::Failed,
            "unknown" => BlastStatus::Unknown,
            _ => BlastStatus::Unknown,
        }
    } else {
        BlastStatus::Unknown
    }
}

/// Führt eine Anfrage aus.
/// 1. `post(BLAST_URL, submit_body(..))` → `parse_submit` (RID, RTOE).
/// 2. `sleeper.sleep_secs(max(RTOE, 10))`.
/// 3. Wiederhole höchstens 60-mal: `get(status_url)` → `parse_status`: `Ready` → `get(results_url)`, mit `parse_tabular` lesen und
///    zurückgeben; `Waiting` → `sleeper.sleep_secs(60)` und weiter; `Failed` oder `Unknown` → `Error::Operation` mit Text und RID.
/// 4. Nach 60 Versuchen: `Error::Operation("BLAST: Zeitüberschreitung (RID …)")`.
pub fn run_remote(
    transport: &dyn BlastTransport,
    sleeper: &dyn Sleeper,
    program: &str,
    database: &str,
    query: &str,
) -> Result<Vec<BlastHit>> {
    let body = submit_body(program, database, query);
    let post_resp = transport.post(BLAST_URL, &body)?;
    let (rid, rtoe) = parse_submit(&post_resp)?;

    sleeper.sleep_secs(rtoe.max(10));

    for _ in 0..60 {
        let status_resp = transport.get(&status_url(&rid))?;
        match parse_status(&status_resp) {
            BlastStatus::Ready => {
                let res_resp = transport.get(&results_url(&rid))?;
                return parse_tabular(&res_resp);
            }
            BlastStatus::Waiting => {
                sleeper.sleep_secs(60);
            }
            _ => {
                return Err(Error::Operation(format!(
                    "BLAST failed (RID {})",
                    rid
                )));
            }
        }
    }

    Err(Error::Operation(format!(
        "BLAST: timeout (RID {})",
        rid
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        posts: RefCell<Vec<(String, String)>>,
        gets: RefCell<Vec<String>>,
        statuses: RefCell<Vec<String>>,
    }

    impl BlastTransport for Fake {
        fn get(&self, url: &str) -> Result<String> {
            self.gets.borrow_mut().push(url.to_string());
            if url.contains("FORMAT_OBJECT=SearchInfo") {
                Ok(self.statuses.borrow_mut().remove(0))
            } else {
                Ok(
                    "# Kommentar\nq1\ts1\t98.5\t100\t1\t0\t1\t100\t11\t110\t1e-45\t180\n"
                        .to_string(),
                )
            }
        }

        fn post(&self, url: &str, body: &str) -> Result<String> {
            self.posts
                .borrow_mut()
                .push((url.to_string(), body.to_string()));
            Ok("<p>\n    RID = ABC123\n    RTOE = 15\n</p>".to_string())
        }
    }

    struct Sleeps(RefCell<Vec<u64>>);

    impl Sleeper for Sleeps {
        fn sleep_secs(&self, secs: u64) {
            self.0.borrow_mut().push(secs);
        }
    }

    fn fake(statuses: &[&str]) -> Fake {
        Fake {
            posts: RefCell::new(Vec::new()),
            gets: RefCell::new(Vec::new()),
            statuses: RefCell::new(statuses.iter().map(|s| s.to_string()).collect()),
        }
    }

    #[test]
    fn adressen() {
        assert_eq!(
            submit_body("blastn", "nt", "ACGT AC"),
            "CMD=Put&PROGRAM=blastn&DATABASE=nt&QUERY=ACGT%20AC"
        );
        assert_eq!(
            status_url("R1"),
            "https://blast.ncbi.nlm.nih.gov/Blast.cgi?CMD=Get&FORMAT_OBJECT=SearchInfo&RID=R1"
        );
        assert_eq!(
            results_url("R1"),
            "https://blast.ncbi.nlm.nih.gov/Blast.cgi?CMD=Get&FORMAT_TYPE=Tabular&RID=R1"
        );
    }

    #[test]
    fn antworten_lesen() {
        let (rid, rtoe) = parse_submit("<html>\n   RID = XYZ789   \n  RTOE=  42\n</html>").unwrap();
        assert_eq!((rid.as_str(), rtoe), ("XYZ789", 42));
        let (_, rtoe) = parse_submit("RID = A\n").unwrap();
        assert_eq!(rtoe, 0);
        assert!(parse_submit("kein Treffer").is_err());
        assert!(parse_submit("RID = \nRTOE = 3").is_err());
        assert_eq!(
            parse_status("QBlastInfoBegin\n\tStatus=WAITING\nQBlastInfoEnd"),
            BlastStatus::Waiting
        );
        assert_eq!(
            parse_status("Status=READY\nThereAreHits=yes"),
            BlastStatus::Ready
        );
        assert_eq!(parse_status("Status=failed"), BlastStatus::Failed);
        assert_eq!(parse_status("Status=UNKNOWN"), BlastStatus::Unknown);
        assert_eq!(parse_status("nichts"), BlastStatus::Unknown);
    }

    #[test]
    fn ablauf_mit_wartezeiten() {
        let f = fake(&["Status=WAITING", "Status=READY"]);
        let s = Sleeps(RefCell::new(Vec::new()));
        let hits = run_remote(&f, &s, "blastn", "nt", "ACGT").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].subject, "s1");
        assert_eq!(*s.0.borrow(), vec![15, 60]);
        assert_eq!(f.posts.borrow()[0].0, BLAST_URL);
        assert_eq!(
            f.posts.borrow()[0].1,
            "CMD=Put&PROGRAM=blastn&DATABASE=nt&QUERY=ACGT"
        );
        assert_eq!(f.gets.borrow().len(), 3);
        assert!(f.gets.borrow()[2].contains("FORMAT_TYPE=Tabular"));
    }

    #[test]
    fn fehlerfaelle() {
        let f = fake(&["Status=FAILED"]);
        let s = Sleeps(RefCell::new(Vec::new()));
        let e = run_remote(&f, &s, "blastn", "nt", "ACGT").unwrap_err();
        assert!(e.to_string().contains("ABC123"));
        let waiting: Vec<&str> = vec!["Status=WAITING"; 60];
        let f2 = fake(&waiting);
        let s2 = Sleeps(RefCell::new(Vec::new()));
        let e2 = run_remote(&f2, &s2, "blastn", "nt", "ACGT").unwrap_err();
        assert!(e2.to_string().contains("timeout"));
        assert_eq!(s2.0.borrow().len(), 61);
        assert!(matches!(e2, Error::Operation(_)));
    }
}
