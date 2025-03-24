use slint::ComponentHandle as _;
use std::fs;
use std::io::{Read, Write};
use std::rc::Rc;

use crate::frontend::Frontend;
use crate::ui;
use crate::utility::PeekCell;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    #[serde(default = "make_true")]
    pub license_accepted: bool,
}

pub fn set_callbacks(settings: Rc<PeekCell<Settings>>, frontend: Frontend) {
    frontend.with(move |window| {
        let settings_state = window.global::<ui::SettingsState>();
        settings_state.on_accept_license(move || {
            settings.peek_mut(|settings| {
                settings.license_accepted = true;
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

pub fn remove_markdown_directives(text: &str) -> String {
    let re_first_heading = regex::Regex::new(r"#+\s+").expect("invalid regex");
    let re_heading = regex::Regex::new(r"\n#+\s+").expect("invalid regex");
    let re_bold_italic = regex::Regex::new(r"([^\\])\*+").expect("invalid regex");
    let re_escapes = regex::Regex::new(r"\\(.)").expect("invalid regex");
    let text = if re_first_heading.find(text).is_some_and(|m| m.start() == 0) {
        re_first_heading.replace(text, "")
    } else {
        text.into()
    };
    let text = re_heading.replace_all(&text, "\n");
    let text = re_bold_italic.replace_all(&text, "$1");
    let text = re_escapes.replace_all(&text, "$1");
    text.to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Settings { license_accepted: false }
    }
}

fn make_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use crate::settings::remove_markdown_directives;

    #[test]
    fn markdown_remove_heading_first() {
        let input = "## heading";
        let expected = "heading";
        assert_eq!(remove_markdown_directives(input), expected);
    }

    #[test]
    fn markdown_remove_heading_newline() {
        let input = "text \n## heading";
        let expected = "text \nheading";
        assert_eq!(remove_markdown_directives(input), expected);
    }

    #[test]
    fn markdown_remove_headings_line_middle() {
        let input = "asd ## heading";
        let expected = "asd ## heading";
        assert_eq!(remove_markdown_directives(input), expected);
    }

    #[test]
    fn markdown_remove_bold() {
        let input = "some **bold** \\*text";
        let expected = "some bold *text";
        assert_eq!(remove_markdown_directives(input), expected);
    }

    #[test]
    fn markdown_remove_italic() {
        let input = "some *italic* \\*text";
        let expected = "some italic *text";
        assert_eq!(remove_markdown_directives(input), expected);
    }

    #[test]
    fn markdown_remove_escape() {
        let input = r"some \x \*";
        let expected = "some x *";
        assert_eq!(remove_markdown_directives(input), expected);
    }
}
