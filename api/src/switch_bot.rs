use std::sync::Arc;

use super::*;

/// Represents a [SwitchBot API] server.
///
/// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
#[derive(Debug, Default)]
pub struct SwitchBot {
    service: Arc<SwitchBotService>,
    devices: DeviceList,
}

impl SwitchBot {
    /// Construct a new instance with the default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a new instance with the authentication information.
    /// This is equivalent to:
    /// ```
    /// # use switchbot_api::SwitchBot;
    /// # fn test(token: &str, secret: &str) {
    /// let mut switch_bot = SwitchBot::new();
    /// switch_bot.set_authentication(token, secret);
    /// # }
    /// ```
    pub fn new_with_authentication(token: &str, secret: &str) -> Self {
        Self {
            service: SwitchBotService::new(token, secret),
            ..Default::default()
        }
    }

    /// Construct an instance for testing.
    /// The instance has the specified number of devices for testing.
    pub fn new_for_test(num_devices: usize) -> Self {
        let mut devices = DeviceList::new();
        for i in 0..num_devices {
            devices.push(Device::new_for_test(i + 1));
        }
        Self {
            devices,
            ..Default::default()
        }
    }

    /// Set the authentication information.
    ///
    /// Please refer to the [SwitchBot documentation about
    /// how to obtain the token and secret key][token-secret].
    ///
    /// [token-secret]: https://github.com/OpenWonderLabs/SwitchBotAPI#open-token-and-secret-key
    pub fn set_authentication(&mut self, token: &str, secret: &str) {
        self.service = SwitchBotService::new(token, secret);
        self.devices.clear();
    }

    /// Returns a list of [`Device`]s.
    /// This list is empty initially.
    /// Call [`SwitchBot::load_devices()`] to populate the list.
    pub fn devices(&self) -> &DeviceList {
        &self.devices
    }

    /// Load the device list from the SwitchBot API.
    pub async fn load_devices(&mut self) -> anyhow::Result<()> {
        let devices = self.service.load_devices().await?;
        self.devices = devices;
        Ok(())
    }
}
