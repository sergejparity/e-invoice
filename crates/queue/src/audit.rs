use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub event_type: String,
    pub job_id: String,
    pub invoice_hash: Option<String>,
    pub transmission_id: Option<String>,
    pub state: String,
    pub error: Option<String>,
    pub sender: Option<String>,
    pub receiver: Option<String>,
}

impl AuditEvent {
    pub fn new(event_type: &str, job_id: &str, state: &str) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            event_type: event_type.to_string(),
            job_id: job_id.to_string(),
            invoice_hash: None,
            transmission_id: None,
            state: state.to_string(),
            error: None,
            sender: None,
            receiver: None,
        }
    }

    pub fn with_hash(mut self, hash: String) -> Self {
        self.invoice_hash = Some(hash);
        self
    }

    pub fn with_transmission_id(mut self, transmission_id: String) -> Self {
        self.transmission_id = Some(transmission_id);
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }

    pub fn with_parties(mut self, sender: String, receiver: String) -> Self {
        self.sender = Some(sender);
        self.receiver = Some(receiver);
        self
    }
}

fn audit_log_path() -> PathBuf {
    PathBuf::from("audit.jsonl")
}

pub fn write_audit_event(event: &AuditEvent) -> Result<()> {
    let path = audit_log_path();
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

    let json = serde_json::to_string(event)?;
    writeln!(file, "{}", json)?;
    tracing::debug!(event_type=%event.event_type, job_id=%event.job_id, "Audit event written");
    Ok(())
}

