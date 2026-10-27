//! Password strength with zxcvbn, mapped to the four UI levels.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrengthEstimate {
    /// 0 empty, 1 weak, 2 fair, 3 good, 4 strong.
    pub level: u8,
    /// log2 of the estimated number of guesses.
    pub bits: u32,
}

pub fn estimate(password: &str) -> StrengthEstimate {
    if password.is_empty() {
        return StrengthEstimate { level: 0, bits: 0 };
    }
    let entropy = zxcvbn::zxcvbn(password, &[]);
    let level = match u8::from(entropy.score()) {
        0 | 1 => 1,
        2 => 2,
        3 => 3,
        _ => 4,
    };
    let bits = (entropy.guesses_log10() * std::f64::consts::LOG2_10).round();
    StrengthEstimate {
        level,
        bits: bits.max(0.0) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(estimate(""), StrengthEstimate { level: 0, bits: 0 });
        assert!(estimate("Password123!").level <= 2, "predictable");
        assert_eq!(estimate("password").level, 1);
        assert_eq!(
            estimate("correct-horse-battery-staple-ocean-violin").level,
            4
        );
        assert_eq!(estimate("tW8#qLz!v2Rm9@kP4sXe").level, 4);
    }

    #[test]
    fn bits_grow_with_length() {
        assert!(estimate("tW8#qLz!v2Rm").bits < estimate("tW8#qLz!v2Rm9@kP4sXe").bits);
    }
}
