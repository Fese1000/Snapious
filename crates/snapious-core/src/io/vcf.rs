//! VCF (Variant Call Format) lesen und schreiben.

use std::path::Path;

use crate::error::{Error, Result};

/// Art der Variante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariantType {
    Snp,
    /// Mehrere aufeinanderfolgende Basen ausgetauscht.
    Mnp,
    Insertion,
    Deletion,
    Other,
}

/// Eine Variante (erste 8 Spalten).
#[derive(Clone, Debug, PartialEq)]
pub struct VariantCall {
    pub chromosome: String,
    /// 1-basiert wie in der Datei.
    pub position: usize,
    pub id: String,
    pub ref_allele: String,
    pub alt_allele: String,
    pub quality: Option<f64>,
    pub filter: String,
    pub info: String,
}

impl VariantCall {
    pub fn is_pass(&self) -> bool {
        self.filter == "PASS"
    }

    pub fn variant_type(&self) -> VariantType {
        let (r, a) = (self.ref_allele.len(), self.alt_allele.len());
        if self.alt_allele.contains(',') || self.alt_allele.starts_with('<') {
            VariantType::Other
        } else if r == 1 && a == 1 {
            VariantType::Snp
        } else if r == a {
            VariantType::Mnp
        } else if r < a {
            VariantType::Insertion
        } else {
            VariantType::Deletion
        }
    }
}

/// Liest VCF-Text. Kommentarzeilen (`#`) und leere Zeilen werden übersprungen.
pub fn parse(text: &str) -> Result<Vec<VariantCall>> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 8 {
            return Err(Error::Io(format!(
                "Zeile {}: {} Spalten gefunden, mindestens 8 erwartet",
                i + 1,
                f.len()
            )));
        }
        let position = f[1]
            .parse::<usize>()
            .map_err(|_| Error::Io(format!("Zeile {}: Ungültige Position '{}'", i + 1, f[1])))?;
        let quality = if f[5] == "." {
            None
        } else {
            Some(f[5].parse::<f64>().map_err(|_| {
                Error::Io(format!("Zeile {}: Ungültige Qualität '{}'", i + 1, f[5]))
            })?)
        };
        out.push(VariantCall {
            chromosome: f[0].to_string(),
            position,
            id: if f[2] == "." {
                String::new()
            } else {
                f[2].to_string()
            },
            ref_allele: f[3].to_ascii_uppercase(),
            alt_allele: f[4].to_ascii_uppercase(),
            quality,
            filter: f[6].to_string(),
            info: if f[7] == "." {
                String::new()
            } else {
                f[7].to_string()
            },
        });
    }
    Ok(out)
}

pub fn read(path: &Path) -> Result<Vec<VariantCall>> {
    parse(&std::fs::read_to_string(path)?)
}

/// VCF-Text (Version 4.2) mit Kopfzeilen.
pub fn format(variants: &[VariantCall]) -> String {
    let mut out =
        String::from("##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n");
    for v in variants {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            v.chromosome,
            v.position,
            if v.id.is_empty() { "." } else { &v.id },
            v.ref_allele,
            v.alt_allele,
            v.quality
                .map(|q| format!("{}", q))
                .unwrap_or_else(|| ".".to_string()),
            v.filter,
            if v.info.is_empty() { "." } else { &v.info }
        ));
    }
    out
}

pub fn write(variants: &[VariantCall], path: &Path) -> Result<()> {
    std::fs::write(path, format(variants))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n\
chr1\t100\trs1\ta\tg\t50.5\tPASS\tDP=20;AF=0.5\n\
chr1\t200\t.\tAT\tA\t.\tLowQual\t.\n\
chr2\t5\t.\tA\tACG\t10\tPASS\t.\n\
chr2\t9\t.\tAC\tGT\t10\tPASS\t.\n\
chr2\t12\t.\tA\tG,T\t10\tPASS\t.\n";

    #[test]
    fn varianten_lesen() {
        let v = parse(TEXT).unwrap();
        assert_eq!(v.len(), 5);
        assert_eq!(v[0].chromosome, "chr1");
        assert_eq!(v[0].position, 100);
        assert_eq!(v[0].id, "rs1");
        assert_eq!(v[0].ref_allele, "A");
        assert_eq!(v[0].alt_allele, "G");
        assert_eq!(v[0].quality, Some(50.5));
        assert_eq!(v[0].info, "DP=20;AF=0.5");
        assert_eq!(v[1].quality, None);
        assert_eq!(v[1].id, "");
        assert!(v[0].is_pass() && !v[1].is_pass());
    }

    #[test]
    fn varianten_typen() {
        let v = parse(TEXT).unwrap();
        assert_eq!(v[0].variant_type(), VariantType::Snp);
        assert_eq!(v[1].variant_type(), VariantType::Deletion);
        assert_eq!(v[2].variant_type(), VariantType::Insertion);
        assert_eq!(v[3].variant_type(), VariantType::Mnp);
        assert_eq!(v[4].variant_type(), VariantType::Other);
    }

    #[test]
    fn round_trip() {
        let v = parse(TEXT).unwrap();
        assert_eq!(parse(&format(&v)).unwrap(), v);
        assert!(format(&[]).starts_with("##fileformat=VCFv4.2"));
    }

    #[test]
    fn fehler() {
        assert!(matches!(parse("chr1\t1\t.\tA\tG\n"), Err(Error::Io(m)) if m.contains("Zeile 1")));
        assert!(parse("chr1\tx\t.\tA\tG\t.\tPASS\t.\n").is_err());
        assert!(parse("chr1\t1\t.\tA\tG\tabc\tPASS\t.\n").is_err());
        assert!(parse("").unwrap().is_empty());
    }

    #[test]
    fn datei() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.vcf");
        write(&parse(TEXT).unwrap(), &p).unwrap();
        assert_eq!(read(&p).unwrap().len(), 5);
    }
}
