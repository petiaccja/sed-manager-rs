//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sha2::{Digest as _, Sha256};

const LICENSE: &str = include_str!("../../LICENSE.md");

pub fn get_plain_license() -> String {
    remove_markdown_directives(LICENSE)
}

pub fn get_license_fingerprint() -> String {
    let mut hex = String::new();
    for byte in Sha256::digest(LICENSE) {
        hex += &format!("{byte:02x}");
    }
    hex
}

fn remove_markdown_directives(text: &str) -> String {
    let re_first_heading = regex::Regex::new(r"#+\s+").expect("invalid regex");
    let re_heading = regex::Regex::new(r"\n#+\s+").expect("invalid regex");
    let re_bold_italic = regex::Regex::new(r"([^\\])\*+").expect("invalid regex");
    let re_escapes = regex::Regex::new(r"\\(.)").expect("invalid regex");
    let re_links = regex::Regex::new(r"\[(.*)\]\((.*)\)").expect("invalid regex");
    let text = if re_first_heading.find(text).is_some_and(|m| m.start() == 0) {
        re_first_heading.replace(text, "")
    } else {
        text.into()
    };
    let text = re_heading.replace_all(&text, "\n");
    let text = re_bold_italic.replace_all(&text, "$1");
    let text = re_escapes.replace_all(&text, "$1");
    let text = re_links.replace_all(&text, "$1");
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn markdown_remove_link() {
        let input = r"some [alternate text](#ref) here";
        let expected = "some alternate text here";
        assert_eq!(remove_markdown_directives(input), expected);
    }
}
