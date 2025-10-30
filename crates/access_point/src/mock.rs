use super::{AccessPointClient, DeliveryState, DeliveryStatus};
use anyhow::Result;
use async_trait::async_trait;
use rand::{distributions::Alphanumeric, Rng};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[derive(Clone, Default)]
pub struct MockClient;

impl MockClient {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

#[async_trait]
impl AccessPointClient for MockClient {
    async fn submit(
        &self,
        _xml: &str,
        _sender: &str,
        _receiver: &str,
        _profile: &str,
    ) -> Result<String> {
        let id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        // simulate network latency
        sleep(Duration::from_millis(200)).await;
        Ok(id)
    }

    async fn status(&self, transmission_id: &str) -> Result<DeliveryStatus> {
        Ok(DeliveryStatus {
            transmission_id: transmission_id.to_string(),
            state: DeliveryState::Delivered,
            message: Some("Mock delivered".to_string()),
        })
    }
}
