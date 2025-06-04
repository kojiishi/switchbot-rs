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
#[derive(Debug, serde::Serialize)]
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
