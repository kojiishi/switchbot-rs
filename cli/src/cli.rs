use std::{
    collections::HashMap,
    future::Future,
    io::{Write, stdout},
    iter::zip,
};

use itertools::Itertools;
use switchbot_api::{Device, DeviceList, Help, SwitchBot};

use crate::{Args, CliCommand, CliCommandLine, MarkdownPrinter, UserInput};

#[derive(Debug, Default)]
pub struct Cli {
    args: Args,
    switch_bot: SwitchBot,
    current_device_indexes: Vec<usize>,
    is_current_devices_changed: bool,
    help: Option<Help>,
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

    fn num_current_devices(&self) -> usize {
        self.current_device_indexes.len()
    }

    fn current_devices_as<'a, T, F>(&'a self, f: F) -> impl Iterator<Item = T> + 'a
    where
        F: Fn(usize) -> T + 'a,
    {
        self.current_device_indexes
            .iter()
            .map(move |&index| f(index))
    }

    fn current_devices(&self) -> impl Iterator<Item = &Device> {
        self.current_devices_as(|index| &self.devices()[index])
    }

    fn current_devices_with_index(&self) -> impl Iterator<Item = (usize, &Device)> {
        self.current_devices_as(|index| (index, &self.devices()[index]))
    }

    fn first_current_device(&self) -> &Device {
        &self.devices()[self.current_device_indexes[0]]
    }

    async fn ensure_devices(&mut self) -> anyhow::Result<()> {
        if self.devices().is_empty() {
            self.switch_bot = self.args.create_switch_bot()?;
            self.switch_bot.load_devices().await?;
            log::debug!("ensure_devices: {} devices", self.devices().len());
        }
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.args.process()?;
        self.run_core().await?;
        self.args.save()?;
        Ok(())
    }

    async fn run_core(&mut self) -> anyhow::Result<()> {
        let mut is_interactive = true;
        if !self.args.alias_updates.is_empty() {
            self.args.aliases.print();
            is_interactive = false;
        }

        if !self.args.commands.is_empty() {
            self.ensure_devices().await?;
            self.execute_args(&self.args.commands.clone()).await?;
        } else if is_interactive {
            self.ensure_devices().await?;
            self.run_interactive().await?;
        }
        Ok(())
    }

    async fn run_interactive(&mut self) -> anyhow::Result<()> {
        let mut input = UserInput::new();
        self.print_devices()?;
        loop {
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
                        self.print_devices()?;
                        continue;
                    }
                    break;
                }
                _ => match self.execute(input_text).await {
                    Ok(_) => {
                        if self.is_current_devices_changed {
                            self.is_current_devices_changed = false;
                            self.print_devices()?;
                        }
                    }
                    Err(error) => log::error!("{error}"),
                },
            }
        }
        Ok(())
    }

    fn print_devices(&self) -> anyhow::Result<()> {
        if !self.has_current_device() {
            return self.print_all_devices();
        }

        if self.current_device_indexes.len() >= 2 {
            return self.print_devices_with_index(self.current_devices_with_index());
        }

        let device = self.first_current_device();
        print!("{device:#}");
        Ok(())
    }

    fn print_all_devices(&self) -> anyhow::Result<()> {
        self.print_devices_with_index(self.devices().iter().enumerate())
    }

    fn print_devices_with_index<'a>(
        &self,
        iter: impl IntoIterator<Item = (usize, &'a Device)>,
    ) -> anyhow::Result<()> {
        let reverse_aliases = self.args.aliases.reverse_map();
        let mut out = MarkdownPrinter::new();
        writeln!(&mut out, "|:-:|---")?;
        writeln!(&mut out, "|#|Name|Alias|Type|ID|")?;
        writeln!(&mut out, "|--:|---")?;
        for (i, device) in iter {
            self.print_device(device, i, &reverse_aliases, &mut out)?;
        }
        writeln!(&mut out, "|---|---")?;
        Ok(())
    }

    fn print_device(
        &self,
        device: &Device,
        index: usize,
        reverse_aliases: &HashMap<&str, Vec<&str>>,
        mut out: impl Write,
    ) -> anyhow::Result<()> {
        let index = index + 1;
        let mut aliases: Vec<&str> = Vec::new();
        if let Some(list) = reverse_aliases.get(index.to_string().as_str()) {
            aliases.extend(list);
        }
        if let Some(list) = reverse_aliases.get(device.device_id()) {
            aliases.extend(list);
        }
        if !aliases.is_empty() {
            aliases.sort();
        }
        writeln!(
            out,
            "|{index}|{}|{}|{}|{}|",
            device.device_name(),
            aliases.iter().join(", "),
            if device.is_remote() {
                device.remote_type()
            } else {
                device.device_type()
            },
            device.device_id()
        )?;
        Ok(())
    }

    const COMMAND_URL: &str = "https://github.com/OpenWonderLabs/SwitchBotAPI#device-specifications-and-supported-features-list";
    const COMMAND_IR_URL: &str = "https://github.com/OpenWonderLabs/SwitchBotAPI/blob/main/devices/others/virtual-infrared-remote-devices.md";

    async fn print_help(&mut self) -> anyhow::Result<()> {
        if self.help.is_none() {
            self.help = Some(Help::load().await?);
        }
        let device = self.first_current_device();
        let command_helps = self.help.as_ref().unwrap().command_helps(device);
        let help_url = if device.is_remote() {
            Self::COMMAND_IR_URL
        } else {
            Self::COMMAND_URL
        };
        if command_helps.is_empty() {
            anyhow::bail!(
                r#"No help for "{}". Please see {} for more information"#,
                device.device_type_or_remote_type(),
                help_url
            )
        }
        let mut out = MarkdownPrinter::new();
        writeln!(&mut out, "|---|---")?;
        for help in command_helps {
            writeln!(
                &mut out,
                "|{}|{}|",
                help.command(),
                help.description().markdown()
            )?;
        }
        writeln!(&mut out, "|---|---")?;
        out.flush()?;
        println!("Please see {help_url} for more information");
        Ok(())
    }

    async fn execute_args(&mut self, list: &[String]) -> anyhow::Result<()> {
        for command in list {
            self.execute(command).await?;
        }
        Ok(())
    }

    async fn execute(&mut self, text: &str) -> anyhow::Result<()> {
        let expanded = self.args.aliases.expand(text);
        let parsed = CliCommandLine::parse(
            &expanded,
            |s| self.parse_device_indexes(s).is_ok(),
            |s| self.args.aliases.expand(s),
        )?;

        if let Some(ref selector) = parsed.device_selector {
            self.set_current_devices(selector)?;
        }

        if let Some(cmd) = parsed.command {
            if cmd.requires_current_device() && !self.has_current_device() {
                return Err(self.set_current_devices(&expanded).unwrap_err());
            }
            self.execute_ast(cmd).await?;
        }
        Ok(())
    }

    async fn execute_ast(&mut self, cmd: CliCommand) -> anyhow::Result<()> {
        match cmd {
            CliCommand::Devices => {
                self.print_all_devices()?;
            }
            CliCommand::AliasList => {
                self.args.aliases.print();
            }
            CliCommand::AliasSet { name, value } => {
                if let Some(val) = value {
                    self.args.aliases.insert(name, val);
                } else {
                    self.args.aliases.remove(&name);
                }
            }
            CliCommand::Help => {
                assert!(self.has_current_device());
                self.print_help().await?;
            }
            CliCommand::Status { key } => {
                assert!(self.has_current_device());
                self.update_status(key.as_deref().unwrap_or("")).await?;
            }
            CliCommand::DeviceCommand(command) => {
                assert!(self.has_current_device());
                self.for_each_selected_device(|device| device.command(&command), |_| Ok(()))
                    .await?;
            }
            CliCommand::If {
                separator: _,
                condition,
                then_command,
                else_command,
            } => {
                assert!(self.has_current_device());
                let (device, expr) = self.device_expr(&condition);
                device.update_status().await?;
                let eval_result = device.eval_condition(expr)?;
                let command = if eval_result {
                    then_command
                } else {
                    else_command
                };
                log::debug!("if: {condition} is {eval_result}, execute {command}");
                Box::pin(self.execute(&command)).await?;
            }
        }
        Ok(())
    }

    fn set_current_devices(&mut self, text: &str) -> anyhow::Result<()> {
        self.current_device_indexes = self.parse_device_indexes(text)?;
        log::debug!("current_device_indexes={:?}", self.current_device_indexes);
        self.is_current_devices_changed = true;
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
        indexes = indexes.into_iter().unique().collect::<Vec<_>>();
        Ok(indexes)
    }

    fn parse_device_index(&self, value: &str) -> anyhow::Result<usize> {
        if let Ok(number) = value.parse::<usize>()
            && number > 0
            && number <= self.devices().len()
        {
            return Ok(number - 1);
        }
        self.devices()
            .index_by_device_id(value)
            .ok_or_else(|| anyhow::anyhow!("Not a valid device: \"{value}\""))
    }

    fn device_expr<'a>(&'a self, expr: &'a str) -> (&'a Device, &'a str) {
        if let Some((device, expr)) = expr.split_once('.')
            && let Ok(device_indexes) = self.parse_device_indexes(device)
        {
            return (&self.devices()[device_indexes[0]], expr);
        }
        (self.first_current_device(), expr)
    }

    async fn update_status(&self, key: &str) -> anyhow::Result<()> {
        self.for_each_selected_device(
            |device: &Device| device.update_status(),
            |device| {
                if key.is_empty() {
                    device.write_status_to(stdout())?;
                } else if let Some(value) = device.status_by_key(key) {
                    println!("{value}");
                } else {
                    log::error!(r#"No status key "{key}" for {device}"#);
                }
                Ok(())
            },
        )
        .await?;
        Ok(())
    }

    async fn for_each_selected_device<'a, 'b, FnAsync, Fut>(
        &'a self,
        fn_async: FnAsync,
        fn_post: impl Fn(&Device) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>
    where
        FnAsync: Fn(&'a Device) -> Fut + Send + Sync,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'b,
    {
        assert!(self.has_current_device());

        let results = if self.num_current_devices() < self.args.parallel_threshold {
            log::debug!("for_each: sequential ({})", self.num_current_devices());
            let mut results = Vec::with_capacity(self.num_current_devices());
            for device in self.current_devices() {
                results.push(fn_async(device).await);
            }
            results
        } else {
            log::debug!("for_each: parallel ({})", self.num_current_devices());
            let (_, join_results) = async_scoped::TokioScope::scope_and_block(|s| {
                for device in self.current_devices() {
                    s.spawn(fn_async(device));
                }
            });
            join_results
                .into_iter()
                .map(|result| result.unwrap_or_else(|error| Err(error.into())))
                .collect()
        };

        let last_error_index = results.iter().rposition(|result| result.is_err());
        for (i, (device, result)) in zip(self.current_devices(), results).enumerate() {
            match result {
                Ok(_) => fn_post(device)?,
                Err(error) => {
                    if i == last_error_index.unwrap() {
                        return Err(error);
                    }
                    log::error!("{error}");
                }
            }
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
        // The result should not be sorted.
        assert_eq!(cli.parse_device_indexes("4,2").unwrap(), vec![3, 1]);
        assert_eq!(cli.parse_device_indexes("device4,2").unwrap(), vec![3, 1]);
        // The result should be unique.
        assert_eq!(cli.parse_device_indexes("2,4,2").unwrap(), vec![1, 3]);
        assert_eq!(cli.parse_device_indexes("4,2,4").unwrap(), vec![3, 1]);
    }

    #[test]
    fn parse_device_indexes_alias() {
        let mut cli = Cli::new_for_test(10);
        cli.args.aliases.insert("k".into(), "3,5".into());
        assert_eq!(cli.parse_device_indexes("k").unwrap(), vec![2, 4]);
        assert_eq!(cli.parse_device_indexes("1,k,4").unwrap(), vec![0, 2, 4, 3]);
        cli.args.aliases.insert("j".into(), "2,k".into());
        assert_eq!(
            cli.parse_device_indexes("1,j,4").unwrap(),
            vec![0, 1, 2, 4, 3]
        );
        assert_eq!(cli.parse_device_indexes("1,j,5").unwrap(), vec![0, 1, 2, 4]);
    }

    #[tokio::test]
    async fn command_alias() {
        let mut cli = Cli::new_for_test(10);
        assert_eq!(cli.args.aliases.len(), 0);

        // Add alias
        cli.execute("alias a=b").await.unwrap();
        assert_eq!(cli.args.aliases.len(), 1);
        assert_eq!(cli.args.aliases.get("a").unwrap(), "b");

        // Update alias
        cli.execute("alias a=c").await.unwrap();
        assert_eq!(cli.args.aliases.len(), 1);
        assert_eq!(cli.args.aliases.get("a").unwrap(), "c");

        // Remove alias
        cli.execute("alias a=").await.unwrap();
        assert_eq!(cli.args.aliases.len(), 0);

        // Print aliases (should return Ok but not change aliases)
        cli.execute("alias").await.unwrap();
        assert_eq!(cli.args.aliases.len(), 0);

        // Remove non-existent alias
        cli.execute("alias a=").await.unwrap();
        assert_eq!(cli.args.aliases.len(), 0);

        // Alias without '=' removes it (consistent with Args::update_alias)
        cli.args.aliases.insert("a".into(), "b".into());
        cli.execute("alias a").await.unwrap();
        assert_eq!(cli.args.aliases.len(), 0);
    }
}
