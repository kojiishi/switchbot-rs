use std::{
    fmt::Display,
    sync::{Arc, Weak},
};

use super::*;

/// Represents a device.
///
/// For the details of fields, please refer to the [devices] section
/// of the API documentation.
///
/// [devices]: https://github.com/OpenWonderLabs/SwitchBotAPI?tab=readme-ov-file#devices
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    device_id: String,
    device_name: String,
    #[serde(default)]
    device_type: String,
    #[serde(default)]
    remote_type: String,
    hub_device_id: String,

    #[serde(skip)]
    service: Weak<SwitchBotService>,
}

impl Device {
    pub(crate) fn new_for_test(index: usize) -> Self {
        Self {
            device_id: format!("device{}", index),
            device_name: format!("Device {}", index),
            device_type: "test".into(),
            ..Default::default()
        }
    }

    /// The device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// The device name.
    /// This is the name configured in the SwitchBot app.
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// True if this device is an infrared remote device.
    pub fn is_remote(&self) -> bool {
        !self.remote_type.is_empty()
    }

    /// The device type.
    /// This is empty if this is an infrared remote device.
    pub fn device_type(&self) -> &str {
        &self.device_type
    }

    /// The device type for an infrared remote device.
    pub fn remote_type(&self) -> &str {
        &self.remote_type
    }

    /// The parent Hub ID.
    pub fn hub_device_id(&self) -> &str {
        &self.hub_device_id
    }

    fn service(&self) -> anyhow::Result<Arc<SwitchBotService>> {
        self.service
            .upgrade()
            .ok_or_else(|| anyhow::anyhow!("The service is dropped"))
    }

    pub(crate) fn set_service(&mut self, service: &Arc<SwitchBotService>) {
        self.service = Arc::downgrade(service);
    }

    /// Send the `command` to the [SwitchBot API].
    ///
    /// Please also see the [`CommandRequest`].
    ///
    /// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
    pub async fn command(&self, command: &CommandRequest) -> anyhow::Result<()> {
        self.service()?.command(self.device_id(), command).await
    }

    fn fmt_multi_line(&self, buf: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(buf, "Name: {}", self.device_name())?;
        writeln!(buf, "ID: {}", self.device_id())?;
        if self.is_remote() {
            write!(buf, "Remote Type: {}", self.remote_type())?;
        } else {
            write!(buf, "Type: {}", self.device_type())?;
        }
        Ok(())
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return self.fmt_multi_line(f);
        }
        write!(
            f,
            "{} ({}, ID:{})",
            self.device_name,
            if self.is_remote() {
                self.remote_type()
            } else {
                self.device_type()
            },
            self.device_id
        )
    }
}
