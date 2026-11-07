//! Read ABIF files (Sanger sequencing, `.ab1`): bases, qualities, peak positions and traces.
//!
//! Layout (all numbers big-endian): bytes 0-3 `ABIF`, bytes 4-5 version. From byte 6 comes the directory entry of the directory itself
//! (28 bytes). Each entry: name (4 bytes text), number (i32), element type (i16), element size (i16), element count (i32),
//! data size (i32), data offset (i32), data handle (i32). If the data size is at most 4, the data itself is in the four bytes
//! of the data offset (left-aligned); otherwise the data offset is the start of the data in the file. For the directory entry the data offset points
//! to the list of entries (element-count entries of 28 bytes each).

use crate::error::{Error, Result};

/// An entry of the directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub number: i32,
    pub element_type: i16,
    pub element_size: i16,
    pub element_count: i32,
    pub data_size: i32,
    pub data_offset: i32,
}

/// Content of a sequencing trace.
#[derive(Clone, Debug, PartialEq)]
pub struct Chromatogram {
    /// Bases (tag `PBAS`, number 1; if number 1 is missing, number 2), uppercase.
    pub bases: String,
    /// Qualities (tag `PCON`, number 1, otherwise 2), one byte per base; empty if the tag is missing.
    pub quality: Vec<u8>,
    /// Peak positions in sample points (tag `PLOC`, number 1, otherwise 2), read as i16 values and as non-negative numbers; empty if missing.
    pub peak_positions: Vec<usize>,
    /// Traces per color channel: letter (from tag `FWO_`, number 1, four characters, e.g. `GATC`) and sample values from `DATA9`, `DATA10`,
    /// `DATA11`, `DATA12` in this order. If `FWO_` is missing, `GATC` applies. Missing `DATA` tags omit the channel.
    pub channels: Vec<(char, Vec<i16>)>,
}

fn be_i16(bytes: &[u8], at: usize) -> Option<i16> {
    let b = bytes.get(at..at.checked_add(2)?)?;
    Some(i16::from_be_bytes([*b.first()?, *b.get(1)?]))
}

fn be_i32(bytes: &[u8], at: usize) -> Option<i32> {
    let b = bytes.get(at..at.checked_add(4)?)?;
    Some(i32::from_be_bytes([
        *b.first()?,
        *b.get(1)?,
        *b.get(2)?,
        *b.get(3)?,
    ]))
}

/// Reads the directory entries. `Error::InvalidInput`: fewer than 34 bytes, wrong signature (not `ABIF`), directory or
/// an entry lies outside the file, negative numbers for element count or data offset.
pub fn read_directory(bytes: &[u8]) -> Result<Vec<DirEntry>> {
    if bytes.len() < 34 {
        return Err(Error::InvalidInput(
            "The file is too short for ABIF".to_string(),
        ));
    }
    if bytes.get(0..4) != Some(b"ABIF".as_slice()) {
        return Err(Error::InvalidInput(
            "Not an ABIF file (signature missing)".to_string(),
        ));
    }
    let bad = || Error::InvalidInput("The ABIF directory is corrupt".to_string());
    let count = be_i32(bytes, 18).ok_or_else(bad)?;
    let offset = be_i32(bytes, 26).ok_or_else(bad)?;
    if count < 0 || offset < 0 {
        return Err(bad());
    }
    let (count, offset) = (count as usize, offset as usize);
    let end = count
        .checked_mul(28)
        .and_then(|n| n.checked_add(offset))
        .ok_or_else(bad)?;
    if end > bytes.len() {
        return Err(bad());
    }
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let at = offset + i * 28;
        let name = String::from_utf8_lossy(bytes.get(at..at + 4).ok_or_else(bad)?).into_owned();
        entries.push(DirEntry {
            name,
            number: be_i32(bytes, at + 4).ok_or_else(bad)?,
            element_type: be_i16(bytes, at + 8).ok_or_else(bad)?,
            element_size: be_i16(bytes, at + 10).ok_or_else(bad)?,
            element_count: be_i32(bytes, at + 12).ok_or_else(bad)?,
            data_size: be_i32(bytes, at + 16).ok_or_else(bad)?,
            data_offset: be_i32(bytes, at + 20).ok_or_else(bad)?,
        });
    }
    Ok(entries)
}

/// The data of an entry (bytes): for `data_size <= 4` the first `data_size` bytes of the data offset field (bytes 20-23 of the entry
/// are stored as `data_offset` in `DirEntry`; for the inline data its four bytes are meant: split `data_offset` as a big-endian i32 into
/// bytes), otherwise `bytes[data_offset .. data_offset + data_size]`. If the range lies outside the file or
/// `data_size` is negative, `Error::InvalidInput`.
pub fn entry_data(bytes: &[u8], entry: &DirEntry) -> Result<Vec<u8>> {
    if entry.data_size < 0 {
        return Err(Error::InvalidInput(format!(
            "Entry {} has a negative data size",
            entry.name
        )));
    }
    let size = entry.data_size as usize;
    if size <= 4 {
        let inline = entry.data_offset.to_be_bytes();
        return Ok(inline.iter().take(size).copied().collect());
    }
    let start = usize::try_from(entry.data_offset).map_err(|_| {
        Error::InvalidInput(format!(
            "Entry {} has an invalid data location",
            entry.name
        ))
    })?;
    bytes
        .get(start..start.saturating_add(size))
        .map(<[u8]>::to_vec)
        .ok_or_else(|| {
            Error::InvalidInput(format!(
                "The data of {} lie outside the file",
                entry.name
            ))
        })
}

/// Reads the trace. `Error::InvalidInput` if `PBAS` (number 1 or 2) is missing or the file is malformed (see `read_directory`).
pub fn parse_abif(bytes: &[u8]) -> Result<Chromatogram> {
    let dir = read_directory(bytes)?;
    let find = |name: &str, numbers: &[i32]| -> Option<&DirEntry> {
        numbers
            .iter()
            .find_map(|n| dir.iter().find(|e| e.name == name && e.number == *n))
    };
    let base_entry = find("PBAS", &[1, 2])
        .ok_or_else(|| Error::InvalidInput("The file contains no bases (PBAS)".to_string()))?;
    let bases = String::from_utf8_lossy(&entry_data(bytes, base_entry)?).to_ascii_uppercase();
    let quality = match find("PCON", &[1, 2]) {
        Some(e) => entry_data(bytes, e)?,
        None => Vec::new(),
    };
    let peak_positions = match find("PLOC", &[1, 2]) {
        Some(e) => entry_data(bytes, e)?
            .chunks(2)
            .filter_map(|c| match c {
                [a, b] => Some(i16::from_be_bytes([*a, *b]).max(0) as usize),
                _ => None,
            })
            .collect(),
        None => Vec::new(),
    };
    let order: Vec<char> = match find("FWO_", &[1]) {
        Some(e) => String::from_utf8_lossy(&entry_data(bytes, e)?)
            .chars()
            .collect(),
        None => "GATC".chars().collect(),
    };
    let mut channels = Vec::new();
    for (k, number) in (9..=12).enumerate() {
        if let Some(e) = find("DATA", &[number]) {
            let values = entry_data(bytes, e)?
                .chunks(2)
                .filter_map(|c| match c {
                    [a, b] => Some(i16::from_be_bytes([*a, *b])),
                    _ => None,
                })
                .collect();
            channels.push((order.get(k).copied().unwrap_or('N'), values));
        }
    }
    Ok(Chromatogram {
        bases,
        quality,
        peak_positions,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an ABIF file: header (6 bytes), directory entry (28 bytes), data from byte 128, then the entry list.
    fn build(entries: &[(&str, i32, i16, i16, Vec<u8>)]) -> Vec<u8> {
        let mut data_area: Vec<u8> = Vec::new();
        let mut rows: Vec<Vec<u8>> = Vec::new();
        for (name, number, etype, esize, data) in entries {
            let mut row = Vec::new();
            row.extend_from_slice(name.as_bytes());
            row.extend_from_slice(&number.to_be_bytes());
            row.extend_from_slice(&etype.to_be_bytes());
            row.extend_from_slice(&esize.to_be_bytes());
            let count = if *esize > 0 {
                data.len() as i32 / *esize as i32
            } else {
                0
            };
            row.extend_from_slice(&count.to_be_bytes());
            row.extend_from_slice(&(data.len() as i32).to_be_bytes());
            if data.len() <= 4 {
                let mut inline = [0u8; 4];
                inline[..data.len()].copy_from_slice(data);
                row.extend_from_slice(&inline);
            } else {
                row.extend_from_slice(&((128 + data_area.len()) as i32).to_be_bytes());
                data_area.extend_from_slice(data);
            }
            row.extend_from_slice(&0i32.to_be_bytes());
            rows.push(row);
        }
        let dir_offset = 128 + data_area.len();
        let mut file = vec![0u8; 128];
        file[..4].copy_from_slice(b"ABIF");
        file[4..6].copy_from_slice(&101i16.to_be_bytes());
        // Directory entry of the directory
        file[6..10].copy_from_slice(b"tdir");
        file[10..14].copy_from_slice(&1i32.to_be_bytes());
        file[14..16].copy_from_slice(&1023i16.to_be_bytes());
        file[16..18].copy_from_slice(&28i16.to_be_bytes());
        file[18..22].copy_from_slice(&(rows.len() as i32).to_be_bytes());
        file[22..26].copy_from_slice(&((rows.len() * 28) as i32).to_be_bytes());
        file[26..30].copy_from_slice(&(dir_offset as i32).to_be_bytes());
        file.extend_from_slice(&data_area);
        for r in rows {
            file.extend_from_slice(&r);
        }
        file
    }

    fn shorts(v: &[i16]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_be_bytes()).collect()
    }

    fn beispiel() -> Vec<u8> {
        build(&[
            ("PBAS", 1, 2, 1, b"acgtn".to_vec()),
            ("PCON", 1, 1, 1, vec![40, 35, 30, 20, 5]),
            ("PLOC", 1, 4, 2, shorts(&[10, 20, 30, 40, 50])),
            ("FWO_", 1, 2, 1, b"ACGT".to_vec()),
            ("DATA", 9, 4, 2, shorts(&[1, 2, 3])),
            ("DATA", 10, 4, 2, shorts(&[4, 5, 6])),
            ("DATA", 11, 4, 2, shorts(&[7, 8, 9])),
            ("DATA", 12, 4, 2, shorts(&[-1, -2, -3])),
        ])
    }

    #[test]
    fn zahlen_lesen() {
        assert_eq!(be_i16(&[0x01, 0x02, 0x03], 0), Some(0x0102));
        assert_eq!(be_i16(&[0xFF, 0xFE], 0), Some(-2));
        assert_eq!(be_i16(&[1, 2], 1), None);
        assert_eq!(be_i32(&[0, 0, 1, 0, 9], 0), Some(256));
        assert_eq!(be_i32(&[0, 0, 1, 0, 9], 1), Some(0x0001_0009));
        assert_eq!(be_i32(&[0, 0, 1], 0), None);
    }

    #[test]
    fn verzeichnis() {
        let f = beispiel();
        let d = read_directory(&f).unwrap();
        assert_eq!(d.len(), 8);
        assert_eq!(d[0].name, "PBAS");
        assert_eq!((d[0].number, d[0].element_type, d[0].data_size), (1, 2, 5));
        assert_eq!(d[4].name, "DATA");
        assert_eq!(d[4].number, 9);
        assert_eq!(entry_data(&f, &d[0]).unwrap(), b"acgtn".to_vec());
        // short entry is stored inline
        assert_eq!(entry_data(&f, &d[3]).unwrap(), b"ACGT".to_vec());
    }

    #[test]
    fn spur_lesen() {
        let c = parse_abif(&beispiel()).unwrap();
        assert_eq!(c.bases, "ACGTN");
        assert_eq!(c.quality, vec![40, 35, 30, 20, 5]);
        assert_eq!(c.peak_positions, vec![10, 20, 30, 40, 50]);
        assert_eq!(c.channels.len(), 4);
        assert_eq!(c.channels[0], ('A', vec![1, 2, 3]));
        assert_eq!(c.channels[1], ('C', vec![4, 5, 6]));
        assert_eq!(c.channels[3], ('T', vec![-1, -2, -3]));
    }

    #[test]
    fn optionale_tags_und_nummer_zwei() {
        let f = build(&[("PBAS", 2, 2, 1, b"GGAC".to_vec())]);
        let c = parse_abif(&f).unwrap();
        assert_eq!(c.bases, "GGAC");
        assert!(c.quality.is_empty() && c.peak_positions.is_empty() && c.channels.is_empty());
        // FWO_ missing: default GATC
        let g = build(&[
            ("PBAS", 1, 2, 1, b"A".to_vec()),
            ("DATA", 9, 4, 2, shorts(&[5])),
            ("DATA", 12, 4, 2, shorts(&[6])),
        ]);
        let c = parse_abif(&g).unwrap();
        assert_eq!(c.channels, vec![('G', vec![5]), ('C', vec![6])]);
    }

    #[test]
    fn fehler() {
        assert!(matches!(parse_abif(&[]), Err(Error::InvalidInput(_))));
        assert!(matches!(
            parse_abif(&[0u8; 100]),
            Err(Error::InvalidInput(_))
        ));
        let mut f = beispiel();
        f[0] = b'X';
        assert!(matches!(parse_abif(&f), Err(Error::InvalidInput(_))));
        // Directory points past the end of the file
        let mut g = beispiel();
        g[26..30].copy_from_slice(&100000i32.to_be_bytes());
        assert!(matches!(parse_abif(&g), Err(Error::InvalidInput(_))));
        // without bases
        let h = build(&[("PCON", 1, 1, 1, vec![1, 2, 3, 4, 5])]);
        assert!(matches!(parse_abif(&h), Err(Error::InvalidInput(_))));
        // truncated file
        let kurz = beispiel()[..140].to_vec();
        assert!(matches!(parse_abif(&kurz), Err(Error::InvalidInput(_))));
    }
}
