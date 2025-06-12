//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::DigitalUnit;

impl DigitalUnit {
    pub fn ratio(&self, lba_size: i32) -> f64 {
        (match self {
            DigitalUnit::LBA => lba_size as i64,
            DigitalUnit::B => 1,
            DigitalUnit::KiB => 1024,
            DigitalUnit::MiB => 1048576,
            DigitalUnit::GiB => 1073741824,
            DigitalUnit::TiB => 1099511627776,
            DigitalUnit::KB => 1000,
            DigitalUnit::MB => 1000_000,
            DigitalUnit::GB => 1000_000_000,
            DigitalUnit::TB => 1000_000_000_000,
            DigitalUnit::AutoDecimal => panic!("auto units don't have a ratio and need special handling"),
            DigitalUnit::AutoBinary => panic!("auto units don't have a ratio and need special handling"),
        }) as f64
            / lba_size as f64
    }
}

impl core::fmt::Display for DigitalUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DigitalUnit::LBA => "LBA",
            DigitalUnit::B => "B",
            DigitalUnit::KiB => "KiB",
            DigitalUnit::MiB => "MiB",
            DigitalUnit::GiB => "GiB",
            DigitalUnit::TiB => "TiB",
            DigitalUnit::KB => "KB",
            DigitalUnit::MB => "MB",
            DigitalUnit::GB => "GB",
            DigitalUnit::TB => "TB",
            DigitalUnit::AutoDecimal => "Auto: decimal",
            DigitalUnit::AutoBinary => "Auto: binary",
        };
        write!(f, "{s}")
    }
}
