use switchbot_api::{CommandRequest, Device, DeviceList, SwitchBot};

use crate::{Args, UserInput};

#[derive(Debug, Default)]
pub struct Cli {
    args: Args,
    switch_bot: SwitchBot,
    current_device_indexes: Vec<usize>,
}

impl Cli {
    pub fn new_from_args() -> Self {
        Self {
            args: Args::new_from_args(),
            ..Default::default()
        }
    }

    #[cfg(test)]
    fn new_for_test(n_devices: usize) -> Self {
        Self {
            switch_bot: SwitchBot::new_for_test(n_devices),
            ..Default::default()
        }
    }

    fn devices(&self) -> &DeviceList {
        self.switch_bot.devices()
    }

    fn has_current_device(&self) -> bool {
        !self.current_device_indexes.is_empty()
    }

    fn current_devices(&self) -> Vec<&Device> {
        self.current_device_indexes
            .iter()
            .map(|&index| &self.devices()[index])
            .collect() // Collect the iterator into a Vec
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
            self.execute_args(&self.args.commands.clone()).await?;
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
            self.print_devices();
            input.set_prompt(if self.has_current_device() {
                "Command> "
            } else {
                "Device> "
            });

            let input_text = input.read_line()?;
            match input_text {
                "q" => break,
                "" => {
                    if self.has_current_device() {
                        self.current_device_indexes.clear();
                        continue;
                    }
                    break;
                }
                _ => {
                    if let Err(error) = self.execute(input_text).await {
                        log::error!("{error}");
                    }
                }
            }
        }
        Ok(())
    }

    fn print_devices(&self) {
        if self.current_device_indexes.is_empty() {
            for (i, device) in self.devices().iter().enumerate() {
                println!("{}: {device}", i + 1);
            }
            return;
        }

        if self.current_device_indexes.len() >= 2 {
            for (i, device) in self
                .current_device_indexes
                .iter()
                .map(|i| (i, &self.devices()[*i]))
            {
                println!("{}: {device}", i + 1);
            }
            return;
        }

        let device = self.current_devices()[0];
        println!("{device:#}");
    }

    async fn execute_args(&mut self, list: &[String]) -> anyhow::Result<()> {
        for command in list {
            self.execute(command).await?;
        }
        Ok(())
    }

    async fn execute(&mut self, mut text: &str) -> anyhow::Result<()> {
        let alias_result: String;
        if let Some(alias) = self.args.aliases.get(text) {
            log::debug!(r#"alias: "{text}" -> "{alias}""#);
            alias_result = alias.clone();
            text = &alias_result;
        }

        let set_device_result = self.set_current_devices(text);
        if set_device_result.is_ok() {
            return Ok(());
        }
        if self.has_current_device() {
            self.execute_command(&CommandRequest::from(text)).await?;
            return Ok(());
        }
        Err(set_device_result.unwrap_err())
    }

    fn set_current_devices(&mut self, text: &str) -> anyhow::Result<()> {
        self.current_device_indexes = self.parse_device_indexes(text)?;
        log::debug!("current_device_indexes={:?}", self.current_device_indexes);
        Ok(())
    }

    fn parse_device_indexes(&self, value: &str) -> anyhow::Result<Vec<usize>> {
        let values = value.split(',');
        let mut indexes: Vec<usize> = Vec::new();
        for s in values {
            if let Some(alias) = self.args.aliases.get(s) {
                indexes.extend(self.parse_device_indexes(alias)?);
                continue;
            }
            indexes.push(self.parse_device_index(s)?);
        }
        indexes.sort();
        indexes.dedup();
        Ok(indexes)
    }

    fn parse_device_index(&self, value: &str) -> anyhow::Result<usize> {
        if let Ok(number) = value.parse::<usize>() {
            if number > 0 && number <= self.devices().len() {
                return Ok(number - 1);
            }
        }
        self.devices()
            .index_by_device_id(value)
            .ok_or_else(|| anyhow::anyhow!("Not a valid device: \"{value}\""))
    }

    async fn execute_command(&self, command: &CommandRequest) -> anyhow::Result<()> {
        let devices = self.current_devices();
        for device in devices {
            device.command(&command).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_device_indexes() {
        let cli = Cli::new_for_test(10);
        assert!(cli.parse_device_indexes("").is_err());
        assert_eq!(cli.parse_device_indexes("4").unwrap(), vec![3]);
        assert_eq!(cli.parse_device_indexes("device4").unwrap(), vec![3]);
        assert_eq!(cli.parse_device_indexes("2,4").unwrap(), vec![1, 3]);
        assert_eq!(cli.parse_device_indexes("2,device4").unwrap(), vec![1, 3]);
        // The result should be sorted.
        assert_eq!(cli.parse_device_indexes("4,2").unwrap(), vec![1, 3]);
        assert_eq!(cli.parse_device_indexes("device4,2").unwrap(), vec![1, 3]);
        // The result should be deduped.
        assert_eq!(cli.parse_device_indexes("2,4,2").unwrap(), vec![1, 3]);
    }

    #[test]
    fn parse_device_indexes_alias() {
        let mut cli = Cli::new_for_test(10);
        cli.args.aliases.insert("k".into(), "3,5".into());
        assert_eq!(cli.parse_device_indexes("k").unwrap(), vec![2, 4]);
        assert_eq!(cli.parse_device_indexes("1,k,4").unwrap(), vec![0, 2, 3, 4]);
        cli.args.aliases.insert("j".into(), "2,k".into());
        assert_eq!(
            cli.parse_device_indexes("1,j,4").unwrap(),
            vec![0, 1, 2, 3, 4]
        );
        assert_eq!(cli.parse_device_indexes("1,j,5").unwrap(), vec![0, 1, 2, 4]);
    }
}
