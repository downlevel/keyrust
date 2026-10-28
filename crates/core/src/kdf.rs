//! Argon2id parameters for new databases and their calibration.

use std::time::{Duration, Instant};

use crate::model::Argon2idParams;

/// The same ceilings as opening (`kdbx::limits::Limits::DEFAULT`), so Keyrust
/// never writes a database it would then refuse to open.
pub const MAX_MEMORY_MIB: u32 = 2048;
pub const MAX_PARALLELISM: u32 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum KdfError {
    #[error("invalid key derivation parameters: {0}")]
    Invalid(&'static str),
    #[error("key derivation failed")]
    Failed,
}

/// The complete range Argon2id and keepass-rs support, including Argon2's
/// requirement of at least 8 KiB of memory per lane.
pub fn validate(params: &Argon2idParams) -> Result<(), KdfError> {
    if !(8..=MAX_MEMORY_MIB).contains(&params.memory_mib) {
        return Err(KdfError::Invalid("memory"));
    }
    if !(1..=MAX_PARALLELISM).contains(&params.parallelism) {
        return Err(KdfError::Invalid("parallelism"));
    }
    if !(1..=MAX_ITERATIONS).contains(&params.iterations) {
        return Err(KdfError::Invalid("iterations"));
    }
    let memory_kib = u64::from(params.memory_mib) * 1024;
    if memory_kib < 8 * u64::from(params.parallelism) {
        return Err(KdfError::Invalid("memory per lane"));
    }
    Ok(())
}

/// Defaults for new databases (aligned with KeePassXC): 64 MiB, 2 lanes.
pub const DEFAULT_MEMORY_MIB: u32 = 64;
pub const DEFAULT_PARALLELISM: u32 = 2;
pub const MIN_ITERATIONS: u32 = 2;
/// Upper bound so calibration on a very fast machine stays sane.
pub const MAX_ITERATIONS: u32 = 1_000;
pub const TARGET: Duration = Duration::from_secs(1);

/// Default parameters, with iterations calibrated to about one second here.
pub fn default_params() -> Result<Argon2idParams, KdfError> {
    Ok(Argon2idParams {
        memory_mib: DEFAULT_MEMORY_MIB,
        iterations: calibrate(DEFAULT_MEMORY_MIB, DEFAULT_PARALLELISM, TARGET)?,
        parallelism: DEFAULT_PARALLELISM,
    })
}

/// Iterations that take about `target` with this memory and parallelism.
pub fn calibrate(memory_mib: u32, parallelism: u32, target: Duration) -> Result<u32, KdfError> {
    validate(&Argon2idParams {
        memory_mib,
        iterations: MIN_ITERATIONS,
        parallelism,
    })?;
    let per_iteration = time_one(memory_mib, parallelism, MIN_ITERATIONS)? / MIN_ITERATIONS;
    if per_iteration.is_zero() {
        return Ok(MAX_ITERATIONS);
    }
    let iterations = target.as_secs_f64() / per_iteration.as_secs_f64();
    Ok((iterations.round() as u32).clamp(MIN_ITERATIONS, MAX_ITERATIONS))
}

fn time_one(memory_mib: u32, parallelism: u32, iterations: u32) -> Result<Duration, KdfError> {
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
    argon2::hash_raw(b"keyrust-calibration", b"keyrust-calibration-salt", &config)
        .map_err(|_| KdfError::Failed)?;
    Ok(start.elapsed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_is_in_range_and_scales_with_the_target() {
        let short = calibrate(8, 1, Duration::from_millis(20)).expect("calibrate");
        let long = calibrate(8, 1, Duration::from_millis(200)).expect("calibrate");
        assert!((MIN_ITERATIONS..=MAX_ITERATIONS).contains(&short));
        assert!(long >= short, "{long} < {short}");
    }

    #[test]
    fn invalid_parameters_are_rejected_before_any_work() {
        let p = |memory_mib, iterations, parallelism| Argon2idParams {
            memory_mib,
            iterations,
            parallelism,
        };
        assert!(validate(&p(8, 1, 1)).is_ok());
        assert!(validate(&p(MAX_MEMORY_MIB, MAX_ITERATIONS, MAX_PARALLELISM)).is_ok());
        assert_eq!(validate(&p(7, 1, 1)), Err(KdfError::Invalid("memory")));
        assert_eq!(
            validate(&p(MAX_MEMORY_MIB + 1, 1, 1)),
            Err(KdfError::Invalid("memory"))
        );
        assert_eq!(validate(&p(8, 0, 1)), Err(KdfError::Invalid("iterations")));
        assert_eq!(
            validate(&p(8, MAX_ITERATIONS + 1, 1)),
            Err(KdfError::Invalid("iterations"))
        );
        assert_eq!(validate(&p(8, 1, 0)), Err(KdfError::Invalid("parallelism")));
        assert_eq!(
            validate(&p(8, 1, MAX_PARALLELISM + 1)),
            Err(KdfError::Invalid("parallelism"))
        );
        assert_eq!(
            calibrate(8, 2048, TARGET),
            Err(KdfError::Invalid("parallelism"))
        );
    }

    #[test]
    fn creatable_parameters_always_open() {
        use crate::kdbx::limits::Limits;
        let max = Limits::DEFAULT;
        assert!(u64::from(MAX_MEMORY_MIB) * 1024 * 1024 <= max.max_argon2_memory_bytes);
        assert!(u64::from(MAX_ITERATIONS) <= max.max_argon2_iterations);
        assert!(MAX_PARALLELISM <= max.max_argon2_parallelism);
    }
}
