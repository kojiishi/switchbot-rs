use std::{fmt::Display, rc::Rc};

use super::*;

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
    pub fn device_id(&self) -> &str {
        &self.deviceId
    }

    pub fn device_name(&self) -> &str {
        &self.deviceName
    }

    pub fn is_remote(&self) -> bool {
        !self.remoteType.is_empty()
    }

    pub fn device_type(&self) -> &str {
        &self.deviceType
    }

    pub fn remote_type(&self) -> &str {
        &self.remoteType
    }

    pub fn hub_device_id(&self) -> &str {
        &self.hubDeviceId
    }

    pub(crate) fn set_service(&mut self, service: Rc<SwitchBotService>) {
        self.service = service;
    }

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
