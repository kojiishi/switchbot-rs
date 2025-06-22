use std::{future::Future, io::stdout, iter::zip};

use itertools::Itertools;
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
        self.run_core().await?;
        self.args.save()?;
        Ok(())
    }

    async fn run_core(&mut self) -> anyhow::Result<()> {
        let mut is_interactive = true;
        if !self.args.alias_updates.is_empty() {
            self.args.print_aliases();
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
        self.print_devices();
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
                        self.print_devices();
                        continue;
                    }
                    break;
                }
                _ => match self.execute(input_text).await {
                    Ok(true) => self.print_devices(),
                    Ok(false) => {}
                    Err(error) => log::error!("{error}"),
                },
            }
        }
        Ok(())
    }

    fn print_devices(&self) {
        if !self.has_current_device() {
            self.print_all_devices();
            return;
        }

        if self.current_device_indexes.len() >= 2 {
            for (i, device) in self.current_devices_with_index() {
                println!("{}: {device}", i + 1);
            }
            return;
        }

        let device = self.first_current_device();
        print!("{device:#}");
    }

    fn print_all_devices(&self) {
        for (i, device) in self.devices().iter().enumerate() {
            println!("{}: {device}", i + 1);
        }
    }

    async fn execute_args(&mut self, list: &[String]) -> anyhow::Result<()> {
        for command in list {
            self.execute(command).await?;
        }
        Ok(())
    }

    async fn execute(&mut self, text: &str) -> anyhow::Result<bool> {
        if let Some(alias) = self.args.aliases.get(text) {
            log::debug!(r#"alias: "{text}" -> "{alias}""#);
            return self.execute_no_alias(&alias.clone()).await;
        }
        self.execute_no_alias(text).await
    }

    /// Returns `true` if the current devices are changed.
    async fn execute_no_alias(&mut self, text: &str) -> anyhow::Result<bool> {
        let set_device_result = self.set_current_devices(text);
        if set_device_result.is_ok() {
            return Ok(true);
        }
        if self.execute_global_builtin_command(text)? {
            return Ok(false);
        }
        if self.has_current_device() {
            if self.execute_if_expr(text).await? {
                return Ok(false);
            }
            self.execute_command(text).await?;
            return Ok(false);
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
        indexes = indexes.into_iter().unique().collect::<Vec<_>>();
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

    async fn execute_if_expr(&mut self, expr: &str) -> anyhow::Result<bool> {
        assert!(self.has_current_device());
        if let Some((condition, then_command, else_command)) = Self::parse_if_expr(expr) {
            let (device, expr) = self.device_expr(condition);
            device.update_status().await?;
            let eval_result = device.eval_condition(expr)?;
            let command = if eval_result {
                then_command
            } else {
                else_command
            };
            log::debug!("if: {condition} is {eval_result}, execute {command}");
            Box::pin(self.execute(command)).await?;
            return Ok(true);
        }
        Ok(false)
    }

    fn parse_if_expr(text: &str) -> Option<(&str, &str, &str)> {
        if let Some(text) = text.strip_prefix("if") {
            if let Some(sep) = text.chars().nth(0) {
                if sep.is_alphanumeric() {
                    return None;
                }
                let fields: Vec<&str> = text[1..].split_terminator(sep).collect();
                match fields.len() {
                    2 => return Some((fields[0], fields[1], "")),
                    3 => return Some((fields[0], fields[1], fields[2])),
                    _ => {}
                }
            }
        }
        None
    }

    fn device_expr<'a>(&'a self, expr: &'a str) -> (&'a Device, &'a str) {
        if let Some((device, expr)) = expr.split_once('.') {
            if let Ok(device_indexes) = self.parse_device_indexes(device) {
                return (&self.devices()[device_indexes[0]], expr);
            }
        }
        (self.first_current_device(), expr)
    }

    fn execute_global_builtin_command(&self, text: &str) -> anyhow::Result<bool> {
        if text == "devices" {
            self.print_all_devices();
            return Ok(true);
        }
        Ok(false)
    }

    async fn execute_device_builtin_command(&self, text: &str) -> anyhow::Result<bool> {
        assert!(self.has_current_device());
        if text == "status" {
            self.update_status("").await?;
            return Ok(true);
        }
        if let Some(key) = text.strip_prefix("status.") {
            self.update_status(key).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn execute_command(&self, text: &str) -> anyhow::Result<()> {
        assert!(self.has_current_device());
        if text.is_empty() {
            return Ok(());
        }
        if self.execute_device_builtin_command(text).await? {
            return Ok(());
        }
        let command = CommandRequest::from(text);
        self.for_each_selected_device(|device| device.command(&command), |_| Ok(()))
            .await?;
        Ok(())
    }

    async fn update_status(&self, key: &str) -> anyhow::Result<()> {
        self.for_each_selected_device(
            |device: &Device| device.update_status(),
            |device| {
                if key.is_empty() {
                    device.write_status_to(stdout())?;
                } else if let Some(value) = device.status_by_key(key) {
                    println!("{}", value);
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

    #[test]
    fn parse_if_expr() {
        assert_eq!(Cli::parse_if_expr(""), None);
        assert_eq!(Cli::parse_if_expr("a"), None);
        assert_eq!(Cli::parse_if_expr("if"), None);
        assert_eq!(Cli::parse_if_expr("if/a"), None);
        assert_eq!(Cli::parse_if_expr("if/a/b"), Some(("a", "b", "")));
        assert_eq!(Cli::parse_if_expr("if/a/b/c"), Some(("a", "b", "c")));
        assert_eq!(Cli::parse_if_expr("if/a//c"), Some(("a", "", "c")));
        // The separator can be any characters as long as they're consistent.
        assert_eq!(Cli::parse_if_expr("if;a;b;c"), Some(("a", "b", "c")));
        assert_eq!(Cli::parse_if_expr("if.a.b.c"), Some(("a", "b", "c")));
        // But non-alphanumeric.
        assert_eq!(Cli::parse_if_expr("ifXaXbXc"), None);
    }
}
