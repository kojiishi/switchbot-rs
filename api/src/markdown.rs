use std::{fmt::Display, sync::LazyLock};

use regex::Regex;

/// Represents a simple Markdown.
///
/// # Examples
/// ```
/// # use switchbot_api::Markdown;
/// assert_eq!(Markdown::new("a<br>b").to_string(), "a\nb");
/// ```
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(from = "String")]
pub struct Markdown {
    markdown: String,
}

impl From<String> for Markdown {
    fn from(markdown: String) -> Self {
        Self { markdown }
    }
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

    fn plain_text(&self) -> String {
        const RE_BR_PAT: &str = r"(?i)<br\s*/?>";
        static RE_BR: LazyLock<Regex> = LazyLock::new(|| Regex::new(RE_BR_PAT).unwrap());
        RE_BR.replace_all(&self.markdown, "\n").into()
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
}
