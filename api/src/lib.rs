//! This is a Rust library to interact with the [SwitchBot API]
//! and control your SwitchBot devices programmatically.
//!
//! [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
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

mod device;
pub use device::*;
mod device_list;
pub use device_list::*;
mod switch_bot;
pub use switch_bot::*;
mod switch_bot_service;
pub(crate) use switch_bot_service::*;
pub use switch_bot_service::{CommandRequest, SwitchBotError};
