//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use slint::ComponentHandle as _;
use std::fs;
use std::io::{Read, Write};
use std::rc::Rc;

use crate::frontend::Frontend;
use crate::license::get_license_fingerprint;
use crate::ui;
use crate::utility::PeekCell;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde(default = "make_none")]
    pub accepted_license_fingerprint: Option<String>,
}

pub fn set_callbacks(settings: Rc<PeekCell<Settings>>, frontend: Frontend) {
    frontend.with(move |window| {
        let settings_state = window.global::<ui::SettingsState>();
        settings_state.on_accept_license(move || {
            settings.peek_mut(|settings| {
                settings.accepted_license_fingerprint = Some(get_license_fingerprint());
            });
        });
    });
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
