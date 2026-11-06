//! Sequenzierungsabgleich („Align to Reference“): einen Read gegen eine Referenz ausrichten und Unterschiede melden.

use crate::bio::align::Aligner;
use crate::cloning::enzymes::iupac_reverse_complement;
use crate::error::{Error, Result};

/// Art eines Unterschieds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DifferenceType {
    Mismatch,
    Insertion,
    Deletion,
    LowQuality,
}

/// Ein Unterschied zwischen Read und Referenz.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    pub kind: DifferenceType,
    /// Position in der Referenz (0-basiert). Bei einer Einfügung die Position der nächsten Referenzbase, bei zirkulärer Referenz modulo Länge.
    pub reference_position: usize,
    /// Position im ausgerichteten Read (bei `reversed` im reverse-komplementierten Read), 0-basiert. Bei einer Löschung
    /// der Index der nächsten Readbase.
    pub read_position: usize,
    /// Base der Referenz, leer bei einer Einfügung.
    pub expected: String,
    /// Base des Reads, leer bei einer Löschung.
    pub observed: String,
    /// Qualität der Readbase (bei einer Löschung 40).
    pub quality: u8,
}

/// Ergebnis des Abgleichs.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckResult {
    pub read_name: String,
    /// `true`, wenn der reverse-komplementierte Read besser passt.
    pub reversed: bool,
    /// Ausgerichteter Referenzbereich: Beginn und Ende (exklusiv). Bei zirkulärer Referenz kann `reference_end < reference_start` sein
    /// (Bereich über den Nullpunkt); liegt das Ende genau auf der Länge, ist es die Länge.
    pub reference_start: usize,
    pub reference_end: usize,
    /// Anteil der Spalten mit gleichen Basen an allen Spalten des Alignments in Prozent.
    pub identity: f64,
    /// Sortiert nach `reference_position`, dann `read_position`, dann Art (Reihenfolge der Varianten von `DifferenceType`).
    pub differences: Vec<Difference>,
    pub aligned_read: String,
    pub aligned_reference: String,
}

/// Einstellung: Basen mit Qualität unter `min_quality` werden zusätzlich als `LowQuality` gemeldet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequencingCheck {
    pub min_quality: u8,
}

impl Default for SequencingCheck {
    /// `min_quality` 20.
    fn default() -> Self {
        SequencingCheck { min_quality: 20 }
    }
}

impl SequencingCheck {
    pub fn new(min_quality: u8) -> SequencingCheck {
        SequencingCheck { min_quality }
    }

    /// Richtet `read` gegen `reference` aus.
    ///
    /// 1. Fehler `Error::InvalidInput` bei leerer Referenz, leerem Read oder wenn `quality` weder leer noch so lang wie `read` ist.
    ///    Eine leere `quality` zählt als „überall 40“. Alle Basen werden in Großbuchstaben mit `T` statt `U` gelesen.
    /// 2. Der Read und sein Reverse-Komplement (`iupac_reverse_complement`, Qualitäten umgekehrt) werden mit
    ///    `Aligner::new(2, -3, -5).semi_global(read, referenz)` ausgerichtet (Ergebnis: `seq1_aln` = Read, `seq2_aln` = Referenz;
    ///    Bereich in der Referenz: `seq2_start..seq2_end`). Bei zirkulärer Referenz wird `referenz + referenz` verwendet. Gewinnt die
    ///    Richtung mit dem höheren `score` (bei Gleichstand vorwärts); `reversed` zeigt, ob es der reverse-komplementierte Read war.
    /// 3. Zirkulär: `reference_start = seq2_start % Länge`, `reference_end = seq2_end − Länge`, wenn `seq2_end > Länge`, sonst `seq2_end`
    ///    (beim Ende genau auf der Länge bleibt die Länge); Positionen der Unterschiede werden modulo Länge angegeben.
    ///    Linear: `reference_start = seq2_start`, `reference_end = seq2_end`.
    /// 4. Spalte für Spalte durch das Alignment (Referenzindex und Readindex mitzählen):
    ///    - beide Zeichen vorhanden und verschieden → `Mismatch` (`expected` Referenzbase, `observed` Readbase);
    ///    - Lücke in der Referenz, Readbase vorhanden → `Insertion` (`reference_position` = Index der nächsten Referenzbase);
    ///    - Lücke im Read, Referenzbase vorhanden → `Deletion` (`read_position` = Index der nächsten Readbase, `quality` 40).
    ///
    ///    Zusätzlich für jede Readbase mit Qualität `< min_quality` (auch bei Übereinstimmung und bei Einfügungen) ein
    ///    `LowQuality`-Eintrag (`expected` Referenzbase oder leer bei Einfügung, `observed` Readbase, `quality` der Base).
    ///
    /// 5. `identity` = Spalten mit gleichen Zeichen ÷ alle Spalten × 100.
    pub fn check(
        &self,
        reference: &str,
        circular: bool,
        read_name: &str,
        read: &str,
        quality: &[u8],
    ) -> Result<CheckResult> {
        if reference.is_empty() || read.is_empty() {
            return Err(Error::InvalidInput(
                "Referenz oder Read darf nicht leer sein".into(),
            ));
        }
        if !quality.is_empty() && quality.len() != read.len() {
            return Err(Error::InvalidInput(
                "Qualitätsvektor muss leer oder gleich lang wie Read sein".into(),
            ));
        }

        let ref_len = reference.len();
        let read_norm = read.to_uppercase().replace('U', "T");
        let ref_norm = reference.to_uppercase().replace('U', "T");

        // Qualitäten normalisieren: leer bedeutet 40 für alle
        let quals: Vec<u8> = if quality.is_empty() {
            vec![40u8; read.len()]
        } else {
            quality.to_vec()
        };

        // Reverse-Qualitäten vorbereiten (für den Fall, dass der reverse-komplementierte Read gewinnt)
        let rc_quals: Vec<u8> = quals.iter().rev().cloned().collect();

        let aligner = Aligner::new(2, -3, -5);

        // Forward-Alignment
        let aln_ref = if circular {
            format!("{}{}", ref_norm, ref_norm)
        } else {
            ref_norm.clone()
        };

        let fwd = aligner.semi_global(&read_norm, &aln_ref)?;

        // Reverse-Alignment: Read wird reverse-komplementiert, Qualitäten werden umgekehrt
        let rc_read = iupac_reverse_complement(&read_norm);
        let bwd = aligner.semi_global(&rc_read, &aln_ref)?;

        // Entscheiden, welche Ausrichtung besser ist (höherer Score gewinnt, bei Gleichstand Forward)
        let (best_fwd, reversed) = if fwd.score >= bwd.score {
            (fwd, false)
        } else {
            (bwd, true)
        };

        let seq1_aln = best_fwd.seq1_aln;
        let seq2_aln = best_fwd.seq2_aln;

        // Referenzbereich berechnen
        let (reference_start, reference_end) = if circular {
            let s = best_fwd.seq2_start;
            let e = best_fwd.seq2_end;
            (s % ref_len, if e > ref_len { e - ref_len } else { e })
        } else {
            (best_fwd.seq2_start, best_fwd.seq2_end)
        };

        let mut differences: Vec<Difference> = Vec::new();
        let mut matching_cols = 0usize;
        // Gesamtlänge des Alignments (Anzahl der Spalten)
        let total_cols = seq1_aln.len();

        // Indizes für die ursprünglichen Sequenzen (vor dem Alignment)
        let mut orig_read_idx = 0usize;
        let mut orig_ref_idx = best_fwd.seq2_start;

        // Die relevanten Qualitäten basierend auf der gewählten Richtung
        let processing_quals = if reversed { &rc_quals } else { &quals };

        // Durch das Alignment iterieren
        for (c1, c2) in seq1_aln.chars().zip(seq2_aln.chars()) {
            let is_gap1 = c1 == '-';
            let is_gap2 = c2 == '-';

            if !is_gap1 && !is_gap2 {
                // Beide Zeichen vorhanden (Match oder Mismatch)
                if c1 == c2 {
                    matching_cols += 1;
                } else {
                    let ref_pos = if circular {
                        orig_ref_idx % ref_len
                    } else {
                        orig_ref_idx
                    };
                    let q = processing_quals.get(orig_read_idx).copied().unwrap_or(40);
                    differences.push(Difference {
                        kind: DifferenceType::Mismatch,
                        reference_position: ref_pos,
                        read_position: orig_read_idx,
                        expected: c2.to_string(),
                        observed: c1.to_string(),
                        quality: q,
                    });
                }

                // Qualitätsprüfung für LowQuality-Eintrag
                let q = processing_quals.get(orig_read_idx).copied().unwrap_or(40);
                if q < self.min_quality {
                    differences.push(Difference {
                        kind: DifferenceType::LowQuality,
                        reference_position: if circular {
                            orig_ref_idx % ref_len
                        } else {
                            orig_ref_idx
                        },
                        read_position: orig_read_idx,
                        expected: c2.to_string(),
                        observed: c1.to_string(),
                        quality: q,
                    });
                }

                orig_read_idx += 1;
                orig_ref_idx += 1;
            } else if is_gap1 && !is_gap2 {
                // Lücke im Read (Deletion in der Referenz bezogen auf den Read, oder Insertion im Read?)
                // Laut Aufgabenstellung: "Lücke im Read, Referenzbase vorhanden → Deletion"
                // Das bedeutet, die Referenz hat eine Base, der Read nicht.
                let ref_pos = if circular {
                    orig_ref_idx % ref_len
                } else {
                    orig_ref_idx
                };
                differences.push(Difference {
                    kind: DifferenceType::Deletion,
                    reference_position: ref_pos,
                    read_position: orig_read_idx, // Index der nächsten Readbase (da Lücke im Read)
                    expected: c2.to_string(),     // Referenzbase
                    observed: String::new(),      // Keine Readbase
                    quality: 40,                  // Standardwert für Deletionen
                });
                orig_ref_idx += 1;
            } else if !is_gap1 && is_gap2 {
                // Lücke in der Referenz (Insertion im Read)
                let ref_pos = if circular {
                    orig_ref_idx % ref_len
                } else {
                    orig_ref_idx
                };
                let q = processing_quals.get(orig_read_idx).copied().unwrap_or(40);
                differences.push(Difference {
                    kind: DifferenceType::Insertion,
                    reference_position: ref_pos, // Position der nächsten Referenzbase (oder aktuelle, da Lücke in Ref)
                    read_position: orig_read_idx,
                    expected: String::new(),  // Keine Referenzbase
                    observed: c1.to_string(), // Readbase
                    quality: q,
                });

                // Qualitätsprüfung für LowQuality bei Einfügung
                if q < self.min_quality {
                    differences.push(Difference {
                        kind: DifferenceType::LowQuality,
                        reference_position: ref_pos,
                        read_position: orig_read_idx,
                        expected: String::new(), // Keine Referenzbase bei Insertion
                        observed: c1.to_string(), // Readbase
                        quality: q,
                    });
                }

                orig_read_idx += 1;
            } else {
                // Beide Lücken (sollte theoretisch nicht vorkommen in sinnvollem Alignment, aber sicherheitshalber)
                orig_read_idx += 1;
                orig_ref_idx += 1;
            }
        }

        // Sortieren der Unterschiede gemäß Spezifikation
        differences.sort_by(|a, b| {
            a.reference_position
                .cmp(&b.reference_position)
                .then_with(|| a.read_position.cmp(&b.read_position))
                .then_with(|| a.kind.cmp(&b.kind))
        });

        // Identität berechnen
        let identity = if total_cols == 0 {
            100.0
        } else {
            (matching_cols as f64 / total_cols as f64) * 100.0
        };

        Ok(CheckResult {
            read_name: read_name.to_string(),
            reversed,
            reference_start,
            reference_end,
            identity,
            differences,
            aligned_read: seq1_aln,
            aligned_reference: seq2_aln,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministische, nicht wiederholende Testfolge.
    fn seq(n: usize, seed: u64) -> String {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ['A', 'C', 'G', 'T'][((s >> 33) % 4) as usize]
            })
            .collect()
    }

    fn other_base(not: &[char]) -> char {
        ['A', 'C', 'G', 'T']
            .into_iter()
            .find(|b| !not.contains(b))
            .unwrap()
    }

    fn rc(s: &str) -> String {
        iupac_reverse_complement(s)
    }

    fn reference() -> String {
        seq(100, 11)
    }

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn exakter_read() {
        let r = reference();
        let read = r[10..40].to_string();
        let res = SequencingCheck::default()
            .check(&r, false, "r1", &read, &[])
            .unwrap();
        assert_eq!(res.read_name, "r1");
        assert!(!res.reversed);
        assert_eq!((res.reference_start, res.reference_end), (10, 40));
        assert!((res.identity - 100.0).abs() < 1e-9);
        assert!(res.differences.is_empty());
        assert_eq!(res.aligned_read, read);
        assert_eq!(res.aligned_reference, read);
    }

    #[test]
    fn austausch_und_rueckwaerts() {
        let r = reference();
        let mut read = chars(&r[10..40]);
        let alt = other_base(&[read[8]]);
        let old = read[8];
        read[8] = alt;
        let read: String = read.into_iter().collect();
        let res = SequencingCheck::default()
            .check(&r, false, "r", &read, &[])
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        let d = &res.differences[0];
        assert_eq!(d.kind, DifferenceType::Mismatch);
        assert_eq!((d.reference_position, d.read_position), (18, 8));
        assert_eq!(d.expected, old.to_string());
        assert_eq!(d.observed, alt.to_string());
        assert!((res.identity - 29.0 / 30.0 * 100.0).abs() < 1e-9);
        // Rückwärts: derselbe Unterschied, Read-Position im ausgerichteten (reverse-komplementierten) Read
        let rev = SequencingCheck::default()
            .check(&r, false, "r", &rc(&read), &[])
            .unwrap();
        assert!(rev.reversed);
        assert_eq!(rev.differences, res.differences);
        assert_eq!((rev.reference_start, rev.reference_end), (10, 40));
    }

    #[test]
    fn einfuegung_und_loeschung() {
        let r = reference();
        let rc_chars = chars(&r);
        // Einfügung zwischen Referenzbasen 24 und 25 (Read-Index 15): Base, die keiner der Nachbarn ist
        let ins = other_base(&[rc_chars[24], rc_chars[25]]);
        let read = format!("{}{}{}", &r[10..25], ins, &r[25..40]);
        let res = SequencingCheck::default()
            .check(&r, false, "ins", &read, &[])
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        let d = &res.differences[0];
        assert_eq!(d.kind, DifferenceType::Insertion);
        assert_eq!((d.reference_position, d.read_position), (25, 15));
        assert_eq!(
            (d.expected.as_str(), d.observed.as_str()),
            ("", ins.to_string().as_str())
        );
        assert!((res.identity - 30.0 / 31.0 * 100.0).abs() < 1e-9);
        // Löschung: eine Referenzbase, die sich von ihren Nachbarn unterscheidet
        let p = (30..60)
            .find(|&p| rc_chars[p] != rc_chars[p - 1] && rc_chars[p] != rc_chars[p + 1])
            .unwrap();
        let read = format!("{}{}", &r[20..p], &r[p + 1..p + 20]);
        let res = SequencingCheck::default()
            .check(&r, false, "del", &read, &[])
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        let d = &res.differences[0];
        assert_eq!(d.kind, DifferenceType::Deletion);
        assert_eq!((d.reference_position, d.read_position), (p, p - 20));
        assert_eq!(
            (d.expected.clone(), d.observed.clone()),
            (rc_chars[p].to_string(), String::new())
        );
        assert_eq!(d.quality, 40);
    }

    #[test]
    fn niedrige_qualitaet() {
        let r = reference();
        let read = r[10..40].to_string();
        let mut q = vec![40u8; 30];
        q[3] = 10;
        let res = SequencingCheck::default()
            .check(&r, false, "q", &read, &q)
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        let d = &res.differences[0];
        assert_eq!(d.kind, DifferenceType::LowQuality);
        assert_eq!(
            (d.reference_position, d.read_position, d.quality),
            (13, 3, 10)
        );
        assert_eq!(d.observed, r[13..14].to_string());
        assert_eq!(d.expected, r[13..14].to_string());
        // eigene Schwelle: 10 ist nicht „unter 10“
        assert!(SequencingCheck::new(10)
            .check(&r, false, "q", &read, &q)
            .unwrap()
            .differences
            .is_empty());
        // rückwärts: Qualitäten werden mitgedreht
        let mut qr = vec![40u8; 30];
        qr[29 - 3] = 10;
        let rev = SequencingCheck::default()
            .check(&r, false, "q", &rc(&read), &qr)
            .unwrap();
        assert!(rev.reversed);
        assert_eq!(rev.differences, res.differences);
    }

    #[test]
    fn austausch_mit_niedriger_qualitaet_gibt_zwei_eintraege() {
        let r = reference();
        let mut read = chars(&r[10..40]);
        read[8] = other_base(&[read[8]]);
        let read: String = read.into_iter().collect();
        let mut q = vec![40u8; 30];
        q[8] = 5;
        let res = SequencingCheck::default()
            .check(&r, false, "m", &read, &q)
            .unwrap();
        let kinds: Vec<DifferenceType> = res.differences.iter().map(|d| d.kind).collect();
        assert_eq!(
            kinds,
            vec![DifferenceType::Mismatch, DifferenceType::LowQuality]
        );
        assert!(res
            .differences
            .iter()
            .all(|d| d.reference_position == 18 && d.read_position == 8));
    }

    #[test]
    fn zirkulaere_referenz() {
        let r = seq(60, 5);
        let doubled = format!("{}{}", r, r);
        let read = doubled[50..80].to_string();
        let res = SequencingCheck::default()
            .check(&r, true, "c", &read, &[])
            .unwrap();
        assert!(!res.reversed);
        assert_eq!((res.reference_start, res.reference_end), (50, 20));
        assert!(res.differences.is_empty());
        assert!((res.identity - 100.0).abs() < 1e-9);
        // Unterschied hinter dem Nullpunkt: Position modulo Länge
        let mut v = chars(&read);
        v[15] = other_base(&[v[15]]);
        let read: String = v.into_iter().collect();
        let res = SequencingCheck::default()
            .check(&r, true, "c", &read, &[])
            .unwrap();
        assert_eq!(res.differences.len(), 1);
        assert_eq!(res.differences[0].reference_position, (50 + 15) % 60);
        assert_eq!(res.differences[0].read_position, 15);
    }

    #[test]
    fn fehler() {
        let r = reference();
        let c = SequencingCheck::default();
        assert!(matches!(
            c.check("", false, "x", "ACGT", &[]),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            c.check(&r, false, "x", "", &[]),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            c.check(&r, false, "x", "ACGTACGT", &[30, 30]),
            Err(Error::InvalidInput(_))
        ));
        assert_eq!(SequencingCheck::default().min_quality, 20);
    }
}
