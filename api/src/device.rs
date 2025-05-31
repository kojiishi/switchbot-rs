use std::{fmt::Display, rc::Rc};

use super::*;

/// Represents a device.
///
/// For the details of fields, please refer to the [devices] section
/// of the API documentation.
///
/// [devices]: https://github.com/OpenWonderLabs/SwitchBotAPI?tab=readme-ov-file#devices
#[derive(Debug, serde::Deserialize)]
#[allow(non_snake_case)]
pub struct Device {
    deviceId: String,
    deviceName: String,
    #[serde(default)]
    deviceType: String,
    #[serde(default)]
    remoteType: String,
    hubDeviceId: String,

    #[serde(skip)]
    service: Rc<SwitchBotService>,
}

impl Device {
    /// The device ID>
    pub fn device_id(&self) -> &str {
        &self.deviceId
    }

    /// The device name.
    /// This is the name configured in the SwitchBot app.
    pub fn device_name(&self) -> &str {
        &self.deviceName
    }

    /// True if this device is an infrared remote device.
    pub fn is_remote(&self) -> bool {
        !self.remoteType.is_empty()
    }

    /// The device type.
    /// This is empty if this is an infrared remote device.
    pub fn device_type(&self) -> &str {
        &self.deviceType
    }

    /// The device type for an infrared remote device.
    pub fn remote_type(&self) -> &str {
        &self.remoteType
    }

    /// The parent Hub ID.
    pub fn hub_device_id(&self) -> &str {
        &self.hubDeviceId
    }

    pub(crate) fn set_service(&mut self, service: Rc<SwitchBotService>) {
        self.service = service;
    }

    /// Send the `command` to the [SwitchBot API].
    ///
    /// Please also see the [`CommandRequest`].
    pub async fn command(&self, command: &CommandRequest) -> anyhow::Result<()> {
        self.service.command(self.device_id(), command).await
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            writeln!(f, "Name: {}", self.device_name())?;
            writeln!(f, "ID: {}", self.device_id())?;
            if self.is_remote() {
                write!(f, "Remote Type: {}", self.remote_type())?;
            } else {
                write!(f, "Type: {}", self.device_type())?;
            }
            return Ok(());
        }
        write!(
            f,
            "{} ({}, ID:{})",
            self.deviceName,
            if self.is_remote() {
                self.remote_type()
            } else {
                self.device_type()
            },
            self.deviceId
        )
    }
}
