use std::rc::Rc;

use super::*;

#[derive(Debug, Default)]
pub struct SwitchBot {
    service: Rc<SwitchBotService>,
    devices: DeviceList,
}

impl SwitchBot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_authentication(token: &str, secret: &str) -> Self {
        Self {
            service: SwitchBotService::new(token, secret),
            ..Default::default()
        }
    }

    pub fn set_authentication(&mut self, token: &str, secret: &str) {
        self.service = SwitchBotService::new(token, secret);
    }

    pub fn devices(&self) -> &DeviceList {
        &self.devices
    }

    pub async fn load_devices(&mut self) -> anyhow::Result<()> {
        let devices = self.service.load_devices().await?;
        self.devices = devices;
        Ok(())
    }
}
