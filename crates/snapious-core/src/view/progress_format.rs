//! Texte für Fortschrittsanzeigen: Dauer, Größe, Restzeit.

/// Dauer in Sekunden als Text: unter 60 → `"{s} s"`; unter 3600 → `"{m} min {s} s"` (ohne Sekunden, wenn 0: `"{m} min"`);
/// sonst `"{h} h {m} min"` (ohne Minuten, wenn 0: `"{h} h"`; Sekunden entfallen). Beispiele: 0 → `"0 s"`, 125 → `"2 min 5 s"`,
/// 3600 → `"1 h"`, 3725 → `"1 h 2 min"`.
pub fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let remaining = secs % 3600;
    let minutes = remaining / 60;
    let seconds = remaining % 60;

    if hours > 0 {
        if minutes > 0 {
            format!("{} h {} min", hours, minutes)
        } else {
            format!("{} h", hours)
        }
    } else if minutes > 0 {
        if seconds > 0 {
            format!("{} min {} s", minutes, seconds)
        } else {
            format!("{} min", minutes)
        }
    } else {
        format!("{} s", seconds)
    }
}

/// Größe in Byte: unter 1024 → `"{n} B"`; sonst durch 1024 teilen in KB, MB, GB (größer wird weiter in GB angegeben)
/// mit einer Nachkommastelle, z. B. 1536 → `"1.5 KB"`, 1048576 → `"1.0 MB"`.
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", (bytes as f64) / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", (bytes as f64) / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", (bytes as f64) / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Geschätzte Restzeit in ganzen Sekunden (gerundet) aus bisher erledigter Menge `done` von `total` nach `elapsed_secs`:
/// `elapsed * (total - done) / done`. `None`, wenn `done == 0`, `done > total` oder `elapsed_secs <= 0.0`.
pub fn eta_secs(done: u64, total: u64, elapsed_secs: f64) -> Option<u64> {
    if done == 0 || done > total || elapsed_secs <= 0.0 {
        return None;
    }
    let remaining = (total - done) as f64;
    Some((elapsed_secs * remaining / (done as f64)).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dauer() {
        assert_eq!(format_duration(0), "0 s");
        assert_eq!(format_duration(59), "59 s");
        assert_eq!(format_duration(60), "1 min");
        assert_eq!(format_duration(125), "2 min 5 s");
        assert_eq!(format_duration(3600), "1 h");
        assert_eq!(format_duration(3725), "1 h 2 min");
        assert_eq!(format_duration(7200), "2 h");
    }

    #[test]
    fn groesse() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(5 * 1024 * 1024 * 1024), "5.0 GB");
        assert_eq!(format_bytes(2048 * 1024 * 1024 * 1024), "2048.0 GB");
    }

    #[test]
    fn restzeit() {
        assert_eq!(eta_secs(25, 100, 50.0), Some(150));
        assert_eq!(eta_secs(100, 100, 10.0), Some(0));
        assert_eq!(eta_secs(1, 3, 1.0), Some(2));
        assert_eq!(eta_secs(0, 100, 5.0), None);
        assert_eq!(eta_secs(101, 100, 5.0), None);
        assert_eq!(eta_secs(5, 100, 0.0), None);
    }
}
