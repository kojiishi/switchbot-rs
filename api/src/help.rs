use std::{
    collections::HashMap,
    fmt::{Debug, Display, Formatter},
    io::{BufRead, BufReader},
    sync::LazyLock,
};

use crate::{CommandRequest, Device, Markdown};

/// Human-readable description of a [`CommandRequest`].
///
/// Please see [`Help::command_helps()`] for how to get this struct.
#[derive(Clone, Debug)]
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
            write!(f, "\n    {}", description)?;
        }
        Ok(())
    }
}

/// Load and parse the documentations at the [SwitchBot API].
///
/// Please see [`Help::command_helps()`] for an example.
///
/// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
#[derive(Default)]
pub struct Help {
    commands: HashMap<String, Vec<CommandHelp>>,
    commands_ir: HashMap<String, Vec<CommandHelp>>,
    device_name_by_type: HashMap<String, String>,
}

impl Debug for Help {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "commands:")?;
        self.fmt_commands(&self.commands, f)?;
        writeln!(f, "commands (IR):")?;
        self.fmt_commands(&self.commands_ir, f)?;
        writeln!(f, "aliases:")?;
        for (device_type, device_name) in &self.device_name_by_type {
            writeln!(f, "- {device_type} -> {device_name}")?;
        }
        Ok(())
    }
}

impl Help {
    /// Loads and parses the documentations from the [SwitchBot API].
    ///
    /// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
    pub async fn load() -> anyhow::Result<Self> {
        let mut loader = HelpLoader::default();
        loader.load().await?;
        Ok(loader.help)
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
        if let Some(alias) = self.device_name_by_type.get(device_type) {
            if let Some(commands) = self.commands.get(alias) {
                return commands;
            }
        }
        CommandHelp::empty_vec()
    }

    fn command_helps_by_remote_type(&self, remote_type: &str) -> &Vec<CommandHelp> {
        if let Some(commands) = self.commands_ir.get(remote_type) {
            return commands;
        }
        // Some remotes have a "DIY " prefix. Try by removing it.
        if let Some(remote_type) = remote_type.strip_prefix("DIY ") {
            if let Some(commands) = self.commands_ir.get(remote_type) {
                return commands;
            }
        }
        CommandHelp::empty_vec()
    }

    fn finalize(&mut self) {
        const OTHER_KEY: &str = "Others";
        if let Some(mut others) = self.commands_ir.remove(OTHER_KEY) {
            for help in &mut others {
                help.command.command_type = help.command.command_type.trim_matches('`').into();
            }
            for helps in self.commands_ir.values_mut() {
                for help in &others {
                    helps.push(help.clone());
                }
            }
        }

        const ALL_KEY: &str = "All home appliance types except Others";
        if let Some(all) = self.commands_ir.remove(ALL_KEY) {
            for helps in self.commands_ir.values_mut() {
                for (i, help) in all.iter().enumerate() {
                    helps.insert(i, help.clone());
                }
            }
        }
    }

    fn fmt_commands(
        &self,
        commands: &HashMap<String, Vec<CommandHelp>>,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        for (device_type, helps) in commands {
            writeln!(f, "* {device_type}")?;
            for help in helps {
                writeln!(f, "  - {}", help)?;
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
enum Section {
    #[default]
    Initial,
    Devices,
    Status,
    Commands,
    CommandsIR,
    Scenes,
}

impl Section {
    fn update(&mut self, line: &str) -> bool {
        static SECTIONS: LazyLock<HashMap<&str, Section>> = LazyLock::new(|| {
            HashMap::from([
                ("## Devices", Section::Devices),
                ("### Get device status", Section::Status),
                ("### Send device control commands", Section::Commands),
                (
                    "#### Command set for virtual infrared remote devices",
                    Section::CommandsIR,
                ),
                ("## Scenes", Section::Scenes),
            ])
        });
        if let Some(s) = SECTIONS.get(line) {
            log::debug!("section: {:?} -> {:?}", self, s);
            *self = *s;
            return true;
        }
        false
    }
}

#[derive(Debug, Default)]
struct HelpLoader {
    help: Help,
    section: Section,
    device_name: String,
    in_command_table: bool,
    command_device_type: String,
    command_helps: Vec<CommandHelp>,
}

impl HelpLoader {
    const URL: &str =
        "https://raw.githubusercontent.com/OpenWonderLabs/SwitchBotAPI/refs/heads/main/README.md";

    pub async fn load(&mut self) -> anyhow::Result<()> {
        let response = reqwest::get(Self::URL).await?.error_for_status()?;
        // let body = response.text().await?;
        // let reader = BufReader::new(body.as_bytes());
        let body = response.bytes().await?;
        let reader = BufReader::new(body.as_ref());
        self.read_lines(reader.lines())?;
        self.help.finalize();
        log::trace!("{:?}", self.help);
        Ok(())
    }

    fn read_lines(
        &mut self,
        lines: impl Iterator<Item = std::io::Result<String>>,
    ) -> anyhow::Result<()> {
        for line_result in lines {
            let line_str = line_result?;
            let line = line_str.trim();
            self.read_line(line)?;
        }
        Ok(())
    }

    fn read_line(&mut self, line: &str) -> anyhow::Result<()> {
        if self.section.update(line) {
            return Ok(());
        }
        match self.section {
            Section::Devices => {
                if self.update_device_type(line) {
                    return Ok(());
                }
                if !self.device_name.is_empty() {
                    if let Some(columns) = Markdown::table_columns(line) {
                        if columns[0] == "deviceType" {
                            if let Some(device_type) = Markdown::em(columns[2]) {
                                self.add_device_alias(device_type);
                            }
                        }
                    }
                }
            }
            Section::Commands | Section::CommandsIR => {
                if self.update_device_type(line) {
                    return Ok(());
                }
                if let Some(columns) = Markdown::table_columns(line) {
                    if !self.in_command_table {
                        if columns.len() == 5 && columns[0] == "deviceType" {
                            self.in_command_table = true;
                        }
                    } else if !columns[0].starts_with('-') {
                        if !columns[0].is_empty() && self.command_device_type != columns[0] {
                            self.flush_command_help();
                            log::trace!("{:?}: {:?}", self.section, columns[0]);
                            self.command_device_type = columns[0].into();
                        }
                        assert!(!self.command_device_type.is_empty());
                        let command = CommandRequest {
                            command_type: columns[1].into(),
                            command: columns[2].into(),
                            parameter: columns[3].into(),
                        };
                        let help = CommandHelp {
                            command,
                            description: Markdown::new(columns[4]),
                        };
                        self.command_helps.push(help);
                    }
                } else {
                    self.flush_command_help();
                    self.in_command_table = false;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn update_device_type(&mut self, line: &str) -> bool {
        if let Some(text) = line.strip_prefix("##### ") {
            self.device_name = text.trim().to_string();
            return true;
        }
        false
    }

    fn add_device_alias(&mut self, device_type: &str) {
        log::trace!("alias = {} -> {device_type}", self.device_name);
        if self.device_name == device_type {
            return;
        }
        self.help
            .device_name_by_type
            .insert(device_type.into(), self.device_name.clone());
    }

    fn flush_command_help(&mut self) {
        if self.command_device_type.is_empty() || self.command_helps.is_empty() {
            return;
        }
        let name = std::mem::take(&mut self.command_device_type);
        log::trace!("flush_command: {:?}: {:?}", self.section, name);
        let helps = std::mem::take(&mut self.command_helps);
        if self.section == Section::CommandsIR {
            let names: Vec<&str> = name.split(',').collect();
            if names.len() > 1 {
                for name in names {
                    self.add_command_help(name.trim().into(), helps.clone());
                }
                return;
            }
        }
        self.add_command_help(name, helps);
    }

    fn add_command_help(&mut self, mut name: String, helps: Vec<CommandHelp>) {
        if name == "Lock" && self.device_name == "Lock Pro" {
            // https://github.com/OpenWonderLabs/SwitchBotAPI/pull/413
            name = "Lock Pro".into();
        }
        let add_to = match self.section {
            Section::Commands => &mut self.help.commands,
            Section::CommandsIR => &mut self.help.commands_ir,
            _ => panic!("Unexpected section {:?}", self.section),
        };
        match add_to.entry(name) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(helps);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().extend(helps);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_update() {
        let mut section = Section::default();
        assert_eq!(section, Section::Initial);
        assert!(section.update("## Devices"));
        assert_eq!(section, Section::Devices);
    }
}
