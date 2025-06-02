//! This library provides the command line tool for the [SwitchBot API]
//! as part of your program.
//!
//! For the usages of the command line tool,
//! please see the [`switchbot-cli`][cli-crate] crate.
//!
//! For the lower-level API,
//! please see the [`switchbot-api`][api-docs] crate.
//!
//! [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
//! [cli-crate]: https://crates.io/crates/switchbot-cli
//! [api-docs]: https://docs.rs/switchbot-api/

mod args;
pub(crate) use args::Args;
mod cli;
pub use cli::Cli;
mod user_input;
pub(crate) use user_input::UserInput;
