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
#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    /// The command.
    pub command: String,
    /// The command parameters.
    /// The default value is `default`.
    pub parameter: String,
    /// The command type.
    /// The default value is `command`.
    pub command_type: String,
}

impl Default for CommandRequest {
    fn default() -> Self {
        Self {
            command: String::default(),
            parameter: "default".into(),
            command_type: "command".into(),
        }
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
