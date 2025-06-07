use std::{fmt::Display, sync::LazyLock};

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
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^([a-zA-Z]+)\s*((=)\s*([a-zA-Z0-9]+))?$").unwrap());
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
        match value {
            serde_json::Value::Bool(b) => {
                if self.operator.is_empty() {
                    log::debug!("eval_condition: bool {b}");
                    return Ok(*b);
                }
            }
            serde_json::Value::String(str) => {
                if self.operator == "=" {
                    let result = str == self.value;
                    log::debug!("eval_condition: str {str} == {} -> {result}", self.value);
                    return Ok(result);
                }
            }
            _ => {
                if self.operator == "=" {
                    let value = value.to_string();
                    let result = value == self.value;
                    log::debug!("eval_condition: {value} == {} -> {result}", self.value);
                    return Ok(result);
                }
            }
        }
        anyhow::bail!("Unsupported condition {self} for {value}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_condition() {
        assert_eq!(
            ConditionExpression::try_from("a").unwrap(),
            ConditionExpression {
                key: "a",
                ..Default::default()
            }
        );
        assert_eq!(
            ConditionExpression::try_from("a=b").unwrap(),
            ConditionExpression {
                key: "a",
                operator: "=",
                value: "b"
            }
        );
        assert_eq!(
            ConditionExpression::try_from("a = b").unwrap(),
            ConditionExpression {
                key: "a",
                operator: "=",
                value: "b"
            }
        );
        assert!(ConditionExpression::try_from("a=").is_err());
        assert_eq!(
            ConditionExpression::try_from("camelCase=camelCase2").unwrap(),
            ConditionExpression {
                key: "camelCase",
                operator: "=",
                value: "camelCase2"
            }
        );
    }
}
