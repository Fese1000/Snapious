//! Zwischenspeicher für Abrufe auf der Platte und eine Anfragenbremse (Rate-Limit), beide ohne eigenes Warten und mit austauschbarer Uhr.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::ncbi::HttpGet;
use crate::error::Result;

/// Uhr in Sekunden (für den Zwischenspeicher) bzw. Millisekunden (für die Bremse); in Tests ersetzbar.
pub trait Clock {
    fn now_secs(&self) -> u64;
    fn now_millis(&self) -> u64;
}

/// Systemuhr (Zeit seit 1970).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Dateiname für eine Adresse: 64-Bit-FNV-1a-Hash der UTF-8-Bytes der Adresse (Startwert `0xcbf29ce484222325`, Primzahl
/// `0x100000001b3`, je Byte erst XOR, dann Multiplikation mit Überlauf), als 16 Hexziffern in Kleinbuchstaben.
pub fn cache_key(url: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in url.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Ein Abruf mit Zwischenspeicher: gespeichert wird je Adresse eine Datei `<dir>/<cache_key>.txt`, deren erste Zeile die Abrufzeit
/// (Sekunden, `Clock::now_secs`) ist und deren Rest die Antwort.
pub struct CachingHttp<H: HttpGet, C: Clock> {
    inner: H,
    dir: PathBuf,
    ttl_secs: u64,
    clock: C,
}

impl<H: HttpGet, C: Clock> CachingHttp<H, C> {
    /// Neuer Zwischenspeicher. Das Verzeichnis wird beim ersten Schreiben angelegt (`create_dir_all`).
    pub fn new(inner: H, dir: PathBuf, ttl_secs: u64, clock: C) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        CachingHttp {
            inner,
            dir,
            ttl_secs,
            clock,
        }
    }
}

impl<H: HttpGet, C: Clock> HttpGet for CachingHttp<H, C> {
    /// Liegt eine lesbare Datei vor und ist `jetzt − Abrufzeit <= ttl_secs`, wird die gespeicherte Antwort (ohne die erste Zeile,
    /// sonst unverändert) zurückgegeben, ohne `inner` aufzurufen. Sonst wird `inner.get` aufgerufen; bei `Ok(text)` wird die Datei
    /// geschrieben (Schreibfehler werden ignoriert) und `text` zurückgegeben; Fehler von `inner` werden weitergegeben und nicht gespeichert.
    /// Eine beschädigte Datei (erste Zeile keine Zahl) gilt als nicht vorhanden.
    fn get(&self, url: &str) -> Result<String> {
        let key = cache_key(url);
        let path = self.dir.join(format!("{}.txt", key));

        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Some(newline_pos) = content.find('\n') {
                if let Ok(t) = content[..newline_pos].trim().parse::<u64>() {
                    let now = self.clock.now_secs();
                    if now.saturating_sub(t) <= self.ttl_secs {
                        return Ok(content[newline_pos + 1..].to_string());
                    }
                }
            } else if content.trim().parse::<u64>().is_ok() {
                // Nur Zeitstempel, keine Antwort -> ungültig
            }
        }

        let text = self.inner.get(url)?;

        let now = self.clock.now_secs();
        let _ = std::fs::write(&path, format!("{}\n{}", now, text));

        Ok(text)
    }
}

/// Anfragenbremse: höchstens `max_requests` Anfragen je `period_ms` Millisekunden. Ein Zeitpunkt `t` zählt nicht mehr
/// mit, sobald `t + period_ms <= jetzt` gilt. Die Bremse wartet nicht selbst, sie sagt nur, wie lange gewartet werden müsste.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    max_requests: usize,
    period_ms: u64,
    times: Vec<u64>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, period_ms: u64) -> Self {
        RateLimiter {
            max_requests,
            period_ms,
            times: Vec::new(),
        }
    }

    /// Wartezeit in Millisekunden bis zur nächsten erlaubten Anfrage zum Zeitpunkt `now_ms`: 0, wenn weniger als `max_requests`
    /// Zeitpunkte noch zählen; sonst (Zeitpunkt, der als Nächsten ausläuft) `+ period_ms − now_ms`. Bei `max_requests == 0` immer `period_ms`.
    pub fn wait_ms(&self, now_ms: u64) -> u64 {
        if self.max_requests == 0 {
            return self.period_ms;
        }

        let valid: Vec<u64> = self
            .times
            .iter()
            .filter(|&&t| t + self.period_ms > now_ms)
            .copied()
            .collect();

        if valid.len() < self.max_requests {
            0
        } else {
            let oldest = valid.iter().copied().min().unwrap_or(now_ms);
            (oldest + self.period_ms).saturating_sub(now_ms)
        }
    }

    /// Merkt eine Anfrage zum Zeitpunkt `now_ms` und vergisst Zeitpunkte, die nicht mehr zählen.
    pub fn record(&mut self, now_ms: u64) {
        self.times.retain(|&t| t + self.period_ms > now_ms);
        self.times.push(now_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use std::cell::{Cell, RefCell};

    struct FakeClock(Cell<u64>);

    impl Clock for &FakeClock {
        fn now_secs(&self) -> u64 {
            self.0.get()
        }

        fn now_millis(&self) -> u64 {
            self.0.get() * 1000
        }
    }

    struct FakeHttp {
        calls: Cell<usize>,
        answers: RefCell<Vec<Result<String>>>,
    }

    impl HttpGet for &FakeHttp {
        fn get(&self, _url: &str) -> Result<String> {
            self.calls.set(self.calls.get() + 1);
            self.answers.borrow_mut().remove(0)
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "snapious-cache-{}-{}-{}",
            tag,
            std::process::id(),
            crate::util::new_id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn schluessel() {
        // FNV-1a 64 von "" ist der Startwert
        assert_eq!(cache_key(""), "cbf29ce484222325");
        // bekannter Wert: FNV-1a 64 von "a"
        assert_eq!(cache_key("a"), "af63dc4c8601ec8c");
        assert_eq!(cache_key("http://x").len(), 16);
        assert_ne!(cache_key("http://x"), cache_key("http://y"));
    }

    #[test]
    fn zweiter_abruf_kommt_aus_dem_speicher() {
        let clock = FakeClock(Cell::new(1000));
        let http = FakeHttp {
            calls: Cell::new(0),
            answers: RefCell::new(vec![
                Ok("Antwort\nmit Zeile 2".to_string()),
                Ok("neu".to_string()),
            ]),
        };
        let dir = temp_dir("a");
        let c = CachingHttp::new(&http, dir.clone(), 60, &clock);
        assert_eq!(c.get("http://x/a").unwrap(), "Antwort\nmit Zeile 2");
        assert_eq!(c.get("http://x/a").unwrap(), "Antwort\nmit Zeile 2");
        assert_eq!(http.calls.get(), 1);
        clock.0.set(1060);
        assert_eq!(c.get("http://x/a").unwrap(), "Antwort\nmit Zeile 2");
        assert_eq!(http.calls.get(), 1);
        clock.0.set(1061);
        assert_eq!(c.get("http://x/a").unwrap(), "neu");
        assert_eq!(http.calls.get(), 2);
        assert!(dir
            .join(format!("{}.txt", cache_key("http://x/a")))
            .exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn fehler_werden_nicht_gespeichert() {
        let clock = FakeClock(Cell::new(5));
        let http = FakeHttp {
            calls: Cell::new(0),
            answers: RefCell::new(vec![
                Err(Error::Io("500".to_string())),
                Ok("gut".to_string()),
            ]),
        };
        let dir = temp_dir("b");
        let c = CachingHttp::new(&http, dir.clone(), 60, &clock);
        assert!(c.get("http://x/b").is_err());
        assert_eq!(c.get("http://x/b").unwrap(), "gut");
        assert_eq!(http.calls.get(), 2);
        // beschädigte Datei wird ignoriert
        let file = dir.join(format!("{}.txt", cache_key("http://x/b")));
        std::fs::write(&file, "keine zahl\nrest").unwrap();
        let http2 = FakeHttp {
            calls: Cell::new(0),
            answers: RefCell::new(vec![Ok("frisch".to_string())]),
        };
        let c2 = CachingHttp::new(&http2, dir.clone(), 60, &clock);
        assert_eq!(c2.get("http://x/b").unwrap(), "frisch");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn bremse() {
        let mut r = RateLimiter::new(3, 1000);
        assert_eq!(r.wait_ms(0), 0);
        r.record(0);
        r.record(100);
        assert_eq!(r.wait_ms(150), 0);
        r.record(200);
        assert_eq!(r.wait_ms(300), 700);
        assert_eq!(r.wait_ms(999), 1);
        // t = 0 läuft bei 1000 aus
        assert_eq!(r.wait_ms(1000), 0);
        r.record(1000);
        // jetzt zählen 100, 200, 1000
        assert_eq!(r.wait_ms(1001), 99);
        assert_eq!(RateLimiter::new(0, 500).wait_ms(10), 500);
    }

    #[test]
    fn systemuhr() {
        let c = SystemClock;
        assert!(c.now_secs() > 1_700_000_000);
        assert!(c.now_millis() / 1000 >= c.now_secs() - 1);
    }
}
