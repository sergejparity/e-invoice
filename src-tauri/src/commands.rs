use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use walkdir::WalkDir;

#[tauri::command]
pub async fn pick_folder() -> Result<Option<String>, String> {
    use tauri::api::dialog::blocking::FileDialogBuilder;

    let folder = FileDialogBuilder::new()
        .set_directory("/Users")
        .pick_folder();

    Ok(folder.map(|p| p.to_string_lossy().to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceFile {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub path: String,
    pub valid: bool,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn scan_folder(dir: String) -> Result<Vec<InvoiceFile>, String> {
    let path = PathBuf::from(dir);
    if !path.exists() || !path.is_dir() {
        return Err("Provided path is not a directory".to_string());
    }
    let mut result = Vec::new();
    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            let p = entry.path();
            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("xml") {
                    let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    result.push(InvoiceFile {
                        path: p.display().to_string(),
                        size_bytes,
                    });
                }
            }
        }
    }
    Ok(result)
}

#[tauri::command]
pub async fn validate_invoices(paths: Vec<String>) -> Result<Vec<ValidationResult>, String> {
    let mut out = Vec::new();
    for p in paths {
        let xml = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
        let res = lat_einv_core::validation::validate(&xml);
        let (valid, errors) = match res {
            Ok(_) => (true, Vec::new()),
            Err(errs) => (false, errs),
        };
        out.push(ValidationResult {
            path: p,
            valid,
            errors,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRequest {
    pub paths: Vec<String>,
    pub sender: String,
    pub receiver: String,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueResponse {
    pub job_ids: Vec<String>,
}

#[tauri::command]
pub async fn enqueue_send(req: SendRequest) -> Result<EnqueueResponse, String> {
    let mut job_ids = Vec::new();
    for p in req.paths {
        let xml = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
        let job_id = queue::enqueue_send_job(&xml, &req.sender, &req.receiver, &req.profile)
            .await
            .map_err(|e| e.to_string())?;
        tracing::info!(%job_id, path=%p, "enqueued invoice");
        job_ids.push(job_id);
    }
    Ok(EnqueueResponse { job_ids })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    pub job_id: String,
    pub state: String,
    pub last_error: Option<String>,
    pub updated_at: String,
    pub transmission_id: Option<String>,
}

#[tauri::command]
pub async fn list_status() -> Result<Vec<JobStatus>, String> {
    let statuses = queue::list_status().map_err(|e| e.to_string())?;
    Ok(statuses
        .into_iter()
        .map(|s| JobStatus {
            job_id: s.job_id,
            state: s.state,
            last_error: s.last_error,
            updated_at: s.updated_at.to_rfc3339(),
            transmission_id: s.transmission_id,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub certificate_thumbprint: Option<String>,
    pub unifiedpost_address: Option<String>,
    pub from_title: Option<String>,
    pub from_eadrese: Option<String>,
}

#[tauri::command]
pub async fn get_settings() -> Result<Settings, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    Ok(Settings {
        certificate_thumbprint: cfg.certificate.thumbprint,
        unifiedpost_address: cfg.provider.base_url,
        from_title: cfg.sender.from_title,
        from_eadrese: cfg.sender.from_eadrese,
    })
}

#[tauri::command]
pub async fn update_settings(settings: Settings) -> Result<(), String> {
    let mut cfg = config::load().unwrap_or_default();

    cfg.certificate.thumbprint = settings.certificate_thumbprint;
    cfg.provider.base_url = settings.unifiedpost_address;
    cfg.sender.from_title = settings.from_title;
    cfg.sender.from_eadrese = settings.from_eadrese;

    config::store(&cfg).map_err(|e| e.to_string())?;
    tracing::info!("Settings updated");
    Ok(())
}
