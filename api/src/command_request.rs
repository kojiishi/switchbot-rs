/// A command request to send to the [SwitchBot API].
///
/// For more details of each field, please refer to the [SwitchBot
/// documentation about device control commands][send-device-control-commands].
///
/// # Examples
/// ```
/// # use switchbot_api::CommandRequest;
/// let command = CommandRequest {
///     command: "turnOn".into(),
///     ..Default::default()
/// };
/// ```
///
/// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
/// [send-device-control-commands]: https://github.com/OpenWonderLabs/SwitchBotAPI/blob/main/README.md#send-device-control-commands
#[derive(Debug, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    /// The command.
    pub command: String,

    /// The command parameters.
    #[serde(skip_serializing_if = "CommandRequest::can_omit_parameter")]
    pub parameter: String,

    /// The command type.
    #[serde(skip_serializing_if = "CommandRequest::can_omit_command_type")]
    pub command_type: String,
}

impl CommandRequest {
    const DEFAULT_PARAMETER: &str = "default";
    const DEFAULT_COMMAND_TYPE: &str = "command";

    fn can_omit_parameter(str: &str) -> bool {
        str.is_empty() || str == Self::DEFAULT_PARAMETER
    }

    fn can_omit_command_type(str: &str) -> bool {
        str.is_empty() || str == Self::DEFAULT_COMMAND_TYPE
    }
}

impl From<&str> for CommandRequest {
    /// Parse a string into a [`CommandRequest`].
    /// Please see the [`switchbot-cli` document] for the syntax.
    ///
    /// [`switchbot-cli` document]: https://github.com/kojiishi/switchbot-rs/tree/main/cli#command
    /// ```
    /// # use switchbot_api::CommandRequest;
    /// assert_eq!(
    ///     CommandRequest::from("turnOn"),
    ///     CommandRequest {
    ///         command: "turnOn".into(),
    ///         ..Default::default()
    ///     }
    /// );
    /// assert_eq!(
    ///     CommandRequest::from("turnOn:parameter:colon/slash"),
    ///     CommandRequest {
    ///         command: "turnOn".into(),
    ///         parameter: "parameter:colon/slash".into(),
    ///         ..Default::default()
    ///     }
    /// );
    /// assert_eq!(
    ///     CommandRequest::from("customize/turnOn"),
    ///     CommandRequest {
    ///         command: "turnOn".into(),
    ///         command_type: "customize".into(),
    ///         ..Default::default()
    ///     }
    /// );
    /// assert_eq!(
    ///     CommandRequest::from("customize/turnOn:parameter:colon/slash"),
    ///     CommandRequest {
    ///         command: "turnOn".into(),
    ///         command_type: "customize".into(),
    ///         parameter: "parameter:colon/slash".into(),
    ///     }
    /// );
    /// ```
    fn from(mut text: &str) -> Self {
        let mut command = CommandRequest::default();
        if let Some((name, parameter)) = text.split_once(':') {
            command.parameter = parameter.into();
            text = name;
        }
        if let Some((command_type, name)) = text.split_once('/') {
            command.command_type = command_type.into();
            text = name;
        }
        command.command = text.into();
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_all() {
        let all = CommandRequest {
            command: "test_command".into(),
            parameter: "param".into(),
            command_type: "type".into(),
        };
        assert_eq!(
            serde_json::to_string(&all).unwrap(),
            r#"{"command":"test_command","parameter":"param","commandType":"type"}"#
        );
    }

    #[test]
    fn serialize_default() {
        let param_type_default = CommandRequest {
            command: "test_command".into(),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&param_type_default).unwrap(),
            r#"{"command":"test_command"}"#
        );
    }

    #[test]
    fn serialize_default_str() {
        let param_type_default = CommandRequest {
            command: "test_command".into(),
            parameter: CommandRequest::DEFAULT_PARAMETER.into(),
            command_type: CommandRequest::DEFAULT_COMMAND_TYPE.into(),
        };
        assert_eq!(
            serde_json::to_string(&param_type_default).unwrap(),
            r#"{"command":"test_command"}"#
        );
    }

    #[test]
    fn serialize_empty() {
        let param_type_empty = CommandRequest {
            command: "test_command".into(),
            parameter: String::default(),
            command_type: String::default(),
        };
        assert_eq!(
            serde_json::to_string(&param_type_empty).unwrap(),
            r#"{"command":"test_command"}"#
        );
    }

    #[test]
    fn serialize_param() {
        let with_param = CommandRequest {
            command: "test_command".into(),
            parameter: "param".into(),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&with_param).unwrap(),
            r#"{"command":"test_command","parameter":"param"}"#
        );
    }

    #[test]
    fn serialize_type() {
        let with_type = CommandRequest {
            command: "test_command".into(),
            command_type: "type".into(),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&with_type).unwrap(),
            r#"{"command":"test_command","commandType":"type"}"#
        );
    }
}
