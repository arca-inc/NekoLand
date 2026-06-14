//! Petit PRNG sans dépendance (xorshift amorcé sur l'horloge), suffisant pour des
//! positions « aléatoires » d'errance et de pelote.

use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn rand_unit() -> f64 {
    thread_local!(static SEED: Cell<u64> = Cell::new(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15)
            | 1
    ));
    SEED.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}
