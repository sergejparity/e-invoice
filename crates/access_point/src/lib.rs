use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryState {
    Pending,
    InFlight,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryStatus {
    pub transmission_id: String,
    pub state: DeliveryState,
    pub message: Option<String>,
}

#[async_trait]
pub trait AccessPointClient: Send + Sync {
    async fn submit(
        &self,
        xml: &str,
        sender: &str,
        receiver: &str,
        profile: &str,
    ) -> Result<String>;
    async fn status(&self, transmission_id: &str) -> Result<DeliveryStatus>;
}

pub mod mock;
pub mod unifiedpost;
pub mod div_service;
pub mod div_types;
