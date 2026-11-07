//! On-disk cache for fetches and a request throttle (rate limit), both without waiting themselves and with a replaceable clock.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::ncbi::HttpGet;
use crate::error::Result;

/// Clock in seconds (for the cache) or milliseconds (for the throttle); replaceable in tests.
pub trait Clock {
    fn now_secs(&self) -> u64;
    fn now_millis(&self) -> u64;
}

/// System clock (time since 1970).
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

/// File name for an address: 64-bit FNV-1a hash of the UTF-8 bytes of the address (offset basis `0xcbf29ce484222325`, prime
/// `0x100000001b3`, per byte first XOR, then multiplication with overflow), as 16 lowercase hex digits.
pub fn cache_key(url: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in url.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// A fetch with a cache: per address a file `<dir>/<cache_key>.txt` is stored whose first line is the fetch time
/// (seconds, `Clock::now_secs`) and whose remainder is the response.
pub struct CachingHttp<H: HttpGet, C: Clock> {
    inner: H,
    dir: PathBuf,
    ttl_secs: u64,
    clock: C,
}

impl<H: HttpGet, C: Clock> CachingHttp<H, C> {
    /// New cache. The directory is created on first write (`create_dir_all`).
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
    /// If a readable file exists and `now − fetch time <= ttl_secs`, the stored response (without the first line,
    /// otherwise unchanged) is returned without calling `inner`. Otherwise `inner.get` is called; on `Ok(text)` the file is
    /// written (write errors are ignored) and `text` is returned; errors from `inner` are propagated and not stored.
    /// A corrupted file (first line not a number) counts as absent.
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
                // Only a timestamp, no response -> invalid
            }
        }

        let text = self.inner.get(url)?;

        let now = self.clock.now_secs();
        let _ = std::fs::write(&path, format!("{}\n{}", now, text));

        Ok(text)
    }
}

/// Request throttle: at most `max_requests` requests per `period_ms` milliseconds. A time `t` no longer counts
/// once `t + period_ms <= now` holds. The throttle does not wait itself; it only says how long one would have to wait.
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

    /// Wait time in milliseconds until the next permitted request at time `now_ms`: 0 if fewer than `max_requests`
    /// times still count; otherwise (time that expires next) `+ period_ms − now_ms`. With `max_requests == 0` always `period_ms`.
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

    /// Records a request at time `now_ms` and forgets times that no longer count.
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
        // FNV-1a 64 of "" is the offset basis
        assert_eq!(cache_key(""), "cbf29ce484222325");
        // known value: FNV-1a 64 of "a"
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
        // corrupted file is ignored
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
        // t = 0 expires at 1000
        assert_eq!(r.wait_ms(1000), 0);
        r.record(1000);
        // now 100, 200, 1000 count
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
