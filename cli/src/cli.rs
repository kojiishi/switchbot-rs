use switchbot_api::{CommandRequest, Device, SwitchBot};

use crate::{Args, UserInput};

#[derive(Debug, Default)]
pub struct Cli {
    args: Args,
    switch_bot: SwitchBot,
    current_device_index: Option<usize>,
}

impl Cli {
    pub fn new_from_args() -> Self {
        Self {
            args: Args::new_from_args(),
            ..Default::default()
        }
    }

    fn has_current_device(&self) -> bool {
        self.current_device_index.is_some()
    }

    fn current_device(&self) -> Option<&Device> {
        if let Some(index) = self.current_device_index {
            return self.switch_bot.devices().get(index);
        }
        None
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        if !self.args.alias_updates.is_empty() {
            self.args.print_aliases();
            self.args.save()?;
            return Ok(());
        }

        self.switch_bot = self.args.create_switch_bot()?;
        self.switch_bot.load_devices().await?;

        if !self.args.commands.is_empty() {
            self.execute_commands(&self.args.commands.clone()).await?;
            self.args.save()?;
            return Ok(());
        }

        self.run_interactive().await?;

        self.args.save()?;
        Ok(())
    }

    async fn run_interactive(&mut self) -> anyhow::Result<()> {
        let mut input = UserInput::new();
        loop {
            if let Some(device) = self.current_device() {
                println!("{device:#}");
                input.set_prompt("Command> ");
            } else {
                self.print_devices();
                input.set_prompt("Device> ");
            }

            let input_text = input.read_line()?;
            match input_text {
                "q" => break,
                "" => {
                    if self.has_current_device() {
                        self.current_device_index = None;
                        continue;
                    }
                    break;
                }
                _ => {
                    if let Err(error) = self.execute_command(input_text).await {
                        log::error!("{error}");
                    }
                }
            }
        }
        Ok(())
    }

    fn print_devices(&self) {
        for (i, device) in self.switch_bot.devices().iter().enumerate() {
            println!("{}: {device}", i + 1);
        }
    }

    async fn execute_commands(&mut self, list: &[String]) -> anyhow::Result<()> {
        for command in list {
            self.execute_command(command).await?;
        }
        Ok(())
    }

    async fn execute_command(&mut self, text: &str) -> anyhow::Result<()> {
        if self.set_current_device(text).is_ok() {
            return Ok(());
        }
        if let Some(device) = self.current_device() {
            let command = self.parse_command(text);
            device.command(&command).await?;
            return Ok(());
        }
        self.set_current_device(text)?;
        Ok(())
    }

    fn set_current_device(&mut self, value: &str) -> anyhow::Result<()> {
        self.current_device_index = Some(self.parse_device_index(value)?);
        log::debug!("current_device_index={:?}", self.current_device_index);
        Ok(())
    }

    fn parse_device_index(&self, value: &str) -> anyhow::Result<usize> {
        if let Ok(number) = value.parse::<usize>() {
            if number > 0 && number <= self.switch_bot.devices().len() {
                return Ok(number - 1);
            }
        }
        self.switch_bot
            .devices()
            .index_by_device_id(value)
            .ok_or_else(|| anyhow::anyhow!("Not a valid device: \"{value}\""))
    }

    fn parse_command(&self, text: &str) -> CommandRequest {
        if let Some(alias) = self.args.aliases.get(text) {
            log::debug!(r#"Command alias: "{text}" -> "{alias}""#);
            return self.parse_command_no_alias(alias);
        }
        self.parse_command_no_alias(text)
    }

    fn parse_command_no_alias(&self, mut text: &str) -> CommandRequest {
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
