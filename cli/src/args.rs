use std::{fs, path::PathBuf};

use clap::Parser;
use switchbot_api::SwitchBot;

use crate::UserInput;

#[derive(Debug, Default, Parser, serde::Deserialize, serde::Serialize)]
#[command(version, about)]
pub struct Args {
    /// The token for the authentication.
    #[arg(long, default_value_t = String::default())]
    pub token: String,
    /// The secret for the authentication.
    #[arg(long, default_value_t = String::default())]
    pub secret: String,

    /// Clear the saved authentication.
    #[arg(long, default_value_t = false)]
    #[serde(skip)]
    pub clear: bool,

    #[serde(skip)]
    pub commands: Vec<String>,
}

impl Args {
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

    pub fn load(&mut self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        log::debug!("load config: {path:?}");
        let json = fs::read_to_string(&path)?;
        let load_data: Args = serde_json::from_str(&json)?;
        if self.token.is_empty() {
            self.token = load_data.token;
        }
        if self.secret.is_empty() {
            self.secret = load_data.secret;
        }
        Ok(())
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
