//! ACE-Dateien (Assemblies von Phrap/CAP3) lesen.

use crate::error::{Error, Result};

/// Ein Read im Contig.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AceRead {
    pub name: String,
    /// Position im Contig (1-basiert wie in der Datei, kann ≤ 0 sein).
    pub offset: i64,
    /// `true`, wenn der Read komplementär (`C`) liegt.
    pub complement: bool,
    pub sequence: String,
}

/// Ein Contig.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AceContig {
    pub name: String,
    /// Konsens ohne Zeilenumbrüche, mit `*` an Lückenstellen.
    pub consensus: String,
    /// Reads in der Reihenfolge der `AF`-Zeilen.
    pub reads: Vec<AceRead>,
}

/// Liest eine ACE-Datei. Zeilen:
/// * `AS ...`: Kopf, ignorieren.
/// * `CO name nbases nreads nsegs strand`: beginnt ein Contig; die folgenden Zeilen bis zur ersten leeren Zeile sind der Konsens
///   (Leerraum entfernt, zusammengesetzt).
/// * `BQ`: Qualitätszeilen bis zur nächsten leeren Zeile überspringen.
/// * `AF name U|C offset`: Read des aktuellen Contigs (`C` = komplementär), Reihenfolge merken.
/// * `RD name len a b`: danach Sequenzzeilen bis zur leeren Zeile (Leerraum entfernt); gehört zum Read mit diesem Namen aus den `AF`-Zeilen.
/// * `QA ...`, `DS ...`, `BS ...`, `WA{`/`}`-Blöcke und alle anderen Zeilen: ignorieren.
///
/// Fehler `Error::InvalidInput`: `AF`/`RD`/`CO` mit zu wenigen Feldern oder nicht lesbarem Versatz, ein `RD` ohne passenden `AF`,
/// ein `AF` ohne `RD` oder keine Contigs.
pub fn parse_ace(text: &str) -> Result<Vec<AceContig>> {
    let mut contigs: Vec<AceContig> = Vec::new();
    let mut lines = text.lines().peekable();
    let mut pending: Vec<(String, Option<String>)> = Vec::new(); // (Read-Name, Sequenz) des aktuellen Contigs

    fn finish(
        contigs: &mut [AceContig],
        pending: &mut Vec<(String, Option<String>)>,
    ) -> Result<()> {
        if let Some(c) = contigs.last_mut() {
            for (i, (name, seq)) in pending.drain(..).enumerate() {
                match seq {
                    Some(s) => c.reads[i].sequence = s,
                    None => {
                        return Err(Error::InvalidInput(format!(
                            "Read '{name}' has no sequence (RD missing)"
                        )))
                    }
                }
            }
        }
        pending.clear();
        Ok(())
    }

    while let Some(line) = lines.next() {
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.first().copied() {
            Some("CO") => {
                finish(&mut contigs, &mut pending)?;
                if f.len() < 6 {
                    return Err(Error::InvalidInput(
                        "CO line has too few fields".to_string(),
                    ));
                }
                let mut consensus = String::new();
                for l in lines.by_ref() {
                    if l.trim().is_empty() {
                        break;
                    }
                    consensus.extend(l.chars().filter(|c| !c.is_whitespace()));
                }
                contigs.push(AceContig {
                    name: f[1].to_string(),
                    consensus,
                    reads: Vec::new(),
                });
            }
            Some("BQ") => {
                for l in lines.by_ref() {
                    if l.trim().is_empty() {
                        break;
                    }
                }
            }
            Some("AF") => {
                if f.len() < 4 || contigs.is_empty() {
                    return Err(Error::InvalidInput("Invalid AF line".to_string()));
                }
                let offset = f[3]
                    .parse::<i64>()
                    .map_err(|_| Error::InvalidInput(format!("Invalid offset '{}'", f[3])))?;
                if let Some(c) = contigs.last_mut() {
                    c.reads.push(AceRead {
                        name: f[1].to_string(),
                        offset,
                        complement: f[2] == "C",
                        sequence: String::new(),
                    });
                }
                pending.push((f[1].to_string(), None));
            }
            Some("RD") => {
                if f.len() < 5 || contigs.is_empty() {
                    return Err(Error::InvalidInput("Invalid RD line".to_string()));
                }
                let mut seq = String::new();
                for l in lines.by_ref() {
                    if l.trim().is_empty() {
                        break;
                    }
                    seq.extend(l.chars().filter(|c| !c.is_whitespace()));
                }
                match pending.iter_mut().find(|(n, s)| n == f[1] && s.is_none()) {
                    Some(slot) => slot.1 = Some(seq),
                    None => {
                        return Err(Error::InvalidInput(format!(
                            "Read '{}' has no AF line",
                            f[1]
                        )))
                    }
                }
            }
            _ => {}
        }
    }
    finish(&mut contigs, &mut pending)?;
    if contigs.is_empty() {
        return Err(Error::InvalidInput(
            "No contigs in the ACE file".to_string(),
        ));
    }
    Ok(contigs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACE: &str = "AS 1 2\n\nCO contig1 8 2 0 U\nACGTAC*G\n\nBQ\n 30 30 30 30 30 30 30\n\nAF r1 U 1\nAF r2 C -2\n\nBS 1 5 r1\n\nRD r1 5 0 0\nACGTA\n\nQA 1 5 1 5\nDS CHROMAT_FILE: x\n\nRD r2 6 0 0\nGT*CGT\n\nQA 1 6 1 6\n";

    #[test]
    fn contig_mit_zwei_reads() {
        let c = parse_ace(ACE).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].name, "contig1");
        assert_eq!(c[0].consensus, "ACGTAC*G");
        assert_eq!(
            c[0].reads,
            vec![
                AceRead {
                    name: "r1".into(),
                    offset: 1,
                    complement: false,
                    sequence: "ACGTA".into()
                },
                AceRead {
                    name: "r2".into(),
                    offset: -2,
                    complement: true,
                    sequence: "GT*CGT".into()
                },
            ]
        );
    }

    #[test]
    fn zwei_contigs_und_fehler() {
        let zwei = format!(
            "{ACE}\nCO contig2 4 1 0 U\nTTTT\n\nBQ\n 1 1 1 1\n\nAF s1 U 1\n\nRD s1 4 0 0\nTTTT\n\n"
        );
        let c = parse_ace(&zwei).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].reads[0].sequence, "TTTT");
        assert!(parse_ace("AS 0 0\n").is_err());
        assert!(parse_ace("CO c 4 1 0 U\nAAAA\n\nRD x 4 0 0\nAAAA\n\n").is_err());
        assert!(parse_ace("CO c 4 1 0 U\nAAAA\n\nAF x U 1\n\n").is_err());
        assert!(parse_ace("CO c 4 1 0 U\nAAAA\n\nAF x U abc\n\n").is_err());
    }
}
