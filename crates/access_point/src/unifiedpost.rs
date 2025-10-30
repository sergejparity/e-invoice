use super::{AccessPointClient, DeliveryState, DeliveryStatus};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct UnifiedpostClient {
    pub base_url: String,
    pub auth: UnifiedpostAuth,
    http_client: reqwest::Client,
    access_token: Arc<RwLock<Option<String>>>,
}

#[derive(Clone)]
pub enum UnifiedpostAuth {
    ApiKey {
        key: String,
    },
    OAuth2 {
        client_id: String,
        client_secret: String,
        token_url: String,
    },
}

#[derive(Debug, Serialize)]
struct SubmitRequest {
    xml: String,
    sender_id: String,
    receiver_id: String,
    document_type: String,
}

#[derive(Debug, Deserialize)]
struct SubmitResponse {
    transmission_id: String,
    #[allow(dead_code)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    transmission_id: String,
    state: String,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct OAuth2TokenRequest {
    grant_type: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Deserialize)]
struct OAuth2TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    expires_in: Option<u64>,
}

impl UnifiedpostClient {
    pub fn new(base_url: String, auth: UnifiedpostAuth) -> Arc<Self> {
        Arc::new(Self {
            base_url,
            auth,
            http_client: reqwest::Client::new(),
            access_token: Arc::new(RwLock::new(None)),
        })
    }

    async fn get_auth_header(&self) -> Result<String> {
        match &self.auth {
            UnifiedpostAuth::ApiKey { key } => Ok(format!("Bearer {}", key)),
            UnifiedpostAuth::OAuth2 {
                client_id,
                client_secret,
                token_url,
            } => {
                // Check if we have a cached token
                {
                    let token_read = self.access_token.read().await;
                    if let Some(t) = token_read.as_ref() {
                        return Ok(format!("Bearer {}", t));
                    }
                }

                // Fetch new token
                let req_body = OAuth2TokenRequest {
                    grant_type: "client_credentials".to_string(),
                    client_id: client_id.clone(),
                    client_secret: client_secret.clone(),
                };

                let resp = self
                    .http_client
                    .post(token_url)
                    .json(&req_body)
                    .send()
                    .await
                    .context("Failed to request OAuth2 token")?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    bail!("OAuth2 token request failed: {} - {}", status, body);
                }

                let token_resp: OAuth2TokenResponse = resp
                    .json()
                    .await
                    .context("Failed to parse token response")?;

                // Cache the token
                {
                    let mut token_write = self.access_token.write().await;
                    *token_write = Some(token_resp.access_token.clone());
                }

                Ok(format!("Bearer {}", token_resp.access_token))
            }
        }
    }
}

#[async_trait]
impl AccessPointClient for UnifiedpostClient {
    async fn submit(
        &self,
        xml: &str,
        sender: &str,
        receiver: &str,
        profile: &str,
    ) -> Result<String> {
        let auth_header = self.get_auth_header().await?;
        let submit_url = format!("{}/api/v1/peppol/send", self.base_url);

        let payload = SubmitRequest {
            xml: xml.to_string(),
            sender_id: sender.to_string(),
            receiver_id: receiver.to_string(),
            document_type: profile.to_string(),
        };

        let resp = self
            .http_client
            .post(&submit_url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send invoice to Unifiedpost")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Unifiedpost submit failed: {} - {}", status, body);
        }

        let submit_resp: SubmitResponse = resp
            .json()
            .await
            .context("Failed to parse submit response")?;

        tracing::info!(
            transmission_id = %submit_resp.transmission_id,
            "Invoice submitted to Unifiedpost"
        );

        Ok(submit_resp.transmission_id)
    }

    async fn status(&self, transmission_id: &str) -> Result<DeliveryStatus> {
        let auth_header = self.get_auth_header().await?;
        let status_url = format!("{}/api/v1/peppol/status/{}", self.base_url, transmission_id);

        let resp = self
            .http_client
            .get(&status_url)
            .header("Authorization", auth_header)
            .send()
            .await
            .context("Failed to query status from Unifiedpost")?;

        if !resp.status().is_success() {
            let status_code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!(
                "Unifiedpost status query failed: {} - {}",
                status_code,
                body
            );
        }

        let status_resp: StatusResponse = resp
            .json()
            .await
            .context("Failed to parse status response")?;

        let state = match status_resp.state.to_lowercase().as_str() {
            "delivered" | "accepted" => DeliveryState::Delivered,
            "failed" | "rejected" => DeliveryState::Failed,
            "in_transit" | "sending" => DeliveryState::InFlight,
            _ => DeliveryState::Pending,
        };

        Ok(DeliveryStatus {
            transmission_id: status_resp.transmission_id,
            state,
            message: status_resp.message,
        })
    }
}
