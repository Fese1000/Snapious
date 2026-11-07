//! Small helper functions without dependencies.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Current time in seconds since 1970 (UTC).
pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// New unique ID (timestamp in nanoseconds + counter, hexadecimal).
pub fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", nanos, n)
}

/// Small deterministic random number generator (SplitMix64) for bootstrap and tests.
/// The same seed always gives the same sequence, independent of platform and threads.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniformly distributed number in `0..n` (`n` must be greater than 0; returns 0 for 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_sind_eindeutig() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
    }

    #[test]
    fn zeit_ist_plausibel() {
        assert!(now_secs() > 1_700_000_000);
    }

    #[test]
    fn rng_ist_deterministisch_und_verteilt() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let xs: Vec<usize> = (0..5).map(|_| a.below(100)).collect();
        let ys: Vec<usize> = (0..5).map(|_| b.below(100)).collect();
        assert_eq!(xs, ys);
        let mut r = Rng::new(1);
        let mut counts = [0usize; 4];
        for _ in 0..4000 {
            counts[r.below(4)] += 1;
        }
        assert!(counts.iter().all(|c| *c > 800 && *c < 1200), "{:?}", counts);
        assert_eq!(Rng::new(7).below(0), 0);
    }
}
