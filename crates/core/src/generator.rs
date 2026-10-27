//! Password generator on the operating system's CSPRNG (`getrandom`), with
//! unbiased sampling.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub const LENGTH: std::ops::RangeInclusive<u32> = 8..=128;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!#$%&*+-=?@^_~";
/// Characters easily confused with one another.
pub const AMBIGUOUS: &str = "Il1O0o";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorOptions {
    pub length: u32,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        Self {
            length: 20,
            lowercase: true,
            uppercase: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: false,
        }
    }
}

impl GeneratorOptions {
    /// Same rules as `Settings::normalized`: length in range, at least one set.
    pub fn normalized(mut self) -> Self {
        self.length = self.length.clamp(*LENGTH.start(), *LENGTH.end());
        if !(self.lowercase || self.uppercase || self.digits || self.symbols) {
            self.lowercase = true;
        }
        self
    }

    fn sets(&self) -> Vec<Vec<char>> {
        [
            (self.lowercase, LOWERCASE),
            (self.uppercase, UPPERCASE),
            (self.digits, DIGITS),
            (self.symbols, SYMBOLS),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, set)| {
            set.chars()
                .filter(|c| !(self.exclude_ambiguous && AMBIGUOUS.contains(*c)))
                .collect()
        })
        .collect()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("the operating system's random number generator failed")]
pub struct RandomError;

/// Uniform index in `0..n` (n > 0) by rejection sampling.
fn index(n: usize) -> Result<usize, RandomError> {
    let n = u32::try_from(n).map_err(|_| RandomError)?;
    // Largest multiple of n that fits in u32: values at or above it are redrawn.
    let zone = u32::MAX - (u32::MAX % n);
    loop {
        let mut bytes = [0u8; 4];
        getrandom::fill(&mut bytes).map_err(|_| RandomError)?;
        let value = u32::from_le_bytes(bytes);
        if value < zone {
            return Ok((value % n) as usize);
        }
    }
}

/// Generates a password with at least one character of every enabled set.
pub fn generate(options: &GeneratorOptions) -> Result<Zeroizing<String>, RandomError> {
    let options = options.normalized();
    let sets = options.sets();
    let all: Vec<char> = sets.iter().flatten().copied().collect();
    let length = options.length as usize;

    let mut chars: Zeroizing<Vec<char>> = Zeroizing::new(Vec::with_capacity(length));
    for set in &sets {
        chars.push(set[index(set.len())?]);
    }
    while chars.len() < length {
        chars.push(all[index(all.len())?]);
    }
    // Fisher–Yates, so the guaranteed characters are not always first.
    for i in (1..chars.len()).rev() {
        let j = index(i + 1)?;
        chars.swap(i, j);
    }
    Ok(Zeroizing::new(chars.iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(lower: bool, upper: bool, digits: bool, symbols: bool) -> GeneratorOptions {
        GeneratorOptions {
            length: 12,
            lowercase: lower,
            uppercase: upper,
            digits,
            symbols,
            exclude_ambiguous: false,
        }
    }

    #[test]
    fn every_enabled_set_is_present_and_others_absent() {
        for _ in 0..200 {
            let pw = generate(&options(false, true, true, false)).expect("generate");
            assert_eq!(pw.chars().count(), 12);
            assert!(pw.chars().any(|c| c.is_ascii_uppercase()));
            assert!(pw.chars().any(|c| c.is_ascii_digit()));
            assert!(
                pw.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            );
        }
    }

    #[test]
    fn short_passwords_still_contain_every_set() {
        let all = GeneratorOptions {
            length: 8,
            ..GeneratorOptions::default()
        };
        for _ in 0..200 {
            let pw = generate(&all).expect("generate");
            assert!(pw.chars().any(|c| c.is_ascii_lowercase()));
            assert!(pw.chars().any(|c| c.is_ascii_uppercase()));
            assert!(pw.chars().any(|c| c.is_ascii_digit()));
            assert!(pw.chars().any(|c| SYMBOLS.contains(c)));
        }
    }

    #[test]
    fn ambiguous_characters_can_be_excluded() {
        let o = GeneratorOptions {
            length: 128,
            exclude_ambiguous: true,
            ..GeneratorOptions::default()
        };
        for _ in 0..50 {
            let pw = generate(&o).expect("generate");
            assert!(!pw.chars().any(|c| AMBIGUOUS.contains(c)));
        }
    }

    #[test]
    fn options_are_normalized() {
        let pw = generate(&GeneratorOptions {
            length: 1000,
            ..options(false, false, false, false)
        })
        .expect("generate");
        assert_eq!(pw.len(), 128);
        assert!(pw.chars().all(|c| c.is_ascii_lowercase()));
        assert_eq!(
            generate(&GeneratorOptions {
                length: 1,
                ..GeneratorOptions::default()
            })
            .expect("generate")
            .len(),
            8
        );
    }

    #[test]
    fn digits_are_roughly_uniform() {
        // Chi-squared over 20 000 digits, 9 degrees of freedom: 40 is far
        // beyond the 0.001 critical value (27.9), so this fails only if biased.
        let o = GeneratorOptions {
            length: 100,
            ..options(false, false, true, false)
        };
        let mut counts = [0f64; 10];
        for _ in 0..200 {
            for c in generate(&o).expect("generate").chars() {
                counts[c.to_digit(10).expect("digit") as usize] += 1.0;
            }
        }
        let expected = counts.iter().sum::<f64>() / 10.0;
        let chi2: f64 = counts
            .iter()
            .map(|c| (c - expected).powi(2) / expected)
            .sum();
        assert!(chi2 < 40.0, "chi2 = {chi2}");
    }
}
