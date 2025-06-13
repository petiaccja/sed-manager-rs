//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::frontend::Frontend;
use crate::ui;
use slint::ComponentHandle as _;

const UNITS: [ui::DigitalUnit; 10] = [
    ui::DigitalUnit::LBA,
    ui::DigitalUnit::KiB,
    ui::DigitalUnit::MiB,
    ui::DigitalUnit::GiB,
    ui::DigitalUnit::TiB,
    ui::DigitalUnit::KB,
    ui::DigitalUnit::MB,
    ui::DigitalUnit::GB,
    ui::DigitalUnit::TB,
    ui::DigitalUnit::B,
];

pub fn set_callbacks(frontend: Frontend) {
    frontend.with(move |window| {
        let duc = window.global::<ui::DigitalUnitConversion>();
        duc.on_parse(|value, block_size| parse(value.into(), block_size));
        duc.on_to_string(|value, unit, block_size| to_string(value, unit, block_size).into());
    });
}

pub fn parse(value: String, block_size: i32) -> i64 {
    let block_size = std::cmp::max(1, block_size);
    let value = value.trim();
    let Some(unit) = UNITS.iter().find(|unit| value.ends_with(&unit.to_string())) else {
        return -1;
    };
    let numeral_slice = &value[..value.len() - unit.to_string().len()];
    let Ok(numeral) = numeral_slice.trim().parse::<f64>() else {
        return -1;
    };
    let ratio = unit.ratio(block_size);
    (numeral * ratio) as i64
}

pub fn to_string_auto_decimal(value: i64, block_size: i32) -> String {
    let num_bytes = value * block_size as i64;
    if num_bytes < 1000 {
        to_string_concrete(value, ui::DigitalUnit::B, block_size)
    } else if num_bytes < 1000_000 {
        to_string_concrete(value, ui::DigitalUnit::KB, block_size)
    } else if num_bytes < 1000_000_000 {
        to_string_concrete(value, ui::DigitalUnit::MB, block_size)
    } else if num_bytes < 1000_000_000_000 {
        to_string_concrete(value, ui::DigitalUnit::GB, block_size)
    } else {
        to_string_concrete(value, ui::DigitalUnit::TB, block_size)
    }
}

pub fn to_string_auto_binary(value: i64, block_size: i32) -> String {
    let num_bytes = value * block_size as i64;
    if num_bytes < 1024 {
        to_string_concrete(value, ui::DigitalUnit::B, block_size)
    } else if num_bytes < 1024 * 1024 {
        to_string_concrete(value, ui::DigitalUnit::KiB, block_size)
    } else if num_bytes < 1024 * 1024 * 1024 {
        to_string_concrete(value, ui::DigitalUnit::MiB, block_size)
    } else if num_bytes < 1024 * 1024 * 1024 * 1024 {
        to_string_concrete(value, ui::DigitalUnit::GiB, block_size)
    } else {
        to_string_concrete(value, ui::DigitalUnit::TiB, block_size)
    }
}

pub fn to_string_concrete(value: i64, unit: ui::DigitalUnit, block_size: i32) -> String {
    let block_size = std::cmp::max(1, block_size);
    let ratio = unit.ratio(block_size);
    let numeral = value as f64 / ratio;
    let rounded = f64::round(numeral * 1000.0) / 1000.0; // Round to 3 digits.
    format!("{rounded} {unit}")
}

pub fn to_string(value: i64, unit: ui::DigitalUnit, block_size: i32) -> String {
    match unit {
        ui::DigitalUnit::AutoDecimal => to_string_auto_decimal(value, block_size),
        ui::DigitalUnit::AutoBinary => to_string_auto_binary(value, block_size),
        _ => to_string_concrete(value, unit, block_size),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_invalid_numeral() {
        assert_eq!(parse("3..2 KB".into(), 512), -1);
    }

    #[test]
    fn parse_invalid_unit() {
        assert_eq!(parse("274718.4 AU".into(), 512), -1);
    }

    #[test]
    fn parse_invalid_sep() {
        assert_eq!(parse("3_B".into(), 512), -1);
    }

    #[test]
    fn parse_nosep() {
        assert_eq!(parse("1537B".into(), 512), 3);
    }

    #[test]
    fn parse_padded() {
        assert_eq!(parse("  1536   B    ".into(), 512), 3);
    }

    #[test]
    fn parse_lba() {
        assert_eq!(parse("3 LBA".into(), 512), 3);
    }

    #[test]
    fn parse_b() {
        assert_eq!(parse("1536 B".into(), 512), 3);
    }

    #[test]
    fn parse_kib() {
        assert_eq!(parse("1.5 KiB".into(), 512), 1536 / 512);
    }

    #[test]
    fn parse_mib() {
        assert_eq!(parse("1.5 MiB".into(), 512), 1536 * 1024 / 512);
    }

    #[test]
    fn parse_gib() {
        assert_eq!(parse("1.5 GiB".into(), 512), 1536 * 1024 * 1024 / 512);
    }

    #[test]
    fn parse_tib() {
        assert_eq!(parse("1.5 TiB".into(), 512), 1536 * 1024 * 1024 * 1024 / 512);
    }

    #[test]
    fn parse_kb() {
        assert_eq!(parse("1.5 kB".into(), 512), 1500 / 512);
    }

    #[test]
    fn parse_mb() {
        assert_eq!(parse("1.5 MB".into(), 512), 1500_000 / 512);
    }

    #[test]
    fn parse_gb() {
        assert_eq!(parse("1.5 GB".into(), 512), 1500_000_000 / 512);
    }

    #[test]
    fn parse_tb() {
        assert_eq!(parse("1.5 TB".into(), 512), 1500_000_000_000 / 512);
    }

    #[test]
    fn to_string_lba() {
        assert_eq!(to_string(3, ui::DigitalUnit::LBA, 512), "3 LBA");
    }

    #[test]
    fn to_string_other() {
        assert_eq!(to_string(6, ui::DigitalUnit::KiB, 512), "3 KiB");
    }
}
