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

A command-line tool for controlling SwitchBot devices
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
to avoid entering them every time.
The `--clear` option clears the saved authentication,
and the `switchbot` command will prompt for the authentication again.
The `--token` and the `--secret` options are available
to specify them as command line arguments,
as well as environment variables `SWITCHBOT_TOKEN` and `SWITCHBOT_SECRET`.

[token-secret]: https://github.com/OpenWonderLabs/SwitchBotAPI#getting-started

## Interactive Mode and Batch Mode

The `switchbot` command can run either interactively,
or in the batch mode.

### Interactive Mode
[interactive mode]: #interactive-mode

The `switchbot` command enters the interactive mode
when no arguments are specified.

```shell-session
$ switchbot
1: Hub Mini AF (Hub Mini, ID:111222333)
2: Bedroom Light (DIY Light, ID:444555666)
3: Living Fan (Fan, ID:777888999)
...
Device>
```
All your devices are listed with a number,
the device name you set in the SwitchBot app,
the device type, and the device ID.

In this mode, following actions are available:
* [Select devices][device].
* [Send commands][command] to the selected devices.
* [Query status][status] of the selected devices.
* [Send different commands depending on the device status][if-command].
* Hit the Enter key twice, or enter `q` to quit the `switchbot` command.

### Batch Mode
[Batch Mode]: #batch-mode

It is also possible to run the `switchbot` command in non-interactive way
by specifying the [device number or the device ID][device]
and the [commands][command] as arguments.
This mode is useful to create your own batch files,
or to use with launcher applications such as Elgato Stream Deck.

```shell-session
switchbot 1 on
```
The example above selects the device 1,
and send the "on" command to it.

You can also specify multiple devices and commands.
```shell-session
switchbot 1 on setMode:101 4,6 off
```
This example turns on the device 1 and set its mode to 101,
and turns off the device 4 and the device 6.

Getting the [device status](#status) in the batch mode is also possible.
```shell-session
switchbot 4,6 status.battery
```
This example prints the battery status of the device 4 and 6.

## Select Device
[device]: #select-device

To select the device to interact with,
enter either the number or the device ID.
```shell-session
1: Hub Mini AF (Hub Mini, ID:111222333)
2: Bedroom Light (DIY Light, ID:444555666)
3: Living Fan (Fan, ID:777888999)
...
Device> 2
Name: Bedroom Light
ID: 444555666
Type: DIY Light
Command>
```
To unselect devices, just hit the Enter key.

### Multiple Devices
[multiple devices]: #multiple-devices

It is possible to select multiple devices at once
by specifying multiple numbers or device IDs separated by `,` (comma).
This is handy when you want to send the same command to multiple devices at once,
such as turning off all devices.
```shell-session
Device> 2,3
```

## Command
[command]: #command

To control your devices, enter the command name you want to send.
The available commands depend on the device type.
Please refer to the
[SwitchBot API documentation about device control commands][send-device-control-commands]
and the [built-in commands]
to find the command you want to send to your devices.

The following [interactive mode] example sends the `turnOn` command to the device number 2.
```shell-session
Device> 2
Name: Bedroom Light
ID: 444555666
Type: DIY Light
Command> turnOn
```
The following [batch mode] example does the same thing from the command line.
```shell-session
switchbot 2 turnOn
```

[send-device-control-commands]: https://github.com/OpenWonderLabs/SwitchBotAPI#send-device-control-commands

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

## Built-in Commands
[built-in commands]: #built-in-commands

Following commands are executed by the `switchbot` command itself
without being sent to the [SwitchBot API].
* The `devices` command prints all devices.
* The [`status`][status] and the [`status.key`][status-key] commands
  query the device status.
* The [`if`-command][if-command].

## Status
[status]: #status

To query the status of your devices, enter `status`.
The output depends on the device type.
Please refer to the
[SwitchBot API documentation about device status][get-device-status]
for the details of the status for each device type.
```shell-session
Command> status
power: "off"
fanSpeed: 23
...
```

[get-device-status]: https://github.com/OpenWonderLabs/SwitchBotAPI#get-device-status

### Status of a Key
[status-key]: #status-of-a-key

It is also possible to check the status of a specific key
by specifying `status.` (status dot) followed by the key name.
For example, the following command shows the power status
of the selected device.
```shell-session
Command> status.power
"off"
```

## If-Command
[if-command]: #if-command

The conditional "if" command allows you to
send different commands depending on the [device status][status].
This capability allows you to create a command to toggle device statuses.
The following example
turns off the device if the power status is on,
and turns it on otherwise.
```shell-session
Command> if/power=on/off/on
```
Following operators are supported.
* `key`, `key=true`, and `key=false` for boolean types.
* `=`, `<`, `<=`, `>`, and `>=` for numeric types.
* `=` for string and other types.

When [multiple devices] are selected,
the first device is used to determine which command to execute.
Then the command is executed on all selected devices.
This is to make the behaviors consistent across multiple devices.
In the following [Batch Mode] example,
if the device 4 is on, both the device 2 and 4 are turned off,
regardless of the power status of the device 2.
```shell-session
switchbot 4,2 if/power=on/off/on
```

If you want to toggle multiple devices independently,
please specify the if-command for each device.
```shell-session
switchbot 4 if/power=on/off/on 2 if/power=on/off/on
```
[Aliases] can make it easier to type, as shown in the example below.
```shell-session
switchbot -a t=if/power=on/off/on 4 t 2 t
```

> [!NOTE]
> If the `/` (slash) is used in the device name or the command,
> other non-alphanumeric characters can be used as the separator,
> as long as the same character is used for all separators.
> ```shell-session
> switchbot 4 if.power=on.off.on
> ```

## Aliases
[aliases]: #aliases

Some commands have aliases for convenience.

By default, following aliases are defined.
* `on` for `turnOn`.
* `off` for `turnOff`.

With these aliases,
the following example sends the `turnOn` command to the device 2.
```shell-session
switchbot 2 on
```

You can also add your own aliases by the `-a` option.
Both commands and devices can be aliased.
```shell-session
switchbot -a fan=777888999 -a hot=fanSpeed:100 -a lights=2,5,6
switchbot fan hot lights on
```
To remove existing aliases, please omit the value.
```shell-session
switchbot -a hot
```

# Change History

Please see the [release notes] for the change history.

[release notes]: https://github.com/kojiishi/switchbot-rs/releases
