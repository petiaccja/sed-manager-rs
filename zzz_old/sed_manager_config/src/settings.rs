//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::fs;
use std::io::{Read, Write};

use crate::license::{get_license_fingerprint, get_plain_license};
use crate::ui;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde(default = "make_none")]
    pub accepted_license_fingerprint: Option<String>,
    #[serde(default = "default_theme")]
    pub theme: ui::Theme,
}

pub fn set_ui(settings: Settings, ui: &ui::SettingsState) {
    ui.set_license_text(get_plain_license().into());
    ui.set_accepted_license_fingerprint(settings.accepted_license_fingerprint.unwrap_or("".into()).into());
    ui.set_license_fingerprint(get_license_fingerprint().into());
    ui.set_theme(settings.theme);
}

pub fn get_ui(ui: &ui::SettingsState) -> Settings {
    let accepted_license_fingerprint: String = ui.get_accepted_license_fingerprint().into();
    let theme = ui.get_theme();
    Settings {
        accepted_license_fingerprint: (!accepted_license_fingerprint.is_empty())
            .then_some(accepted_license_fingerprint),
        theme,
    }
}

pub fn save(settings: &Settings) -> Result<(), std::io::Error> {
    let json = serde_json::to_string(settings).unwrap();
    if let Some(home_dir) = dirs::home_dir() {
        let dir = home_dir.join(".sed_manager");
        fs::create_dir_all(&dir)?;
        let file_path = dir.join("settings.json");
        let mut file = fs::OpenOptions::new().create(true).truncate(true).write(true).open(&file_path)?;
        file.write_all(json.as_bytes())
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    }
}

pub fn load() -> Result<Settings, std::io::Error> {
    if let Some(home_dir) = dirs::home_dir() {
        let dir = home_dir.join(".sed_manager");
        let file_path = dir.join("settings.json");
        let mut file = fs::OpenOptions::new().read(true).open(&file_path)?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;
        serde_json::from_str(&json).map_err(|_| std::io::ErrorKind::InvalidData.into())
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    }
}

fn make_none<T>() -> Option<T> {
    None
}

fn default_theme() -> ui::Theme {
    ui::Theme::System
}
