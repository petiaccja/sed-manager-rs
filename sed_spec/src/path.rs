#[repr(transparent)]
pub struct Path {
    path: str,
}

impl Path {
    pub fn new(s: &str) -> &Self {
        unsafe { &*(s.as_ref() as *const str as *const Path) }
    }

    pub fn security_provider(&self) -> Option<&str> {
        let first_sep = self.path.rfind("::")?;
        let second_sep = self.path[..first_sep].rfind("::")?;
        Some(&self.path[..second_sep])
    }

    pub fn table(&self) -> Option<&str> {
        let mut it = self.path.rsplit("::");
        it.next()?;
        it.next()
    }

    pub fn object(&self) -> &str {
        self.path.rsplit("::").next().unwrap_or("")
    }
}

impl AsRef<Path> for str {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<Path> for String {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

impl From<&Path> for String {
    fn from(value: &Path) -> Self {
        value.path.into()
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case("", None, None, "")]
    #[case("Anybody", None, None, "Anybody")]
    #[case("::Anybody", None, Some(""), "Anybody")]
    #[case("Authority::Anybody", None, Some("Authority"), "Anybody")]
    #[case("::Authority::Anybody", Some(""), Some("Authority"), "Anybody")]
    #[case("Admin::Authority::Anybody", Some("Admin"), Some("Authority"), "Anybody")]
    #[case("::Admin::Authority::Anybody", Some("::Admin"), Some("Authority"), "Anybody")]
    #[case("Garbage::Admin::Authority::Anybody", Some("Garbage::Admin"), Some("Authority"), "Anybody")]
    fn components(
        #[case] path: impl AsRef<Path>,
        #[case] sp: Option<&str>,
        #[case] table: Option<&str>,
        #[case] object: &str,
    ) {
        let path = path.as_ref();
        assert_eq!(path.security_provider(), sp);
        assert_eq!(path.table(), table);
        assert_eq!(path.object(), object);
    }
}
