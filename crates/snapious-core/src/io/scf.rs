//! SCF-Dateien (Chromatogramme, Version 3) lesen: Basen, Qualitäten, Peak-Positionen.

use crate::error::{Error, Result};

/// Gelesene Basen eines Chromatogramms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScfRead {
    pub bases: String,
    /// Qualität der gerufenen Base: der Wert aus dem Array der Wahrscheinlichkeiten, das zu dieser Base gehört (A, C, G, T).
    /// Bei anderen Zeichen (z. B. `N`) das Maximum der vier Werte.
    pub qualities: Vec<u8>,
    /// Position des Peaks im Spurverlauf je Base.
    pub peaks: Vec<u32>,
}

/// Liest die Basen-Daten einer SCF-Datei Version 3 (alle Zahlen Big-Endian, `u32`). Kopf (128 Bytes):
/// Magic `.scf` (4 Bytes), `samples`, `samples_offset`, `bases`, `bases_left_clip`, `bases_right_clip`, `bases_offset`, `comments_size`,
/// `comments_offset`, dann `version` (4 Zeichen, z. B. `3.00`), danach weitere Felder, die ignoriert werden. Ab `bases_offset` liegen
/// hintereinander, jeweils `bases` Einträge: `peak_index` (`u32` je Base), `prob_A`, `prob_C`, `prob_G`, `prob_T` (je 1 Byte je Base),
/// die Basenzeichen (1 Byte je Base), dann 3 Spare-Bytes je Base (ignorieren). Die Spurdaten (`samples`) werden nicht gelesen.
/// Fehler `Error::InvalidInput`: Datei kürzer als 128 Bytes, falsches Magic, Version nicht `3.00` (also `version != b"3.00"`) oder
/// Abschnitte, die über das Dateiende hinausgehen (kein Absturz bei beliebigen Eingaben; `checked_add` / `get` verwenden).
fn u32_at(data: &[u8], pos: usize) -> Result<u32> {
    let end = pos
        .checked_add(4)
        .ok_or_else(|| Error::InvalidInput("SCF: position too large".to_string()))?;
    let b = data
        .get(pos..end)
        .ok_or_else(|| Error::InvalidInput("SCF: file too short".to_string()))?;
    Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn slice_at(data: &[u8], pos: usize, len: usize) -> Result<&[u8]> {
    let end = pos
        .checked_add(len)
        .ok_or_else(|| Error::InvalidInput("SCF: section too large".to_string()))?;
    data.get(pos..end)
        .ok_or_else(|| Error::InvalidInput("SCF: section extends beyond the end of the file".to_string()))
}

pub fn parse_scf(data: &[u8]) -> Result<ScfRead> {
    if data.len() < 128 || &data[..4] != b".scf" {
        return Err(Error::InvalidInput("Not an SCF file".to_string()));
    }
    if &data[36..40] != b"3.00" {
        return Err(Error::InvalidInput(
            "Only SCF version 3.00 is supported".to_string(),
        ));
    }
    let bases = u32_at(data, 12)? as usize;
    let offset = u32_at(data, 24)? as usize;
    let peaks_len = bases
        .checked_mul(4)
        .ok_or_else(|| Error::InvalidInput("SCF: number of bases too large".to_string()))?;
    let peaks_raw = slice_at(data, offset, peaks_len)?;
    let peaks: Vec<u32> = peaks_raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_be_bytes(*c))
        .collect();
    let mut pos = offset + peaks_len; // kann nicht überlaufen: slice_at hat geprüft
    let prob_a = slice_at(data, pos, bases)?;
    pos += bases;
    let prob_c = slice_at(data, pos, bases)?;
    pos += bases;
    let prob_g = slice_at(data, pos, bases)?;
    pos += bases;
    let prob_t = slice_at(data, pos, bases)?;
    pos += bases;
    let chars = slice_at(data, pos, bases)?;
    // die Spare-Bytes (3 je Base) müssen nicht vorhanden sein, werden aber nicht gelesen
    let mut text = String::with_capacity(bases);
    let mut qualities = Vec::with_capacity(bases);
    for i in 0..bases {
        let c = chars[i].to_ascii_uppercase();
        text.push(c as char);
        let all = [prob_a[i], prob_c[i], prob_g[i], prob_t[i]];
        qualities.push(match c {
            b'A' => all[0],
            b'C' => all[1],
            b'G' => all[2],
            b'T' => all[3],
            _ => all.into_iter().max().unwrap_or(0),
        });
    }
    Ok(ScfRead {
        bases: text,
        qualities,
        peaks,
    })
}

/// Liest die Spurdaten (Messwerte) einer SCF-Datei Version 3. Gleicher Kopf wie bei `parse_scf` (Zahlen Big-Endian, `u32`): `samples` bei Byte 4,
/// `samples_offset` bei Byte 8, die Größe eines Messwertes (`sample_size`, 1 oder 2) bei Byte 40. Ab `samples_offset` folgen nacheinander die vier
/// Kanäle **A, C, G, T**, jeweils `samples` Werte der Größe `sample_size` (Big-Endian, bei Größe 1 ein Byte ohne Vorzeichen). Jeder Kanal ist
/// doppelt differenzkodiert: zweimal nacheinander (zwei Durchgänge) wird für `i = 0..samples` gerechnet `wert[i] = wert[i] + vorher; vorher = wert[i]`
/// (`vorher` beginnt je Durchgang bei 0; Rechnung mit `u16` und `wrapping_add`). Ergebnis: vier Vektoren (A, C, G, T) mit je `samples` Werten.
/// Fehler `Error::InvalidInput`: Datei kürzer als 128 Bytes, falsches Magic, Version nicht `3.00`, `sample_size` nicht 1 oder 2 oder Daten, die über das Dateiende
/// reichen (kein Absturz bei beliebigen Eingaben; `checked_add`, `checked_mul` und `get` verwenden).
/// Beispiel: gespeichert `[10, 0, 0]` ergibt `[10, 20, 30]`.
pub fn parse_scf_samples(data: &[u8]) -> Result<Vec<Vec<u16>>> {
    if data.len() < 128 || &data[..4] != b".scf" {
        return Err(Error::InvalidInput("Not an SCF file".to_string()));
    }
    if &data[36..40] != b"3.00" {
        return Err(Error::InvalidInput(
            "Only SCF version 3.00 is supported".to_string(),
        ));
    }
    let samples = u32_at(data, 4)? as usize;
    let offset = u32_at(data, 8)? as usize;
    let size = u32_at(data, 40)? as usize;
    if size != 1 && size != 2 {
        return Err(Error::InvalidInput(
            "The sample size must be 1 or 2 bytes".to_string(),
        ));
    }
    let channel_len = samples
        .checked_mul(size)
        .ok_or_else(|| Error::InvalidInput("SCF: number of samples too large".to_string()))?;
    let mut out = Vec::with_capacity(4);
    for ch in 0..4usize {
        let start = ch
            .checked_mul(channel_len)
            .and_then(|o| o.checked_add(offset))
            .ok_or_else(|| Error::InvalidInput("SCF: section too large".to_string()))?;
        let raw = slice_at(data, start, channel_len)?;
        let mut values: Vec<u16> = if size == 2 {
            raw.chunks(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect()
        } else {
            raw.iter().map(|b| *b as u16).collect()
        };
        for _ in 0..2 {
            let mut previous = 0u16;
            for v in values.iter_mut() {
                *v = v.wrapping_add(previous);
                previous = *v;
            }
        }
        out.push(values);
    }
    Ok(out)
}

#[cfg(test)]
mod samples_tests {
    use super::*;

    /// SCF mit drei Messwerten je Kanal (Größe 2), keine Basen; die gespeicherten Werte sind doppelt differenzkodiert.
    fn mit_spuren(stored: [[u16; 3]; 4]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b".scf");
        // samples, samples_offset, bases, left, right, bases_offset, comments_size, comments_offset
        for v in [3u32, 128, 0, 0, 0, 152, 0, 152] {
            d.extend_from_slice(&v.to_be_bytes());
        }
        d.extend_from_slice(b"3.00");
        d.extend_from_slice(&2u32.to_be_bytes()); // sample_size
        d.resize(128, 0);
        for ch in stored {
            for v in ch {
                d.extend_from_slice(&v.to_be_bytes());
            }
        }
        d
    }

    #[test]
    fn vier_kanaele() {
        let d = mit_spuren([[10, 0, 0], [5, 1, 1], [0, 0, 0], [1, 2, 3]]);
        let s = parse_scf_samples(&d).unwrap();
        assert_eq!(s.len(), 4);
        assert_eq!(s[0], vec![10, 20, 30]);
        assert_eq!(s[1], vec![5, 11, 18]);
        assert_eq!(s[2], vec![0, 0, 0]);
        assert_eq!(s[3], vec![1, 4, 10]);
    }

    #[test]
    fn fehler() {
        let d = mit_spuren([[0; 3]; 4]);
        assert!(parse_scf_samples(&[]).is_err());
        assert!(parse_scf_samples(&d[..140]).is_err());
        let mut falsch = d.clone();
        falsch[0] = b'X';
        assert!(parse_scf_samples(&falsch).is_err());
        let mut v2 = d.clone();
        v2[36..40].copy_from_slice(b"2.00");
        assert!(parse_scf_samples(&v2).is_err());
        let mut groesse = d;
        groesse[40..44].copy_from_slice(&3u32.to_be_bytes());
        assert!(parse_scf_samples(&groesse).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn be(v: u32) -> [u8; 4] {
        v.to_be_bytes()
    }

    /// Baut eine SCF-Datei mit drei Basen `ACG`.
    fn beispiel() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b".scf");
        for v in [0u32, 128, 3, 0, 0, 128, 0, 164] {
            d.extend_from_slice(&be(v)); // samples, samples_offset, bases, left, right, bases_offset, comments_size, comments_offset
        }
        d.extend_from_slice(b"3.00");
        d.extend_from_slice(&be(2)); // sample_size
        d.resize(128, 0);
        for p in [10u32, 20, 30] {
            d.extend_from_slice(&be(p));
        }
        d.extend_from_slice(&[40, 2, 1]); // prob_A
        d.extend_from_slice(&[1, 30, 2]); // prob_C
        d.extend_from_slice(&[1, 2, 20]); // prob_G
        d.extend_from_slice(&[1, 1, 1]); // prob_T
        d.extend_from_slice(b"ACG");
        d.extend_from_slice(&[0; 9]);
        d
    }

    #[test]
    fn drei_basen() {
        let r = parse_scf(&beispiel()).unwrap();
        assert_eq!(r.bases, "ACG");
        assert_eq!(r.qualities, vec![40, 30, 20]);
        assert_eq!(r.peaks, vec![10, 20, 30]);
    }

    #[test]
    fn andere_zeichen_und_fehler() {
        let mut d = beispiel();
        d[150] = 55; // prob_T der zweiten Base
        d[153] = b'N'; // zweite Base wird N: Maximum der vier Werte (2, 30, 2, 55)
        assert_eq!(parse_scf(&d).unwrap().qualities, vec![40, 55, 20]);
        assert_eq!(parse_scf(&d).unwrap().bases, "ANG");
        assert!(parse_scf(&[]).is_err());
        assert!(parse_scf(&beispiel()[..100]).is_err());
        assert!(parse_scf(&beispiel()[..140]).is_err());
        let mut falsch = beispiel();
        falsch[0] = b'X';
        assert!(parse_scf(&falsch).is_err());
        let mut v2 = beispiel();
        v2[36..40].copy_from_slice(b"2.00");
        assert!(parse_scf(&v2).is_err());
    }
}
