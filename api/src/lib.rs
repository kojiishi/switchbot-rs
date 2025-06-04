//! This is a Rust library to interact with the [SwitchBot API]
//! and control your SwitchBot devices programmatically.
//!
//! For a command line tool,
//! please see the [`switchbot-cli`][cli-crate] crate.
//!
//! [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
//! [cli-crate]: https://crates.io/crates/switchbot-cli
//!
//! # Examples
//! ```no_run
//! # use switchbot_api::SwitchBot;
//! # async fn test(token: &str, secret: &str) -> anyhow::Result<()> {
//! let mut switch_bot = SwitchBot::new_with_authentication(token, secret);
//! switch_bot.load_devices().await?;
//! # Ok(())
//! # }
//! ```

mod command_request;
pub use command_request::*;
mod device;
pub use device::*;
mod device_list;
pub use device_list::*;
mod switch_bot;
pub use switch_bot::*;
mod switch_bot_service;
pub use switch_bot_service::*;
