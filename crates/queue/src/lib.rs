mod audit;

use access_point::{AccessPointClient, DeliveryState};
use anyhow::{anyhow, Result};
use audit::{write_audit_event, AuditEvent};
use chrono::{DateTime, Utc};
use lat_einv_core::parsing::compute_sha256_hex;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

static GLOBAL_QUEUE: OnceCell<Arc<Queue>> = OnceCell::new();

#[derive(Clone)]
struct Queue {
    db: Db,
    access_point: Arc<dyn AccessPointClient + 'static>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: String,
    pub state: String,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub transmission_id: Option<String>,
    pub invoice_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobPayload {
    xml: String,
    sender: String,
    receiver: String,
    profile: String,
}

impl Queue {
    fn new(db: Db, access_point: Arc<dyn AccessPointClient + 'static>) -> Self {
        Self { db, access_point }
    }

    fn jobs_tree(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("jobs")?)
    }

    fn payloads_tree(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("payloads")?)
    }

    async fn enqueue(&self, payload: JobPayload) -> Result<String> {
        let job_id = self.generate_job_id();
        let hash = compute_sha256_hex(&payload.xml);
        let now = Utc::now();
        let rec = JobRecord {
            job_id: job_id.clone(),
            state: "queued".to_string(),
            last_error: None,
            created_at: now,
            updated_at: now,
            transmission_id: None,
            invoice_hash: hash.clone(),
        };

        let jobs = self.jobs_tree()?;
        let payloads = self.payloads_tree()?;

        jobs.insert(job_id.as_bytes(), serde_json::to_vec(&rec)?)?;
        payloads.insert(job_id.as_bytes(), serde_json::to_vec(&payload)?)?;

        // Audit log
        let _ = write_audit_event(
            &AuditEvent::new("job_enqueued", &job_id, "queued")
                .with_hash(hash)
                .with_parties(payload.sender.clone(), payload.receiver.clone()),
        );

        self.dispatch(job_id.clone());
        Ok(job_id)
    }

    fn dispatch(&self, job_id: String) {
        let jobs = self.jobs_tree().expect("jobs tree");
        let payloads = self.payloads_tree().expect("payloads tree");
        let client = Arc::clone(&self.access_point);

        tokio::spawn(async move {
            if let Err(e) = Self::process_job(client, jobs, payloads, job_id.clone()).await {
                tracing::error!(job_id=%job_id, error=%e, "job processing failed");
            }
        });
    }

    async fn process_job(
        client: Arc<dyn AccessPointClient + 'static>,
        jobs: sled::Tree,
        payloads: sled::Tree,
        job_id: String,
    ) -> Result<()> {
        update_state(&jobs, &job_id, |rec| {
            rec.state = "in_flight".into();
            rec.updated_at = Utc::now();
            rec.last_error = None;
        })?;

        let payload_bytes = payloads
            .get(job_id.as_bytes())?
            .ok_or_else(|| anyhow!("payload missing"))?;
        let payload: JobPayload = serde_json::from_slice(&payload_bytes)?;

        let transmit_result = client
            .submit(
                &payload.xml,
                &payload.sender,
                &payload.receiver,
                &payload.profile,
            )
            .await;

        match transmit_result {
            Ok(transmission_id) => {
                update_state(&jobs, &job_id, |rec| {
                    rec.state = "sent".into();
                    rec.updated_at = Utc::now();
                    rec.transmission_id = Some(transmission_id.clone());
                })?;

                // Audit log
                let _ = write_audit_event(
                    &AuditEvent::new("invoice_submitted", &job_id, "sent")
                        .with_transmission_id(transmission_id.clone()),
                );

                // Simulate polling for delivery (mock client reports delivered immediately).
                sleep(Duration::from_millis(100)).await;

                let status_res = client.status(transmission_id.as_str()).await;
                match status_res {
                    Ok(status) => {
                        let final_state = match status.state {
                            DeliveryState::Delivered => "delivered",
                            DeliveryState::Failed => "failed",
                            DeliveryState::InFlight => "in_flight",
                            DeliveryState::Pending => "pending",
                        };

                        update_state(&jobs, &job_id, |rec| {
                            rec.state = final_state.into();
                            rec.updated_at = Utc::now();
                            rec.last_error = match status.state {
                                DeliveryState::Failed => status.message.clone(),
                                _ => None,
                            };
                        })?;

                        // Audit log
                        let mut event =
                            AuditEvent::new("delivery_status_updated", &job_id, final_state)
                                .with_transmission_id(transmission_id.clone());
                        if let DeliveryState::Failed = status.state {
                            if let Some(msg) = status.message {
                                event = event.with_error(msg);
                            }
                        }
                        let _ = write_audit_event(&event);
                    }
                    Err(err) => {
                        update_state(&jobs, &job_id, |rec| {
                            rec.state = "failed".into();
                            rec.updated_at = Utc::now();
                            rec.last_error = Some(format!("status error: {err}"));
                        })?;

                        // Audit log
                        let _ = write_audit_event(
                            &AuditEvent::new("delivery_status_error", &job_id, "failed")
                                .with_error(err.to_string()),
                        );
                    }
                }
            }
            Err(err) => {
                update_state(&jobs, &job_id, |rec| {
                    rec.state = "failed".into();
                    rec.updated_at = Utc::now();
                    rec.last_error = Some(err.to_string());
                })?;

                // Audit log
                let _ = write_audit_event(
                    &AuditEvent::new("submission_failed", &job_id, "failed")
                        .with_error(err.to_string()),
                );
            }
        }

        Ok(())
    }

    fn generate_job_id(&self) -> String {
        use rand::{distributions::Alphanumeric, Rng};
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect()
    }

    fn list(&self) -> Result<Vec<JobRecord>> {
        let jobs = self.jobs_tree()?;
        let mut out = Vec::new();
        for item in jobs.iter() {
            let (_k, v) = item?;
            let rec: JobRecord = serde_json::from_slice(&v)?;
            out.push(rec);
        }
        out.sort_by_key(|r| r.created_at);
        out.reverse();
        Ok(out)
    }
}

fn update_state<F>(jobs: &sled::Tree, job_id: &str, mut f: F) -> Result<()>
where
    F: FnMut(&mut JobRecord),
{
    let key = job_id.as_bytes();
    let existing = jobs
        .get(key)?
        .ok_or_else(|| anyhow!("job not found: {job_id}"))?;
    let mut rec: JobRecord = serde_json::from_slice(&existing)?;
    f(&mut rec);
    jobs.insert(key, serde_json::to_vec(&rec)?)?;
    Ok(())
}

pub fn init(access_point: Arc<dyn AccessPointClient + 'static>) -> Result<()> {
    let db = sled::open(".einv_queue")?;
    let queue = Arc::new(Queue::new(db, access_point));
    GLOBAL_QUEUE
        .set(queue)
        .map_err(|_| anyhow!("queue already initialized"))?;
    Ok(())
}

pub async fn enqueue_send_job(
    xml: &str,
    sender: &str,
    receiver: &str,
    profile: &str,
) -> Result<String> {
    let payload = JobPayload {
        xml: xml.to_string(),
        sender: sender.to_string(),
        receiver: receiver.to_string(),
        profile: profile.to_string(),
    };
    let queue = GLOBAL_QUEUE
        .get()
        .ok_or_else(|| anyhow!("queue not initialized"))?;
    queue.enqueue(payload).await
}

pub fn list_status() -> Result<Vec<JobRecord>> {
    let queue = GLOBAL_QUEUE
        .get()
        .ok_or_else(|| anyhow!("queue not initialized"))?;
    queue.list()
}
