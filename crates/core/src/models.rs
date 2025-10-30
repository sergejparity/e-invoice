use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    InFlight,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceMetadata {
    pub invoice_id: String,
    pub sender: String,
    pub receiver: String,
    pub profile: String,
    pub sha256: String,
}
