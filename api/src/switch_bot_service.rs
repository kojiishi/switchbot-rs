use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{
    sync::Arc,
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

    pub fn new(token: &str, secret: &str) -> Arc<Self> {
        Arc::new(SwitchBotService {
            client: reqwest::Client::new(),
            token: token.to_string(),
            secret: secret.to_string(),
        })
    }

    pub async fn load_devices(self: &Arc<SwitchBotService>) -> anyhow::Result<DeviceList> {
        let url = format!("{}/v1.1/devices", Self::HOST);
        let request = self.client.get(url);
        let device_list = self.send_as::<DeviceListResponse>(request).await?;

        let mut devices = DeviceList::with_capacity(
            device_list.device_list.len() + device_list.infrared_remote_list.len(),
        );
        devices.extend(device_list.device_list);
        devices.extend(device_list.infrared_remote_list);
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
        let url = format!("{}/v1.1/devices/{device_id}/commands", Self::HOST);
        let body = serde_json::to_value(command)?;
        log::debug!("command.request: {body}");
        let request = self.client.post(url).json(&body);
        self.send_as_opt(request).await?;
        Ok(())
    }

    pub(crate) async fn status(&self, device_id: &str) -> anyhow::Result<Option<Device>> {
        let url = format!("{}/v1.1/devices/{device_id}/status", Self::HOST);
        let request = self.client.get(url);
        let body_json = self.send_as_json(request).await?;
        if let serde_json::Value::Object(object) = &body_json {
            // Hub Mini returns `"body":{}`. Make this not an error.
            if object.is_empty() {
                return Ok(None);
            }
        }
        let device: Device = serde_json::from_value(body_json)?;
        Ok(Some(device))
    }

    async fn send_as<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<T> {
        let body_json = self.send_as_json(request).await?;
        let body: T = serde_json::from_value(body_json)?;
        Ok(body)
    }

    async fn send_as_json(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<serde_json::Value> {
        let body_json = self
            .send_as_opt(request)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Missing `body`"))?;
        Ok(body_json)
    }

    async fn send_as_opt(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        let start_time = Instant::now();
        let response = self.add_headers(request)?.send().await?;
        log::trace!("response: {response:?}");
        response.error_for_status_ref()?;

        let json: serde_json::Value = response.json().await?;
        log::trace!("response.json: {json}: elapsed {:?}", start_time.elapsed());
        Self::body_from_json(json)
    }

    fn body_from_json(json: serde_json::Value) -> anyhow::Result<Option<serde_json::Value>> {
        // First, parse to `Option<serde_json::Value>` because the `body` may be
        // missing, or doesn't contain required fields.
        // The `SwitchBotError` should be raised even when the `body` failed to
        // deserialize.
        let response: SwitchBotResponse<Option<serde_json::Value>> = serde_json::from_value(json)?;

        // All statusCode other than 100 looks like errors.
        // https://github.com/OpenWonderLabs/SwitchBotAPI#errors
        if response.status_code != 100 {
            return Err(SwitchBotError::from(response).into());
        }
        Ok(response.body)
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
    fn body_from_json() {
        let result = SwitchBotService::body_from_json(
            serde_json::json!({"message":"OK", "statusCode":100, "body":{}}),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn body_from_json_error() {
        let result = SwitchBotService::body_from_json(
            serde_json::json!({"message":"error", "statusCode":500, "body":{}}),
        );
        assert!(result.is_err());
        let error = result.unwrap_err();
        let switch_bot_error = error.downcast_ref::<SwitchBotError>();
        assert!(switch_bot_error.is_some());
        assert_eq!(switch_bot_error.unwrap().status_code, 500);
    }

    #[test]
    fn body_from_json_no_body() {
        let result =
            SwitchBotService::body_from_json(serde_json::json!({"message":"OK", "statusCode":100}));
        assert!(result.is_ok());
        let body = result.unwrap();
        assert!(body.is_none());
    }

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
