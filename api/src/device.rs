use std::{
    collections::HashMap,
    fmt::Display,
    io,
    sync::{Arc, RwLock, RwLockReadGuard, Weak},
};

use super::*;

/// Represents a device.
///
/// For the details of fields, please refer to the [devices] section
/// of the API documentation.
///
/// [devices]: https://github.com/OpenWonderLabs/SwitchBotAPI#devices
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    device_id: String,
    #[serde(default)] // Missing in the status response.
    device_name: String,
    #[serde(default)]
    device_type: String,
    #[serde(default)]
    remote_type: String,
    hub_device_id: String,

    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,

    #[serde(skip)]
    status: RwLock<HashMap<String, serde_json::Value>>,

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
    ///
    /// # Examples
    /// ```no_run
    /// # use switchbot_api::{CommandRequest, Device};
    /// # async fn turn_on(device: &Device) -> anyhow::Result<()> {
    /// let command = CommandRequest { command: "turnOn".into(), ..Default::default() };
    /// device.command(&command).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn command(&self, command: &CommandRequest) -> anyhow::Result<()> {
        self.service()?.command(self.device_id(), command).await
    }

    /// Get the [device status] from the [SwitchBot API].
    ///
    /// Please see [`Device::status_by_key()`] and some other functions
    /// to retrieve the status captured by this function.
    ///
    /// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
    /// [device status]: https://github.com/OpenWonderLabs/SwitchBotAPI#get-device-status
    pub async fn update_status(&self) -> anyhow::Result<()> {
        let status = self.service()?.status(self.device_id()).await?;
        assert_eq!(self.device_id, status.device_id);
        let mut writer = self.status.write().unwrap();
        *writer = status.extra;
        Ok(())
    }

    fn status(&self) -> RwLockReadGuard<'_, HashMap<String, serde_json::Value>> {
        self.status.read().unwrap()
    }

    /// Get the value of a key from the [device status].
    ///
    /// The [`Device::update_status()`] must be called prior to this function.
    ///
    /// # Examples
    /// ```no_run
    /// # use switchbot_api::Device;
    /// # async fn print_power_status(device: &Device) -> anyhow::Result<()> {
    /// device.update_status().await?;
    /// println!("Power = {}", device.status_by_key("power").unwrap());
    /// # Ok(())
    /// # }
    /// ```
    /// [device status]: https://github.com/OpenWonderLabs/SwitchBotAPI#get-device-status
    pub fn status_by_key(&self, key: &str) -> Option<serde_json::Value> {
        self.status().get(key).cloned()
    }

    /// Evaluate a conditional expression.
    ///
    /// The expression should be in the form of `key=value`.
    /// When the value is a boolean type, `key` is also a valid expression.
    ///
    /// Returns an error if the expression is invalid,
    /// or if the `key` does not exist.
    ///
    /// The [`Device::update_status()`] must be called prior to this function.
    ///
    /// # Examples
    /// ```no_run
    /// # use switchbot_api::Device;
    /// # async fn print_power_status(device: &Device) -> anyhow::Result<()> {
    /// device.update_status().await?;
    /// println!("Power-on = {}", device.eval_condition("power=on")?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn eval_condition(&self, condition: &str) -> anyhow::Result<bool> {
        let condition = ConditionalExpression::try_from(condition)?;
        let value = self
            .status_by_key(condition.key)
            .ok_or_else(|| anyhow::anyhow!(r#"No status key "{}" for {self}"#, condition.key))?;
        condition.evaluate(&value)
    }

    /// Write the list of the [device status] to the `writer`.
    ///
    /// The [`Device::update_status()`] must be called prior to this function.
    ///
    /// # Examples
    /// ```no_run
    /// # use switchbot_api::Device;
    /// # async fn print_status(device: &Device) -> anyhow::Result<()> {
    /// device.update_status().await?;
    /// device.write_status_to(std::io::stdout());
    /// # Ok(())
    /// # }
    /// ```
    /// [device status]: https://github.com/OpenWonderLabs/SwitchBotAPI#get-device-status
    pub fn write_status_to(&self, mut writer: impl io::Write) -> io::Result<()> {
        let status = self.status();
        for (key, value) in status.iter() {
            writeln!(writer, "{key}: {value}")?;
        }
        Ok(())
    }

    fn fmt_multi_line(&self, buf: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(buf, "Name: {}", self.device_name())?;
        writeln!(buf, "ID: {}", self.device_id())?;
        if self.is_remote() {
            writeln!(buf, "Remote Type: {}", self.remote_type())?;
        } else {
            writeln!(buf, "Type: {}", self.device_type())?;
        }
        let status = self.status();
        if !status.is_empty() {
            writeln!(buf, "Status:")?;
            for (key, value) in status.iter() {
                writeln!(buf, "  {key}: {value}")?;
            }
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
