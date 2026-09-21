//! Numeric interpretation: the static encoded width a PTC/PFC declaration establishes,
//! the number a text-valued SCOS cell records, and the unsigned conversion of a recorded
//! numeric cell into the range its field requires.

use super::resolution::{info, unavailable};
use crate::model::{Info, Scalar};

pub(super) fn encoded_bits(ptc: Option<u16>, pfc: Option<u32>) -> Option<u64> {
    match (ptc, pfc) {
        (Some(1), Some(0)) => Some(1),
        (Some(2 | 6), Some(n @ 1..=32)) => Some(u64::from(n)),
        (Some(3 | 4), Some(n @ 0..=12)) => Some(u64::from(n) + 4),
        (Some(3 | 4), Some(13)) => Some(24),
        (Some(3 | 4), Some(14)) => Some(32),
        (Some(3 | 4), Some(15)) => Some(48),
        (Some(3 | 4), Some(16)) => Some(64),
        (Some(5), Some(1 | 3)) => Some(32),
        (Some(5), Some(2)) => Some(64),
        (Some(5), Some(4)) => Some(48),
        (Some(7 | 8), Some(n @ 1..)) => Some(u64::from(n) * 8),
        (Some(9), Some(1)) => Some(48),
        (Some(9), Some(2 | 30)) => Some(64),
        (Some(9 | 10), Some(n @ 3..=18)) => {
            // CUC formats enumerate one to four coarse octets, each with zero to three fine octets.
            Some(u64::from(1 + (n - 3) / 4 + (n - 3) % 4) * 8)
        }
        _ => None,
    }
}

/// Text-valued SCOS cells take their interpretation from the declared format and radix.
pub(super) fn number(
    cell: &Info<String>,
    format: Option<&str>,
    radix: Option<&str>,
) -> Info<Scalar> {
    let Some(text) = &cell.value else {
        return info(None, &cell.sources[0]);
    };
    let radix = match radix {
        Some("H") => 16,
        Some("O") => 8,
        _ => 10,
    };
    let value = match format {
        Some("U") => u64::from_str_radix(text, radix).ok().map(Scalar::Unsigned),
        Some("I") => text.parse::<i64>().ok().map(Scalar::Integer),
        Some("R") => text
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(|_| Scalar::Decimal(text.clone())),
        _ => None,
    };
    if value.is_none() {
        unavailable(
            &cell.sources[0],
            "Recorded number cannot be interpreted with its declared format",
        )
    } else {
        info(value, &cell.sources[0])
    }
}

pub(super) fn unsigned<T: TryFrom<i64>>(cell: &Info<i64>, field: &str) -> Info<T> {
    let value = cell.value.and_then(|v| T::try_from(v).ok());
    if cell.value.is_some() && value.is_none() {
        unavailable(
            &cell.sources[0],
            &format!("{field} is outside the supported unsigned range"),
        )
    } else {
        Info {
            value,
            problems: cell.problems.clone(),
            sources: cell.sources.clone(),
        }
    }
}
