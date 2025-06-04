use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{
    rc::Rc,
    time::{Instant, SystemTime},
};
use uuid::Uuid;

use super::*;

#[derive(Debug, Default)]
pub(crate) struct SwitchBotService {
    client: reqwest::Client,
    token: String,
    secret: String,
}

impl SwitchBotService {
    const HOST: &str = "https://api.switch-bot.com";

    pub fn new(token: &str, secret: &str) -> Rc<Self> {
        Rc::new(SwitchBotService {
            client: reqwest::Client::new(),
            token: token.to_string(),
            secret: secret.to_string(),
        })
    }

    pub async fn load_devices(self: &Rc<SwitchBotService>) -> anyhow::Result<DeviceList> {
        let start_time = Instant::now();
        let url = format!("{}/v1.1/devices", Self::HOST);
        let json: serde_json::Value = self
            .add_headers(self.client.get(url))?
            .send()
            .await?
            .json()
            .await?;
        log::trace!(
            "devices.json: {json:#?}: elapsed {:?}",
            start_time.elapsed()
        );

        let response: SwitchBotResponse<DeviceListResponse> = serde_json::from_value(json)?;
        // log::trace!("devices: {response:#?}");
        let mut devices = DeviceList::with_capacity(
            response.body.device_list.len() + response.body.infrared_remote_list.len(),
        );
        devices.extend(response.body.device_list);
        devices.extend(response.body.infrared_remote_list);
        for device in devices.iter_mut() {
            device.set_service(self);
        }
        Ok(devices)
    }

    pub(crate) async fn command(
        &self,
        device_id: &str,
        command: &CommandRequest,
    ) -> anyhow::Result<()> {
        let start_time = Instant::now();
        let url = format!("{}/v1.1/devices/{device_id}/commands", Self::HOST);
        let body = serde_json::to_value(command)?;
        log::debug!("command.request: {body}");
        let json: serde_json::Value = self
            .add_headers(self.client.post(url))?
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        log::trace!(
            "command.response: {json}: elapsed {:?}",
            start_time.elapsed()
        );

        let response: SwitchBotError = serde_json::from_value(json)?;
        // All statusCode other than 100 looks like errors.
        // https://github.com/OpenWonderLabs/SwitchBotAPI?tab=readme-ov-file#errors
        if response.status_code != 100 {
            return Err(response.into());
        }
        Ok(())
    }

    fn add_headers(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        let duration_since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let t = duration_since_epoch.as_millis().to_string();
        let nonce = Uuid::new_v4().to_string();

        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes())?;
        mac.update(self.token.as_bytes());
        mac.update(t.as_bytes());
        mac.update(nonce.as_bytes());
        let result = mac.finalize();
        let sign = STANDARD.encode(result.into_bytes());

        Ok(builder
            .header("Authorization", self.token.clone())
            .header("t", t)
            .header("sign", sign)
            .header("nonce", nonce))
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwitchBotResponse<T> {
    #[allow(dead_code)]
    pub status_code: u16,
    #[allow(dead_code)]
    pub message: String,
    pub body: T,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceListResponse {
    device_list: Vec<Device>,
    infrared_remote_list: Vec<Device>,
}

/// Error from the [SwitchBot API].
///
/// [SwitchBot API]: https://github.com/OpenWonderLabs/SwitchBotAPI
#[derive(Debug, thiserror::Error, serde::Deserialize)]
#[error("SwitchBot API error: {message} ({status_code})")]
#[serde(rename_all = "camelCase")]
pub struct SwitchBotError {
    status_code: u16,
    message: String,
}

impl<T> From<SwitchBotResponse<T>> for SwitchBotError {
    fn from(response: SwitchBotResponse<T>) -> Self {
        Self {
            status_code: response.status_code,
            message: response.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_from_json() -> anyhow::Result<()> {
        let json_no_body = serde_json::json!(
            {"message":"unknown command", "statusCode":160});
        let error: SwitchBotError = serde_json::from_value(json_no_body)?;
        assert_eq!(error.status_code, 160);
        assert_eq!(error.message, "unknown command");

        // Some responses have empty `body`. Ensure it's ignored.
        let json_with_body = serde_json::json!(
            {"message":"unknown command", "statusCode":160, "body":{}});
        let error: SwitchBotError = serde_json::from_value(json_with_body)?;
        assert_eq!(error.status_code, 160);
        assert_eq!(error.message, "unknown command");
        Ok(())
    }
}
