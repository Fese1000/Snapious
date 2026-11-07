//! BAM-Dateien lesen. BAM besteht aus aneinandergehängten gzip-Blöcken (BGZF); das Entpacken übernimmt das Programm `gzip -dc`,
//! das Zerlegen der entpackten Bytes ist reine Rechnung (`parse_bam`) und lässt sich ohne Datei testen.

use std::path::Path;

use crate::error::{Error, Result};
use crate::io::sam::SamRecord;

/// Ein gelesenes BAM.
#[derive(Clone, Debug, PartialEq)]
pub struct BamFile {
    /// Kopftext (SAM-Kopfzeilen).
    pub header_text: String,
    /// Referenzen `(Name, Länge)` in Dateireihenfolge.
    pub references: Vec<(String, u32)>,
    /// Alle Datensätze als `SamRecord` (gleiche Konventionen wie bei `io::sam`: `position` 1-basiert, 0 = nicht ausgerichtet).
    pub records: Vec<SamRecord>,
}

/// Zerlegt die entpackten BAM-Bytes (alle Zahlen little-endian).
/// Kopf: Magic `BAM\x01`; `l_text` (i32) und der Kopftext (UTF-8, ein NUL-Zeichen am Ende wird abgeschnitten); `n_ref` (i32); je Referenz
/// `l_name` (i32), der Name (`l_name` Bytes einschließlich NUL, das NUL wird entfernt) und `l_ref` (i32).
/// Datensatz: `block_size` (i32) und danach genau `block_size` Bytes: `refID` (i32), `pos` (i32, 0-basiert), `l_read_name` (u8), `mapq` (u8),
/// `bin` (u16), `n_cigar_op` (u16), `flag` (u16), `l_seq` (i32), `next_refID` (i32), `next_pos` (i32), `tlen` (i32), `read_name`
/// (`l_read_name` Bytes einschließlich NUL), `cigar` (`n_cigar_op` Werte u32: Länge = `x >> 4`, Operation = `"MIDNSHP=X"` an Stelle `x & 0xf`),
/// `seq` (`(l_seq + 1) / 2` Bytes, je 4 Bit, hohes Halbbyte zuerst, Zeichen `"=ACMGRSVTWYHKDBN"`), `qual` (`l_seq` Bytes Phred ohne Offset;
/// `0xFF` am ersten Byte = Qualität fehlt). Rest des Blocks (optionale Felder) wird übersprungen.
/// Abbildung auf `SamRecord`: `reference` = Name zu `refID` oder `*` bei −1 (oder unbekannter Nummer); `position = pos + 1` (bei `pos == −1` → 0);
/// `cigar` als Text (`4M2D`), `*` ohne Operationen; `mate_reference` = `=`, wenn `next_refID == refID` und nicht −1, sonst Name oder `*`;
/// `mate_position = next_pos + 1` (bei −1 → 0); `template_length = tlen`; `sequence` leer → `*`; `quality` = Zeichen `Phred + 33`, `*` wenn fehlend oder `l_seq == 0`.
/// Fehler `Error::InvalidInput`: falsches Magic, abgeschnittene Daten (Zugriff hinter das Ende), unbekannte Operation.
pub fn parse_bam(data: &[u8]) -> Result<BamFile> {
    let bad = |m: &str| Error::InvalidInput(m.to_string());
    let mut cur = Cursor { data, pos: 0 };
    if cur.take(4)? != b"BAM\x01" {
        return Err(bad("Not a BAM file (magic missing)"));
    }
    let l_text = cur.i32()? as usize;
    let mut text_bytes = cur.take(l_text)?.to_vec();
    while text_bytes.last() == Some(&0) {
        text_bytes.pop();
    }
    let header_text = String::from_utf8_lossy(&text_bytes).to_string();
    let n_ref = cur.i32()?;
    let mut references = Vec::new();
    for _ in 0..n_ref.max(0) {
        let l_name = cur.i32()? as usize;
        let mut name = cur.take(l_name)?.to_vec();
        while name.last() == Some(&0) {
            name.pop();
        }
        let l_ref = cur.i32()?;
        references.push((
            String::from_utf8_lossy(&name).to_string(),
            l_ref.max(0) as u32,
        ));
    }
    let ref_name = |id: i32| -> String {
        usize::try_from(id)
            .ok()
            .and_then(|i| references.get(i))
            .map_or("*".to_string(), |r| r.0.clone())
    };
    const CODES: &[u8; 16] = b"=ACMGRSVTWYHKDBN";
    const OPS: &[u8; 9] = b"MIDNSHP=X";
    let mut records = Vec::new();
    while cur.pos < data.len() {
        let block_size = cur.i32()?;
        let block = cur.take(block_size.max(0) as usize)?;
        let mut b = Cursor {
            data: block,
            pos: 0,
        };
        let ref_id = b.i32()?;
        let pos = b.i32()?;
        let l_read_name = b.u8()? as usize;
        let mapq = b.u8()?;
        let _bin = b.u16()?;
        let n_cigar = b.u16()? as usize;
        let flag = b.u16()?;
        let l_seq = b.i32()?.max(0) as usize;
        let next_ref = b.i32()?;
        let next_pos = b.i32()?;
        let tlen = b.i32()?;
        let mut name = b.take(l_read_name)?.to_vec();
        while name.last() == Some(&0) {
            name.pop();
        }
        let mut cigar = String::new();
        for _ in 0..n_cigar {
            let x = b.u32()?;
            let op = *OPS
                .get((x & 0xf) as usize)
                .ok_or_else(|| bad("Unknown CIGAR operation"))?;
            cigar.push_str(&format!("{}{}", x >> 4, op as char));
        }
        if cigar.is_empty() {
            cigar.push('*');
        }
        let packed = b.take(l_seq.div_ceil(2))?;
        let mut sequence = String::new();
        for i in 0..l_seq {
            let byte = packed[i / 2];
            let code = if i % 2 == 0 { byte >> 4 } else { byte & 0xf };
            sequence.push(CODES[code as usize] as char);
        }
        let qual = b.take(l_seq)?;
        let quality = if l_seq == 0 || qual.first() == Some(&0xFF) {
            "*".to_string()
        } else {
            qual.iter()
                .map(|q| (q.saturating_add(33)) as char)
                .collect()
        };
        if sequence.is_empty() {
            sequence.push('*');
        }
        let mate_reference = if next_ref >= 0 && next_ref == ref_id {
            "=".to_string()
        } else {
            ref_name(next_ref)
        };
        records.push(SamRecord {
            read_name: String::from_utf8_lossy(&name).to_string(),
            flag,
            reference: ref_name(ref_id),
            position: (pos + 1).max(0) as usize,
            mapq,
            cigar,
            mate_reference,
            mate_position: (next_pos + 1).max(0) as usize,
            template_length: tlen as i64,
            sequence,
            quality,
        });
    }
    Ok(BamFile {
        header_text,
        references,
        records,
    })
}

/// Entpackt `path` mit `gzip -dc` und ruft `parse_bam` auf. Fehler beim Start oder Exitcode ≠ 0 → `Error::Io` mit dem Text von `gzip`.
pub fn read_bam(path: &Path) -> Result<BamFile> {
    let out = std::process::Command::new("gzip")
        .arg("-dc")
        .arg(path)
        .output()
        .map_err(|e| Error::Io(format!("gzip could not be started: {}", e)))?;
    if !out.status.success() {
        return Err(Error::Io(format!(
            "gzip: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    parse_bam(&out.stdout)
}

/// Lesezeiger über einen Bytebereich mit Längenprüfung.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|e| *e <= self.data.len())
            .ok_or_else(|| Error::InvalidInput("BAM data is truncated".to_string()))?;
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le32(v: i32) -> Vec<u8> {
        v.to_le_bytes().to_vec()
    }

    /// Baut einen BAM-Datensatz für den Test.
    #[allow(clippy::too_many_arguments)]
    fn record(
        ref_id: i32,
        pos: i32,
        mapq: u8,
        flag: u16,
        name: &str,
        cigar: &[(u32, u8)],
        seq: &str,
        qual: Option<&[u8]>,
        next_ref: i32,
        next_pos: i32,
        tlen: i32,
    ) -> Vec<u8> {
        const CODES: &str = "=ACMGRSVTWYHKDBN";
        let mut b = Vec::new();
        b.extend(le32(ref_id));
        b.extend(le32(pos));
        b.push(name.len() as u8 + 1);
        b.push(mapq);
        b.extend(0u16.to_le_bytes());
        b.extend((cigar.len() as u16).to_le_bytes());
        b.extend(flag.to_le_bytes());
        b.extend(le32(seq.len() as i32));
        b.extend(le32(next_ref));
        b.extend(le32(next_pos));
        b.extend(le32(tlen));
        b.extend(name.as_bytes());
        b.push(0);
        for (len, op) in cigar {
            b.extend(((len << 4) | *op as u32).to_le_bytes());
        }
        let codes: Vec<u8> = seq.chars().map(|c| CODES.find(c).unwrap() as u8).collect();
        for pair in codes.chunks(2) {
            b.push((pair[0] << 4) | pair.get(1).copied().unwrap_or(0));
        }
        match qual {
            Some(q) => b.extend_from_slice(q),
            None => b.extend(std::iter::repeat_n(0xFFu8, seq.len())),
        }
        // optionales Feld, das übersprungen werden muss: NM:C:1
        b.extend(b"NMC\x01");
        let mut out = le32(b.len() as i32);
        out.extend(b);
        out
    }

    fn header() -> Vec<u8> {
        let text = "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:1000\n";
        let mut b = b"BAM\x01".to_vec();
        b.extend(le32(text.len() as i32 + 1));
        b.extend(text.as_bytes());
        b.push(0);
        b.extend(le32(2));
        for (name, len) in [("chr1", 1000), ("chr2", 500)] {
            b.extend(le32(name.len() as i32 + 1));
            b.extend(name.as_bytes());
            b.push(0);
            b.extend(le32(len));
        }
        b
    }

    fn sample() -> Vec<u8> {
        let mut b = header();
        // 'M' = 0, 'I' = 1, 'D' = 2, 'N' = 3, 'S' = 4
        b.extend(record(
            0,
            4,
            60,
            0,
            "r1",
            &[(4, 0)],
            "ACGT",
            Some(&[40, 40, 30, 20]),
            -1,
            -1,
            0,
        ));
        b.extend(record(
            1,
            9,
            20,
            16,
            "r2",
            &[(3, 4), (4, 0), (2, 2), (1, 0)],
            "TTACGTA",
            None,
            1,
            99,
            -150,
        ));
        b.extend(record(
            -1,
            -1,
            0,
            4,
            "r3",
            &[],
            "GN",
            Some(&[10, 11]),
            -1,
            -1,
            0,
        ));
        b
    }

    #[test]
    fn kopf_und_datensaetze() {
        let f = parse_bam(&sample()).unwrap();
        assert!(f.header_text.starts_with("@HD\tVN:1.6"));
        assert!(!f.header_text.contains('\0'));
        assert_eq!(
            f.references,
            vec![("chr1".to_string(), 1000), ("chr2".to_string(), 500)]
        );
        assert_eq!(f.records.len(), 3);
        let r1 = &f.records[0];
        assert_eq!(r1.read_name, "r1");
        assert_eq!(r1.flag, 0);
        assert_eq!(r1.reference, "chr1");
        assert_eq!(r1.position, 5);
        assert_eq!(r1.mapq, 60);
        assert_eq!(r1.cigar, "4M");
        assert_eq!(r1.mate_reference, "*");
        assert_eq!(r1.mate_position, 0);
        assert_eq!(r1.sequence, "ACGT");
        assert_eq!(r1.quality, "II?5");
        assert_eq!(r1.position0(), Some(4));
        assert_eq!(r1.reference_length().unwrap(), 4);
    }

    #[test]
    fn cigar_mate_und_fehlende_qualitaet() {
        let f = parse_bam(&sample()).unwrap();
        let r2 = &f.records[1];
        assert_eq!(r2.reference, "chr2");
        assert_eq!(r2.position, 10);
        assert_eq!(r2.cigar, "3S4M2D1M");
        assert_eq!(r2.mate_reference, "=");
        assert_eq!(r2.mate_position, 100);
        assert_eq!(r2.template_length, -150);
        assert_eq!(r2.sequence, "TTACGTA");
        assert_eq!(r2.quality, "*");
        assert!(r2.is_reverse());
        assert_eq!(r2.reference_length().unwrap(), 7);
        let r3 = &f.records[2];
        assert_eq!(r3.reference, "*");
        assert_eq!(r3.position, 0);
        assert_eq!(r3.cigar, "*");
        assert_eq!(r3.sequence, "GN");
        assert_eq!(r3.quality, "+,");
        assert!(r3.is_unmapped());
    }

    #[test]
    fn fehler() {
        assert!(matches!(
            parse_bam(b"XXXX\x00\x00\x00\x00"),
            Err(Error::InvalidInput(_))
        ));
        let mut cut = sample();
        cut.truncate(cut.len() - 5);
        assert!(matches!(parse_bam(&cut), Err(Error::InvalidInput(_))));
        assert!(parse_bam(&[]).is_err());
        let only_header = parse_bam(&header()).unwrap();
        assert!(only_header.records.is_empty());
        // gzip-Datei: nur wenn gzip vorhanden ist
        let dir = std::env::temp_dir().join(format!("snapious-bam-{}", crate::util::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let raw = dir.join("x.bam.raw");
        std::fs::write(&raw, sample()).unwrap();
        if std::process::Command::new("gzip")
            .arg("-k")
            .arg(&raw)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            let f = read_bam(&dir.join("x.bam.raw.gz")).unwrap();
            assert_eq!(f.records.len(), 3);
        }
        assert!(read_bam(&dir.join("fehlt.bam")).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
