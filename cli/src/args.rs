use std::{collections::HashMap, fs, path::PathBuf, time::Duration};

use clap::Parser;
use itertools::Itertools;
use switchbot_api::{Device, SwitchBot};

use crate::UserInput;

#[derive(Debug, Default, Parser, serde::Deserialize, serde::Serialize)]
#[command(version, about)]
pub(crate) struct Args {
    /// The token for the authentication.
    #[arg(long, default_value_t, env = "SWITCHBOT_TOKEN")]
    pub token: String,
    /// The secret for the authentication.
    #[arg(long, default_value_t, env = "SWITCHBOT_SECRET")]
    pub secret: String,

    /// Clear the saved authentication.
    #[arg(long)]
    #[serde(skip)]
    pub clear: bool,

    /// Add/remove aliases ("alias=value" to add, omit the value to remove).
    #[arg(short, long = "alias")]
    #[serde(skip)]
    pub alias_updates: Vec<String>,

    /// The interval for remote devices in seconds [default: 0.5].
    #[arg(long)]
    #[serde(skip)]
    pub pause: Option<f64>,

    /// The minimum number of tasks to parallelize.
    #[arg(short = 'P', long, default_value_t = 2)]
    #[serde(skip)]
    pub parallel_threshold: usize,

    #[arg(skip)]
    #[serde(default)]
    pub aliases: HashMap<String, String>,

    #[serde(skip)]
    pub commands: Vec<String>,

    #[arg(skip)]
    #[serde(default, rename = "version")]
    pub config_version: u8,
}

impl Args {
    pub fn new_from_args() -> Self {
        let mut args = Args::parse();
        if let Err(error) = args.merge_config() {
            log::debug!("Load config error: {error}");
        }
        args.ensure_default();
        args
    }

    pub fn process(&mut self) -> anyhow::Result<()> {
        if let Some(seconds) = self.pause {
            Device::set_default_min_internal_for_remote_devices(Duration::from_secs_f64(seconds));
        }
        if !self.alias_updates.is_empty() {
            self.update_aliases();
        }
        Ok(())
    }

    pub fn create_switch_bot(&mut self) -> anyhow::Result<SwitchBot> {
        self.ensure_auth()?;
        Ok(SwitchBot::new_with_authentication(
            &self.token,
            &self.secret,
        ))
    }

    pub fn ensure_auth(&mut self) -> anyhow::Result<()> {
        log::trace!("ensure_auth: {} {}", self.token, self.secret);
        if self.token.is_empty() {
            let mut input = UserInput::new_with_prompt("Token> ");
            self.token = input.read_line()?.into();
        }
        if self.secret.is_empty() {
            let mut input = UserInput::new_with_prompt("Secret> ");
            self.secret = input.read_line()?.into();
        }
        Ok(())
    }

    pub fn clear_auth(&mut self) {
        self.token = String::default();
        self.secret = String::default();
    }

    pub fn ensure_default(&mut self) {
        if self.config_version < 1 {
            self.aliases.extend([
                ("on".into(), "turnOn".into()),
                ("off".into(), "turnOff".into()),
            ]);
            self.config_version = 1;
        }
        if self.config_version < 2 {
            self.add_alias_if_missing("d", "devices");
            self.config_version = 2;
        }
        if self.config_version < 3 {
            self.add_alias_if_missing("h", "help");
            self.config_version = 3;
        }
    }

    fn add_alias_if_missing(&mut self, alias: &str, command: &str) {
        if !self.aliases.contains_key(alias) {
            self.aliases.insert(alias.into(), command.into());
        }
    }

    pub fn update_aliases(&mut self) {
        for alias in &self.alias_updates {
            if alias.is_empty() {
                continue;
            }
            if let Some((alias, command)) = alias.split_once('=') {
                if !command.is_empty() {
                    self.aliases.insert(alias.into(), command.into());
                } else {
                    self.aliases.remove(alias);
                }
            } else {
                self.aliases.remove(alias);
            }
        }
    }

    pub fn print_aliases(&self) {
        for (alias, to) in self.aliases.iter().sorted() {
            println!("{alias}={to}");
        }
    }

    pub fn merge_config(&mut self) -> anyhow::Result<()> {
        let mut args: Args = Self::load()?;
        if self.clear {
            args.clear_auth();
        }
        self.merge(&args);
        Ok(())
    }

    fn merge(&mut self, other: &Args) {
        if self.token.is_empty() {
            self.token = other.token.clone();
        }
        if self.secret.is_empty() {
            self.secret = other.secret.clone();
        }
        self.aliases.extend(other.aliases.clone());
    }

    pub fn load() -> anyhow::Result<Args> {
        let path = Self::config_path()?;
        log::debug!("load config: {path:?}");
        let json = fs::read_to_string(&path)?;
        let args: Args = serde_json::from_str(&json)?;
        Ok(args)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        log::debug!("save config: {path:?}");
        fs::create_dir_all(path.parent().unwrap())?;
        let json = serde_json::to_string(self)?;
        fs::write(&path, json)?;
        Ok(())
    }

    fn config_path() -> anyhow::Result<PathBuf> {
        if let Some(dirs) = directories::ProjectDirs::from("", "kojii", "switchbot") {
            let dir = dirs.config_dir();
            let path = dir.join("config.json");
            return Ok(path);
        }
        Err(anyhow::anyhow!("No config directory found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_default() {
        let mut args = Args::default();
        assert_eq!(args.config_version, 0);
        assert_eq!(args.aliases.len(), 0);
        args.ensure_default();
        assert_eq!(args.config_version, 3);
        assert_eq!(args.aliases.len(), 4);
    }

    #[test]
    fn args_from_json_no_alias() -> anyhow::Result<()> {
        let args: Args = serde_json::from_str(r#"{"token":"test_token", "secret":"test_secret"}"#)?;
        assert_eq!(args.token, "test_token");
        assert!(args.aliases.is_empty());
        Ok(())
    }

    #[test]
    fn update_aliases() {
        let mut args = Args::default();
        assert_eq!(args.aliases.len(), 0);

        // Empty string is allowed as a no-op.
        args.alias_updates = vec!["".into()];
        args.update_aliases();
        assert_eq!(args.aliases.len(), 0);

        // The alias can contains the `=` character.
        args.alias_updates = vec!["a=b=c".into()];
        args.update_aliases();
        assert_eq!(args.aliases.len(), 1);
        assert_eq!(args.aliases.get("a").unwrap(), "b=c");

        args.alias_updates = vec!["a=b".into(), "c=d".into()];
        args.update_aliases();
        assert_eq!(args.aliases.len(), 2);
        assert_eq!(args.aliases.get("a").unwrap(), "b");
        assert_eq!(args.aliases.get("c").unwrap(), "d");

        // No value removes the alias.
        args.alias_updates = vec!["c".into()];
        args.update_aliases();
        assert_eq!(args.aliases.len(), 1);
        assert_eq!(args.aliases.get("a").unwrap(), "b");

        // Removing non-existent alias is allowed.
        args.alias_updates = vec!["z".into()];
        args.update_aliases();
        assert_eq!(args.aliases.len(), 1);
        assert_eq!(args.aliases.get("a").unwrap(), "b");

        // Update existing alias.
        args.alias_updates = vec!["a=x".into()];
        args.update_aliases();
        assert_eq!(args.aliases.len(), 1);
        assert_eq!(args.aliases.get("a").unwrap(), "x");

        // Empty value also removes the alias.
        args.alias_updates = vec!["a=".into()];
        args.update_aliases();
        assert_eq!(args.aliases.len(), 0);
    }
}
