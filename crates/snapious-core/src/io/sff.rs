//! Read SFF files (454 sequencing, Standard Flowgram Format).

use crate::error::{Error, Result};

/// A read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SffRead {
    pub name: String,
    pub bases: String,
    pub qualities: Vec<u8>,
    /// Quality clip left/right, 1-based as in the file (0 = no clip).
    pub clip_left: u16,
    pub clip_right: u16,
    /// Flowgram values (one `u16` per flow).
    pub flowgram: Vec<u16>,
}

/// Content of an SFF file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SffFile {
    pub flow_chars: String,
    pub key: String,
    pub reads: Vec<SffRead>,
}

/// Reads SFF (all numbers big-endian). General header: magic `.sff` (4), version (4, bytes `00 00 00 01`), `index_offset` (`u64`),
/// `index_length` (`u32`), `number_of_reads` (`u32`), `header_length` (`u16`), `key_length` (`u16`), `number_of_flows` (`u16`),
/// `flowgram_format` (`u8`), then `flow_chars` (`number_of_flows` characters) and `key` (`key_length` characters); the header spans
/// `header_length` bytes in total (the rest is padding), and the first read starts there.
/// Per read: read header with `read_header_length` (`u16`), `name_length` (`u16`), `number_of_bases` (`u32`), `clip_qual_left` (`u16`),
/// `clip_qual_right` (`u16`), `clip_adapter_left` (`u16`), `clip_adapter_right` (`u16`), then the name (`name_length` characters); the read header
/// spans `read_header_length` bytes. Then the data: `flowgram` (`number_of_flows` x `u16`), `flow_index_per_base` (1 byte per base,
/// ignore), `bases` (1 character per base), `quality_scores` (1 byte per base), padded to a multiple of 8 bytes (counted from
/// the start of the read's data, i.e. the length `flows*2 + 3*bases` rounded up to a multiple of 8). The next read follows directly after.
/// `Error::InvalidInput`: wrong magic, wrong version, data extending past the end of the file (no panic on arbitrary
/// input; use `get` and `checked_add`), or fewer reads than `number_of_reads`.
/// Small read cursor with bounds checking.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| Error::InvalidInput("SFF: length too large".to_string()))?;
        let out = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| Error::InvalidInput("SFF: file too short".to_string()))?;
        self.pos = end;
        Ok(out)
    }

    fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

pub fn parse_sff(data: &[u8]) -> Result<SffFile> {
    let mut c = Cursor { data, pos: 0 };
    if c.take(4)? != b".sff" {
        return Err(Error::InvalidInput("Not an SFF file".to_string()));
    }
    if c.take(4)? != [0, 0, 0, 1] {
        return Err(Error::InvalidInput(
            "Only SFF version 1 is supported".to_string(),
        ));
    }
    c.take(8)?; // index_offset
    c.take(4)?; // index_length
    let reads_total = c.u32()? as usize;
    let header_length = c.u16()? as usize;
    let key_length = c.u16()? as usize;
    let flows = c.u16()? as usize;
    c.take(1)?; // flowgram_format
    let flow_chars = String::from_utf8_lossy(c.take(flows)?).to_string();
    let key = String::from_utf8_lossy(c.take(key_length)?).to_string();
    if header_length < c.pos {
        return Err(Error::InvalidInput("SFF: header length too small".to_string()));
    }
    c.pos = header_length;
    if c.pos > data.len() {
        return Err(Error::InvalidInput("SFF: file too short".to_string()));
    }
    let mut reads = Vec::new();
    for _ in 0..reads_total {
        let start = c.pos;
        let read_header_length = c.u16()? as usize;
        let name_length = c.u16()? as usize;
        let n_bases = c.u32()? as usize;
        let clip_left = c.u16()?;
        let clip_right = c.u16()?;
        c.u16()?; // clip_adapter_left
        c.u16()?; // clip_adapter_right
        let name = String::from_utf8_lossy(c.take(name_length)?).to_string();
        c.pos = start
            .checked_add(read_header_length)
            .ok_or_else(|| Error::InvalidInput("SFF: read header too large".to_string()))?;
        let data_start = c.pos;
        let mut flowgram = Vec::with_capacity(flows);
        for _ in 0..flows {
            flowgram.push(c.u16()?);
        }
        c.take(n_bases)?; // flow_index_per_base
        let bases = String::from_utf8_lossy(c.take(n_bases)?).to_string();
        let qualities = c.take(n_bases)?.to_vec();
        let used = c.pos - data_start;
        let padded = used.div_ceil(8) * 8;
        // Skip padding; if it is missing at the end of the file, that is not an error
        c.pos = (data_start + padded).min(data.len());
        reads.push(SffRead {
            name,
            bases,
            qualities,
            clip_left,
            clip_right,
            flowgram,
        });
    }
    Ok(SffFile {
        flow_chars,
        key,
        reads,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad8(v: &mut Vec<u8>) {
        while !v.len().is_multiple_of(8) {
            v.push(0);
        }
    }

    /// An SFF file with four flows `TACG`, key `TCAG` and one read `r01` with the bases `ACG`.
    fn beispiel() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(b".sff");
        d.extend_from_slice(&[0, 0, 0, 1]);
        d.extend_from_slice(&0u64.to_be_bytes()); // index_offset
        d.extend_from_slice(&0u32.to_be_bytes()); // index_length
        d.extend_from_slice(&1u32.to_be_bytes()); // number_of_reads
        d.extend_from_slice(&40u16.to_be_bytes()); // header_length
        d.extend_from_slice(&4u16.to_be_bytes()); // key_length
        d.extend_from_slice(&4u16.to_be_bytes()); // number_of_flows
        d.push(1); // flowgram_format
        d.extend_from_slice(b"TACG");
        d.extend_from_slice(b"TCAG");
        pad8(&mut d);
        assert_eq!(d.len(), 40);
        // Read header
        let mut h = Vec::new();
        h.extend_from_slice(&24u16.to_be_bytes());
        h.extend_from_slice(&3u16.to_be_bytes());
        h.extend_from_slice(&3u32.to_be_bytes());
        h.extend_from_slice(&1u16.to_be_bytes()); // clip_qual_left
        h.extend_from_slice(&3u16.to_be_bytes()); // clip_qual_right
        h.extend_from_slice(&0u16.to_be_bytes());
        h.extend_from_slice(&0u16.to_be_bytes());
        h.extend_from_slice(b"r01");
        pad8(&mut h);
        assert_eq!(h.len(), 24);
        d.extend_from_slice(&h);
        // Data
        let mut r = Vec::new();
        for f in [100u16, 200, 0, 100] {
            r.extend_from_slice(&f.to_be_bytes());
        }
        r.extend_from_slice(&[1, 2, 1]); // flow_index_per_base
        r.extend_from_slice(b"ACG");
        r.extend_from_slice(&[30, 31, 32]);
        pad8(&mut r);
        d.extend_from_slice(&r);
        d
    }

    #[test]
    fn ein_read() {
        let f = parse_sff(&beispiel()).unwrap();
        assert_eq!(f.flow_chars, "TACG");
        assert_eq!(f.key, "TCAG");
        assert_eq!(f.reads.len(), 1);
        let r = &f.reads[0];
        assert_eq!(r.name, "r01");
        assert_eq!(r.bases, "ACG");
        assert_eq!(r.qualities, vec![30, 31, 32]);
        assert_eq!((r.clip_left, r.clip_right), (1, 3));
        assert_eq!(r.flowgram, vec![100, 200, 0, 100]);
    }

    #[test]
    fn fehler_und_abgeschnitten() {
        assert!(parse_sff(&[]).is_err());
        let ganz = beispiel();
        for n in [10usize, 39, 45, 60, ganz.len() - 8] {
            assert!(parse_sff(&ganz[..n]).is_err(), "Länge {n}");
        }
        let mut falsch = beispiel();
        falsch[0] = b'X';
        assert!(parse_sff(&falsch).is_err());
        let mut v = beispiel();
        v[7] = 2;
        assert!(parse_sff(&v).is_err());
        let mut zwei = beispiel();
        zwei[20..24].copy_from_slice(&2u32.to_be_bytes()); // number_of_reads = 2, but only one read present
        assert!(parse_sff(&zwei).is_err());
    }
}
