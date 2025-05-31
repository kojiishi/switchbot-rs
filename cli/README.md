[![CI-badge]][CI]
[![crate-badge]][crate]
[![docs-badge]][docs]

[CI-badge]: https://github.com/kojiishi/switchbot-rs/actions/workflows/rust-ci.yml/badge.svg
[CI]: https://github.com/kojiishi/switchbot-rs/actions/workflows/rust-ci.yml
[crate-badge]: https://img.shields.io/crates/v/switchbot-cli.svg
[crate]: https://crates.io/crates/switchbot-cli
[docs-badge]: https://docs.rs/switchbot-cli/badge.svg
[docs]: https://docs.rs/switchbot-cli/

# switchbot-cli

A command-line interface for controlling SwitchBot devices
using the [SwitchBot API].

[SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI

# Install

## Prerequisites

* [Install Rust] if it's not installed yet.

[install Rust]: https://rustup.rs/

## From [`crates.io`][crate]

```shell-session
cargo install switchbot-cli
```

# Usages

## Authentication

On the first run, the `switchbot` command prompts to enter the authentication.
```shell-session
$ switchbot
Token>
```
Please refer to the [SwitchBot documentation about
how to obtain the token and secret key][token-secret].

They are saved in your configuration directory
to save you from entering them each time.
The `--clear` option clears the saved authentication,
and the `switchbot` command will prompt for the authentication again.
The `--token` and the `--secret` options are available
to specify them explicitly.

[token-secret]: https://github.com/OpenWonderLabs/SwitchBotAPI?tab=readme-ov-file#open-token-and-secret-key

## Interactive Mode

Once the authentication is done, the interactive mode starts.
```shell-session
$ switchbot
1: My Bedroom Light (DIY Light, ID:111222333)
2: Hub Mini AF (Hub Mini, ID:444555666)
...
Device>
```
All your devices are listed with a number,
the device name you set in the SwitchBot app,
the device type, and the device ID.

## Select Device

To select the device to interact with,
enter either the number or the device ID.
```shell-session
1: Hub Mini AF (Hub Mini, ID:111222333)
2: My Bedroom Light (DIY Light, ID:444555666)
...
Device> 2
Name: My Bedroom Light
ID: 444555666
Type: DIY Light
Command>
```

## Command

To control your devices, you can send commands.
The available commands depend on the device type.
please find the command you want to send to your devices
from the [SwitchBot documentation about device control commands][send-device-control-commands].

The following example sends the `turnOn` command to the DIY Light.
```shell-session
Device> 2
Name: My Bedroom Light
ID: 444555666
Type: DIY Light
Command> turnOn
```

[send-device-control-commands]: https://github.com/OpenWonderLabs/SwitchBotAPI?tab=readme-ov-file#send-device-control-commands

### Parameters

If the command you want to send has "command parameters" other than `default`,
append a `:` (colon) and the command parameters.
```shell-session
Command> setMode:101
```

### Command Type

If the command has a "commandType" other than `command`,
prepend it with a `/` (slash) as the separator.
```shell-session
Command> customize/button1
```

### Quit

Hit the Enter key twice, or enter `q` to quit the `switchbot` command.

## Non-interactive Mode

It is also possible to run the `switchbot` command in non-interactive mode
by specifying the device number or the device ID and the command as arguments.

This is useful to create your own batch files,
or to use with launcher applications such as Elgato Stream Deck.

```shell-session
$ switchbot 1 turnOn
```
You can also specify multiple devices and commands.
```shell-session
$ switchbot 1 turnOn setMode:101 4 turnOff
```
