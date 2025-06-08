use std::{borrow::Cow, fmt::Display, sync::LazyLock};

use regex::Regex;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct ConditionExpression<'a> {
    pub key: &'a str,
    operator: &'a str,
    value: &'a str,
}

impl Display for ConditionExpression<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.key, self.operator, self.value)
    }
}

impl<'a> TryFrom<&'a str> for ConditionExpression<'a> {
    type Error = anyhow::Error;

    fn try_from(condition: &'a str) -> Result<Self, Self::Error> {
        const RE_PAT: &str = r"^([a-zA-Z]+)(\s*(=)\s*([a-zA-Z0-9]+))?$";
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(RE_PAT).unwrap());
        if let Some(captures) = RE.captures(condition) {
            return Ok(ConditionExpression {
                key: captures.get(1).map_or_else(|| "", |m| m.as_str()),
                operator: captures.get(3).map_or_else(|| "", |m| m.as_str()),
                value: captures.get(4).map_or_else(|| "", |m| m.as_str()),
            });
        }
        Err(anyhow::anyhow!(r#"Not a valid expression "{condition}""#))
    }
}

impl ConditionExpression<'_> {
    pub fn evaluate(&self, value: &serde_json::Value) -> anyhow::Result<bool> {
        let value_str: Cow<'_, str> = match value {
            serde_json::Value::Bool(b) => {
                if self.operator.is_empty() {
                    log::debug!("evaluate: bool {b}");
                    return Ok(*b);
                }
                b.to_string().into()
            }
            serde_json::Value::String(str) => str.into(),
            _ => value.to_string().into(),
        };
        if self.operator == "=" {
            let result = value_str == self.value;
            log::debug!(r#"evaluate: "{value_str}" = "{}" -> {result}"#, self.value);
            return Ok(result);
        }
        anyhow::bail!("Unsupported condition {self} for {value}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(str: &str) -> anyhow::Result<ConditionExpression> {
        ConditionExpression::try_from(str)
    }

    fn from_key(key: &str) -> ConditionExpression {
        ConditionExpression {
            key,
            ..Default::default()
        }
    }

    fn from_strs<'a>(key: &'a str, operator: &'a str, value: &'a str) -> ConditionExpression<'a> {
        ConditionExpression {
            key,
            operator,
            value,
        }
    }

    #[test]
    fn parse_condition() -> anyhow::Result<()> {
        assert_eq!(parse("a")?, from_key("a"));
        assert_eq!(parse("a=b")?, from_strs("a", "=", "b"));
        assert_eq!(parse("a = b")?, from_strs("a", "=", "b"));
        assert!(parse("a=").is_err());
        assert!(parse("1=a").is_err());
        assert_eq!(parse("a=12")?, from_strs("a", "=", "12"));
        assert_eq!(parse("aZ=xZ2")?, from_strs("aZ", "=", "xZ2"));
        Ok(())
    }

    fn evaluate(expr: &str, value: impl serde::Serialize) -> anyhow::Result<bool> {
        ConditionExpression::try_from(expr)?.evaluate(&serde_json::json!(value))
    }

    #[test]
    fn evaluate_bool() -> anyhow::Result<()> {
        assert!(evaluate("a", true)?);
        assert!(!(evaluate("a", false)?));
        assert!(evaluate("a=true", true)?);
        assert!(!(evaluate("a=true", false)?));
        assert!(evaluate("a=false", false)?);
        Ok(())
    }

    #[test]
    fn evaluate_str() -> anyhow::Result<()> {
        assert!(evaluate("a", "on").is_err());
        assert!(evaluate("a=on", "on")?);
        assert!(!(evaluate("a=on", "off")?));
        Ok(())
    }

    #[test]
    fn evaluate_num() -> anyhow::Result<()> {
        assert!(evaluate("a", 123).is_err());
        assert!(evaluate("a=123", 123)?);
        assert!(!(evaluate("a=123", 124)?));
        Ok(())
    }
}
