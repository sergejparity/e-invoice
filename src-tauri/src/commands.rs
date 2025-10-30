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
