//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::fmt::Display;
use std::str::FromStr;

use sed_manager::Alignment;
use sed_manager_gui_slint as ui;
use slint::SharedString;

use crate::ui_conv::FromUi as _;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Unit {
    B,
    KB,
    MB,
    GB,
    TB,
    KiB,
    MiB,
    GiB,
    TiB,
    Lba,
}

impl Unit {
    /// How many bytes the unit equals to. LBAs are treated differently and return `None`.
    pub const fn bytes_per_unit(&self) -> Option<u64> {
        match self {
            Unit::Lba => None,
            Unit::B => Some(1),
            Unit::KB => Some(1_000),
            Unit::MB => Some(1_000_000),
            Unit::GB => Some(1_000_000_000),
            Unit::TB => Some(1_000_000_000_000),
            Unit::KiB => Some(1024),
            Unit::MiB => Some(1024 * 1024),
            Unit::GiB => Some(1024 * 1024 * 1024),
            Unit::TiB => Some(1024 * 1024 * 1024 * 1024),
        }
    }
}

impl FromStr for Unit {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "lba" => Ok(Unit::Lba),
            "b" => Ok(Unit::B),
            "kb" => Ok(Unit::KB),
            "mb" => Ok(Unit::MB),
            "gb" => Ok(Unit::GB),
            "tb" => Ok(Unit::TB),
            "kib" => Ok(Unit::KiB),
            "mib" => Ok(Unit::MiB),
            "gib" => Ok(Unit::GiB),
            "tib" => Ok(Unit::TiB),
            _ => Err(()),
        }
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Unit::B => "B",
            Unit::KB => "KB",
            Unit::MB => "MB",
            Unit::GB => "GB",
            Unit::TB => "TB",
            Unit::KiB => "KiB",
            Unit::MiB => "MiB",
            Unit::GiB => "GiB",
            Unit::TiB => "TiB",
            Unit::Lba => "LBA",
        };
        f.write_str(s)
    }
}

// Splits off the leading (unsigned, decimal) numeric part of `str`. LBAs can't be negative,
// so a leading sign is deliberately not accepted here; it just makes the number part empty,
// and thus the whole string invalid.
fn split_number_unit(str: &str) -> (&str, &str) {
    let bytes = str.as_bytes();
    let mut end = 0;
    while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
        end += 1;
    }
    (&str[..end], &str[end..])
}

fn parse_number_and_unit(str: &str) -> Option<(f64, Unit)> {
    let (number_part, unit_part) = split_number_unit(str.trim());
    let value: f64 = number_part.parse().ok()?;
    let unit = unit_part.parse().ok()?;
    Some((value, unit))
}

/// Convert the value & unit to LBAs. Return `None` if the size of a sector is not
/// known and the conversion cannot be performed.
fn to_lba(value: f64, unit: Unit, geometry: &ui::Geometry) -> Option<i64> {
    let lba = match unit.bytes_per_unit() {
        None => value,
        Some(bytes_per_unit) => match geometry.logical_sector_size {
            0 => return None,
            bytes_per_sector => value * bytes_per_unit as f64 / bytes_per_sector as f64,
        },
    };
    Some(lba.round() as i64)
}

// Formats `value` in fixed-point notation with at most `sig_digits` significant digits,
// trimming trailing zeros and a trailing decimal point where possible.
fn format_significant(value: f64, sig_digits: i32) -> String {
    if value <= 0.0 {
        return "0".to_string();
    }
    let magnitude = value.log10().floor() as i32;
    let decimals = (sig_digits - 1 - magnitude).max(0) as usize;
    let formatted = format!("{:.*}", decimals, value);
    if formatted.contains('.') {
        formatted.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        formatted
    }
}

pub fn to_string(sector: ui::Sector, geometry: ui::Geometry) -> SharedString {
    let lba = sector.value.max(0);

    // The sector size is unknown, so conversion to bytes is impossible: fall back to LBAs.
    if geometry.logical_sector_size <= 0 {
        format!("{} {}", lba, Unit::Lba).into()
    } else {
        let num_bytes = lba * geometry.logical_sector_size as i64;

        for unit in [Unit::TiB, Unit::GiB, Unit::MiB, Unit::KiB] {
            let bytes_per_unit = unit.bytes_per_unit().expect("byte units always have a byte size");
            if num_bytes >= bytes_per_unit as i64 {
                let value = num_bytes as f64 / bytes_per_unit as f64;
                return format!("{} {}", format_significant(value, 5), unit).into();
            }
        }
        format!("{} {}", format_significant(num_bytes as f64, 5), Unit::B).into()
    }
}

pub fn is_valid(str: SharedString, geometry: ui::Geometry) -> bool {
    let Some((value, unit)) = parse_number_and_unit(&str) else {
        return false;
    };
    to_lba(value, unit, &geometry).is_some()
}

pub fn parse(str: SharedString, geometry: ui::Geometry) -> ui::Sector {
    let Some((value, unit)) = parse_number_and_unit(&str) else {
        return ui::Sector::default();
    };
    ui::Sector { value: to_lba(value, unit, &geometry).unwrap_or(0) }
}

// Aligns the LBA such that `(value - lowest_aligned_lba) modulo alignment_granularity == 0`.
// Always rounds down (so that align after capping to drive size does not bump it over the
// size again).
pub fn align(sector: ui::Sector, alignment: ui::Alignment) -> ui::Sector {
    let alignment = Alignment::from_ui(&alignment);
    let value = alignment.align_lba(sector.value.max(0) as u64);
    ui::Sector { value: value as i64 }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn geometry(logical_sector_size: i32) -> ui::Geometry {
        ui::Geometry { logical_sector_size, logical_sector_count: ui::Sector { value: 0 } }
    }

    #[rstest]
    #[case::bytes(500, 1, "500 B")]
    // 2000 sectors * 512 B = 1,024,000 B = 1000 KiB.
    #[case::kib(2000, 512, "1000 KiB")]
    // 26030 sectors * 512 B = 13,327,360 B = 12.7097... MiB, rounded to 5 sig digits.
    #[case::mib(26030, 512, "12.71 MiB")]
    #[case::gib(1 << 21, 1024, "2 GiB")]
    #[case::zero(0, 512, "0 B")]
    #[case::unknown_sector_size_falls_back_to_lba(12345, 0, "12345 LBA")]
    fn to_string_cases(#[case] lba: i64, #[case] sector_size: i32, #[case] expected: &str) {
        let sector = ui::Sector { value: lba };
        assert_eq!(to_string(sector, geometry(sector_size)).as_str(), expected);
    }

    #[rstest]
    // Plain numbers.
    #[case::zero("0", 512, true)]
    #[case::integer("12", 512, true)]
    #[case::decimal("12.5", 512, true)]
    #[case::leading_dot(".5", 512, true)]
    #[case::trailing_dot("5.", 512, true)]
    // Units, with and without a space, case-insensitive.
    #[case::mib_with_space("12 MiB", 512, true)]
    #[case::mib_without_space("12MiB", 512, true)]
    #[case::mib_lowercase("12 mib", 512, true)]
    #[case::lba_uppercase("1 LBA", 512, true)]
    #[case::lba_lowercase("1 lba", 512, true)]
    #[case::kb("3 kB", 512, true)]
    #[case::gb("3 GB", 512, true)]
    #[case::tib("3 TiB", 512, true)]
    #[case::b("3 B", 512, true)]
    // Garbage.
    #[case::empty("", 512, false)]
    #[case::not_a_number("abc", 512, false)]
    #[case::negative("-5", 512, false)]
    #[case::unknown_unit("5 XiB", 512, false)]
    #[case::two_decimal_points("5..5", 512, false)]
    #[case::unit_without_a_number("MiB", 512, false)]
    // Zero sector size: byte-based units can't be converted, so only LBA is valid.
    #[case::zero_sector_size_bare_number("1234", 0, true)]
    #[case::zero_sector_size_explicit_lba("1234 LBA", 0, true)]
    #[case::zero_sector_size_rejects_mib("5 MiB", 0, false)]
    #[case::zero_sector_size_rejects_kb("5 kB", 0, false)]
    fn is_valid_cases(#[case] str: &str, #[case] sector_size: i32, #[case] expected: bool) {
        assert_eq!(is_valid(str.into(), geometry(sector_size)), expected);
    }

    #[rstest]
    #[case::bare_number_is_lba("1234", 512, 1234)]
    #[case::explicit_lba("1234 LBA", 512, 1234)]
    // 1000 KiB = 1,024,000 B = 2000 sectors of 512 B.
    #[case::kib_with_space("1000 KiB", 512, 2000)]
    #[case::kib_without_space("1000KiB", 512, 2000)]
    // 1 kB = 1000 B = 2 sectors of 500 B.
    #[case::decimal_unit("1 kB", 500, 2)]
    #[case::invalid_falls_back_to_zero("garbage", 512, 0)]
    #[case::zero_sector_size_falls_back_to_zero_for_byte_units("5 MiB", 0, 0)]
    #[case::zero_sector_size_still_accepts_bare_number("1234", 0, 1234)]
    #[case::zero_sector_size_still_accepts_explicit_lba("1234 LBA", 0, 1234)]
    fn parse_cases(#[case] str: &str, #[case] sector_size: i32, #[case] expected: i64) {
        assert_eq!(parse(str.into(), geometry(sector_size)).value, expected);
    }

    #[test]
    fn align_cases() {
        let sector = ui::Sector { value: 27 };
        let alignment = ui::Alignment {
            alignment_granularity: ui::Sector { value: 8 },
            alignment_required: true,
            lowest_aligned_lba: ui::Sector { value: 4 },
        };
        assert_eq!(align(sector, alignment).value, 20);
    }
}
