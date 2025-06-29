use std::{fmt::Display, sync::LazyLock};

use regex::Regex;

/// Represents a simple Markdown.
///
/// # Examples
/// ```
/// # use switchbot_api::Markdown;
/// assert_eq!(Markdown::new("a<br>b").to_string(), "a\nb");
/// ```
#[derive(Clone, Debug, Default)]
pub struct Markdown {
    markdown: String,
}

impl Markdown {
    pub fn new(markdown: &str) -> Self {
        Self {
            markdown: markdown.to_string(),
        }
    }

    /// The original Markdown.
    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    pub(crate) fn em(text: &str) -> Option<&str> {
        const RE_EM_PAT: &str = r"\*([^*]+)\*";
        static RE_EM: LazyLock<Regex> = LazyLock::new(|| Regex::new(RE_EM_PAT).unwrap());
        if let Some(captures) = RE_EM.captures(text) {
            return captures.get(1).map(|m| m.as_str());
        }
        None
    }

    fn plain_text(&self) -> String {
        const RE_BR_PAT: &str = r"(?i)<br\s*/?>";
        static RE_BR: LazyLock<Regex> = LazyLock::new(|| Regex::new(RE_BR_PAT).unwrap());
        RE_BR.replace_all(&self.markdown, "\n").into()
    }

    pub(crate) fn table_columns(line: &str) -> Option<Vec<&str>> {
        if line.starts_with('|') && line.ends_with('|') {
            let columns = line
                .split_terminator('|')
                .skip(1)
                .map(|s| s.trim())
                .collect::<Vec<&str>>();
            return Some(columns);
        }
        None
    }
}

impl Display for Markdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.plain_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn em() {
        assert_eq!(Markdown::em("*a*"), Some("a"));
        assert_eq!(Markdown::em("x*a*x"), Some("a"));

        assert_eq!(Markdown::em("a"), None);
        assert_eq!(Markdown::em("*a"), None);
        assert_eq!(Markdown::em("a*"), None);

        assert_eq!(
            Markdown::em("device type. *Hub*, *Hub Plus*, *Hub Mini*, *Hub 2* or *Hub 3*."),
            Some("Hub")
        );
    }

    fn to_plain_text(markdown: &str) -> String {
        Markdown::new(markdown).plain_text()
    }

    #[test]
    fn plain_text() {
        assert_eq!(to_plain_text(""), "");

        assert_eq!(to_plain_text("<br>"), "\n");
        assert_eq!(to_plain_text("<br/>"), "\n");
        assert_eq!(to_plain_text("<br />"), "\n");
        assert_eq!(to_plain_text("<BR>"), "\n");

        assert_eq!(to_plain_text("a<br>b"), "a\nb");

        assert_eq!(to_plain_text("a<br>b<br>c"), "a\nb\nc");
    }

    fn to_table_columns(line: &str) -> Option<Vec<&str>> {
        Markdown::table_columns(line)
    }

    #[test]
    fn table_columns() {
        assert_eq!(to_table_columns("1|2|3"), None);
        assert_eq!(to_table_columns("|1|2|3|"), Some(vec!["1", "2", "3"]));
        assert_eq!(to_table_columns("| 1 | 2 | 3 |"), Some(vec!["1", "2", "3"]));
    }
}
