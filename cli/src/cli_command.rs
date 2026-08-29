use std::borrow::Cow;

use switchbot_api::CommandRequest;
use winnow::{
    ModalResult, Parser,
    ascii::{space0, space1},
    combinator::{alt, eof, fail},
    token::{any, rest},
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CliCommand {
    // Global built-in commands
    AliasList,
    AliasSet {
        name: String,
        value: Option<String>,
    },
    Devices,

    // Device built-in commands
    Help,
    Status {
        key: Option<String>,
    },

    // Custom device command (e.g., turnOn, customize/button1, setMode:101)
    DeviceCommand(CommandRequest),

    // Conditional expressions
    If {
        separator: char,
        condition: String,
        then_command: String,
        else_command: String,
    },
}

impl CliCommand {
    pub(crate) fn requires_current_device(&self) -> bool {
        match self {
            CliCommand::Devices | CliCommand::AliasList | CliCommand::AliasSet { .. } => false,
            CliCommand::Help
            | CliCommand::Status { .. }
            | CliCommand::DeviceCommand(_)
            | CliCommand::If { .. } => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CliCommandLine {
    pub(crate) device_selector: Option<String>,
    pub(crate) command: Option<CliCommand>,
}

impl CliCommandLine {
    /// Parse command input, using a validator callback to resolve ambiguity of whether
    /// the first word is a device selector or the start of a command.
    pub(crate) fn parse(
        input: &str,
        is_valid_device_selector: impl Fn(&str) -> bool,
        expand_alias: impl for<'a> Fn(&'a str) -> Cow<'a, str>,
    ) -> anyhow::Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(Self {
                device_selector: None,
                command: None,
            });
        }

        // 1. Check if the entire input is a device selector
        if is_valid_device_selector(input) {
            return Ok(Self {
                device_selector: Some(input.to_string()),
                command: None,
            });
        }

        // 2. Check if the first word is a device selector
        if let Some(pos) = input.find(' ') {
            let first_word = &input[..pos];
            if is_valid_device_selector(first_word) {
                let rest_str = input[pos + 1..].trim_start();
                let expanded_rest = expand_alias(rest_str);
                let mut parser_input = expanded_rest.as_ref();
                let cmd = parse_command(&mut parser_input)
                    .map_err(|e| anyhow::anyhow!("Failed to parse command: {e}"))?;
                return Ok(Self {
                    device_selector: Some(first_word.to_string()),
                    command: Some(cmd),
                });
            }
        }

        // 3. Otherwise, treat the entire input as a command
        let mut parser_input = input;
        let cmd = parse_command(&mut parser_input)
            .map_err(|e| anyhow::anyhow!("Failed to parse command: {e}"))?;
        Ok(Self {
            device_selector: None,
            command: Some(cmd),
        })
    }
}

fn parse_command(input: &mut &str) -> ModalResult<CliCommand> {
    alt((
        parse_devices,
        parse_alias,
        parse_help,
        parse_status,
        parse_if,
        parse_device_command,
    ))
    .parse_next(input)
}

fn parse_devices(input: &mut &str) -> ModalResult<CliCommand> {
    ("devices", space0, eof).parse_next(input)?;
    Ok(CliCommand::Devices)
}

fn parse_alias(input: &mut &str) -> ModalResult<CliCommand> {
    "alias".parse_next(input)?;
    alt((
        (space0, eof).map(|_| CliCommand::AliasList),
        (space1, rest).map(|(_, val): (&str, &str)| {
            let val = val.trim();
            if val.is_empty() {
                CliCommand::AliasList
            } else if let Some((name, value)) = val.split_once('=') {
                let name = name.trim().to_string();
                let value = value.trim();
                let value = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
                CliCommand::AliasSet { name, value }
            } else {
                CliCommand::AliasSet {
                    name: val.to_string(),
                    value: None,
                }
            }
        }),
    ))
    .parse_next(input)
}

fn parse_help(input: &mut &str) -> ModalResult<CliCommand> {
    ("help", space0, eof).parse_next(input)?;
    Ok(CliCommand::Help)
}

fn parse_status(input: &mut &str) -> ModalResult<CliCommand> {
    "status".parse_next(input)?;
    alt((
        (space0, eof).map(|_| CliCommand::Status { key: None }),
        (".", rest).map(|(_, key): (&str, &str)| {
            let key = key.trim();
            CliCommand::Status {
                key: if key.is_empty() {
                    None
                } else {
                    Some(key.to_string())
                },
            }
        }),
    ))
    .parse_next(input)
}

fn parse_if(input: &mut &str) -> ModalResult<CliCommand> {
    "if".parse_next(input)?;
    let sep: char = any.parse_next(input)?;
    if sep.is_alphanumeric() {
        return fail.parse_next(input);
    }
    let rest_str: &str = rest.parse_next(input)?;
    let fields: Vec<&str> = rest_str.split_terminator(sep).collect();
    match fields.len() {
        2 => Ok(CliCommand::If {
            separator: sep,
            condition: fields[0].to_string(),
            then_command: fields[1].to_string(),
            else_command: String::new(),
        }),
        3 => Ok(CliCommand::If {
            separator: sep,
            condition: fields[0].to_string(),
            then_command: fields[1].to_string(),
            else_command: fields[2].to_string(),
        }),
        _ => fail.parse_next(input),
    }
}

fn parse_device_command(input: &mut &str) -> ModalResult<CliCommand> {
    let text = rest.parse_next(input)?;
    Ok(CliCommand::DeviceCommand(CommandRequest::from(text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_validator(s: &str) -> bool {
        // Simple logic for tests: if it's numeric or has comma, it's a device target
        s.chars().all(|c| c.is_ascii_digit() || c == ',')
    }

    #[test]
    fn parse_devices() {
        let res = CliCommandLine::parse("devices", dummy_validator, |s| Cow::Borrowed(s)).unwrap();
        assert_eq!(res.device_selector, None);
        assert_eq!(res.command, Some(CliCommand::Devices));
    }

    #[test]
    fn parse_alias() {
        let res = CliCommandLine::parse("alias", dummy_validator, |s| Cow::Borrowed(s)).unwrap();
        assert_eq!(res.command, Some(CliCommand::AliasList));

        let res =
            CliCommandLine::parse("alias a=b", dummy_validator, |s| Cow::Borrowed(s)).unwrap();
        assert_eq!(
            res.command,
            Some(CliCommand::AliasSet {
                name: "a".to_string(),
                value: Some("b".to_string())
            })
        );

        let res = CliCommandLine::parse("alias a", dummy_validator, |s| Cow::Borrowed(s)).unwrap();
        assert_eq!(
            res.command,
            Some(CliCommand::AliasSet {
                name: "a".to_string(),
                value: None
            })
        );
    }

    #[test]
    fn parse_status() {
        let res = CliCommandLine::parse("status", dummy_validator, |s| Cow::Borrowed(s)).unwrap();
        assert_eq!(res.command, Some(CliCommand::Status { key: None }));

        let res =
            CliCommandLine::parse("status.power", dummy_validator, |s| Cow::Borrowed(s)).unwrap();
        assert_eq!(
            res.command,
            Some(CliCommand::Status {
                key: Some("power".to_string())
            })
        );
    }

    #[test]
    fn parse_if() {
        let res =
            CliCommandLine::parse("if/power=on/off/on", dummy_validator, |s| Cow::Borrowed(s))
                .unwrap();
        assert_eq!(
            res.command,
            Some(CliCommand::If {
                separator: '/',
                condition: "power=on".to_string(),
                then_command: "off".to_string(),
                else_command: "on".to_string()
            })
        );
    }

    #[test]
    fn device_selection() {
        let res = CliCommandLine::parse("1,2", dummy_validator, |s| Cow::Borrowed(s)).unwrap();
        assert_eq!(res.device_selector, Some("1,2".to_string()));
        assert_eq!(res.command, None);

        let res =
            CliCommandLine::parse("1,2 turnOn", dummy_validator, |s| Cow::Borrowed(s)).unwrap();
        assert_eq!(res.device_selector, Some("1,2".to_string()));
        assert_eq!(
            res.command,
            Some(CliCommand::DeviceCommand(CommandRequest::from("turnOn")))
        );
    }

    fn dummy_expand(s: &str) -> Cow<'_, str> {
        if s == "on" {
            Cow::Owned("turnOn".to_string())
        } else {
            Cow::Borrowed(s)
        }
    }

    #[test]
    fn device_selection_with_alias() {
        let res = CliCommandLine::parse("1,2 on", dummy_validator, dummy_expand).unwrap();
        assert_eq!(res.device_selector, Some("1,2".to_string()));
        assert_eq!(
            res.command,
            Some(CliCommand::DeviceCommand(CommandRequest::from("turnOn")))
        );
    }
}
