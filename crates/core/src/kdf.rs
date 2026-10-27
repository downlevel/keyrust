//! Argon2id parameters for new databases and their calibration.

use std::time::{Duration, Instant};

use crate::model::Argon2idParams;

/// Defaults for new databases (aligned with KeePassXC): 64 MiB, 2 lanes.
pub const DEFAULT_MEMORY_MIB: u32 = 64;
pub const DEFAULT_PARALLELISM: u32 = 2;
pub const MIN_ITERATIONS: u32 = 2;
/// Upper bound so calibration on a very fast machine stays sane.
pub const MAX_ITERATIONS: u32 = 1_000;
pub const TARGET: Duration = Duration::from_secs(1);

/// Default parameters, with iterations calibrated to about one second here.
pub fn default_params() -> Argon2idParams {
    Argon2idParams {
        memory_mib: DEFAULT_MEMORY_MIB,
        iterations: calibrate(DEFAULT_MEMORY_MIB, DEFAULT_PARALLELISM, TARGET),
        parallelism: DEFAULT_PARALLELISM,
    }
}

/// Iterations that take about `target` with this memory and parallelism.
pub fn calibrate(memory_mib: u32, parallelism: u32, target: Duration) -> u32 {
    let per_iteration = time_one(memory_mib, parallelism, MIN_ITERATIONS) / MIN_ITERATIONS;
    if per_iteration.is_zero() {
        return MAX_ITERATIONS;
    }
    let iterations = target.as_secs_f64() / per_iteration.as_secs_f64();
    (iterations.round() as u32).clamp(MIN_ITERATIONS, MAX_ITERATIONS)
}

fn time_one(memory_mib: u32, parallelism: u32, iterations: u32) -> Duration {
    let config = argon2::Config {
        variant: argon2::Variant::Argon2id,
        version: argon2::Version::Version13,
        mem_cost: memory_mib.saturating_mul(1024),
        time_cost: iterations,
        lanes: parallelism.max(1),
        hash_length: 32,
        ..argon2::Config::default()
    };
    let start = Instant::now();
    // Fixed inputs: only the duration matters.
    let _ = argon2::hash_raw(b"keyrust-calibration", b"keyrust-calibration-salt", &config);
    start.elapsed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_is_in_range_and_scales_with_the_target() {
        let short = calibrate(8, 1, Duration::from_millis(20));
        let long = calibrate(8, 1, Duration::from_millis(200));
        assert!((MIN_ITERATIONS..=MAX_ITERATIONS).contains(&short));
        assert!(long >= short, "{long} < {short}");
    }
}
