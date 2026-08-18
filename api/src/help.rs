use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
};

use crate::{CommandRequest, Device, Markdown};

/// Human-readable description of a [`CommandRequest`].
///
/// Please see [`Help::command_helps()`] for how to get this struct.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct CommandHelp {
    command: CommandRequest,
    description: Markdown,
}

impl CommandHelp {
    fn empty_vec() -> &'static Vec<CommandHelp> {
        static EMPTY: Vec<CommandHelp> = Vec::new();
        &EMPTY
    }

    /// The [`CommandRequest`].
    /// Note that this may contain human-readable text
    /// and may not be able to send to the SwitchBot API directly.
    pub fn command(&self) -> &CommandRequest {
        &self.command
    }

    /// The human-readable description of the [`command()`][CommandHelp::command()].
    pub fn description(&self) -> &Markdown {
        &self.description
    }
}

impl Display for CommandHelp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command)?;
        for description in self.description.to_string().split('\n') {
            write!(f, "\n    {description}")?;
        }
        Ok(())
    }
}

/// Load and parse the documentations at the [SwitchBot API].
///
/// Please see [`Help::command_helps()`] for an example.
///
/// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
#[derive(Default, serde::Deserialize)]
pub struct Help {
    commands: HashMap<String, Vec<CommandHelp>>,
    commands_ir: HashMap<String, Vec<CommandHelp>>,
    device_type_aliases: HashMap<String, Vec<String>>,
}

impl Debug for Help {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "commands:")?;
        self.fmt_commands(&self.commands, f)?;
        writeln!(f, "commands (IR):")?;
        self.fmt_commands(&self.commands_ir, f)?;
        writeln!(f, "aliases:")?;
        for (device_type, aliases) in &self.device_type_aliases {
            writeln!(f, "- {device_type} -> {aliases:?}")?;
        }
        Ok(())
    }
}

impl Help {
    /// Loads the documentations from the [SwitchBot API] local data file.
    ///
    /// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
    pub async fn load() -> anyhow::Result<Self> {
        let json_str = include_str!("../../data/help_data.json");
        let help: Self = serde_json::from_str(json_str)?;
        Ok(help)
    }

    /// Adds a device type alias.
    #[cfg(test)]
    fn add_device_type_alias(&mut self, device_type: String, device_name: String) {
        let aliases = self.device_type_aliases.entry(device_type).or_default();
        if !aliases.contains(&device_name) {
            aliases.push(device_name);
        }
    }

    /// Get a list of [`CommandHelp`] for a [`Device`].
    /// Returns an empty `Vec` if no [`CommandHelp`]s are found.
    ///
    /// # Examples
    /// ```no_run
    /// # use switchbot_api::{Device, Help};
    /// # async fn help(device: &Device) -> anyhow::Result<()> {
    /// let help = Help::load().await?;
    /// let command_helps = help.command_helps(device);
    /// for command_help in command_helps {
    ///   println!("{}", command_help);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn command_helps(&self, device: &Device) -> &Vec<CommandHelp> {
        if device.is_remote() {
            return self.command_helps_by_remote_type(device.remote_type());
        }
        self.command_helps_by_device_type(device.device_type())
    }

    fn command_helps_by_device_type(&self, device_type: &str) -> &Vec<CommandHelp> {
        if let Some(commands) = self.commands.get(device_type) {
            return commands;
        }
        if let Some(aliases) = self.device_type_aliases.get(device_type) {
            for alias in aliases {
                if let Some(commands) = self.commands.get(alias) {
                    return commands;
                }
            }
        }
        CommandHelp::empty_vec()
    }

    fn command_helps_by_remote_type(&self, remote_type: &str) -> &Vec<CommandHelp> {
        if let Some(commands) = self.commands_ir.get(remote_type) {
            return commands;
        }
        // Some remotes have a "DIY " prefix. Try by removing it.
        if let Some(remote_type) = remote_type.strip_prefix("DIY ")
            && let Some(commands) = self.commands_ir.get(remote_type)
        {
            return commands;
        }
        CommandHelp::empty_vec()
    }

    fn fmt_commands(
        &self,
        commands: &HashMap<String, Vec<CommandHelp>>,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        for (device_type, helps) in commands {
            writeln!(f, "* {device_type}")?;
            for help in helps {
                writeln!(f, "  - {help}")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_aliases() {
        let mut help = Help::default();
        help.commands.insert(
            "TargetDevice".into(),
            vec![CommandHelp {
                command: CommandRequest::default(),
                description: Markdown::new("test"),
            }],
        );
        help.add_device_type_alias("AliasType".into(), "NonExistentDevice".into());
        help.add_device_type_alias("AliasType".into(), "TargetDevice".into());
        let helps = help.command_helps_by_device_type("AliasType");
        assert_eq!(helps.len(), 1);
    }
}
