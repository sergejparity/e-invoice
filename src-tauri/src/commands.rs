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
    pub provider_kind: Option<String>,
    pub certificate_thumbprint: Option<String>,
    pub unifiedpost_address: Option<String>,
    pub from_title: Option<String>,
    pub from_eadrese: Option<String>,
}

#[tauri::command]
pub async fn get_settings() -> Result<Settings, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    Ok(Settings {
        provider_kind: Some(cfg.provider.kind),
        certificate_thumbprint: cfg.certificate.thumbprint,
        unifiedpost_address: cfg.provider.base_url,
        from_title: cfg.sender.from_title,
        from_eadrese: cfg.sender.from_eadrese,
    })
}

#[tauri::command]
pub async fn update_settings(settings: Settings) -> Result<(), String> {
    let mut cfg = config::load().unwrap_or_default();

    if let Some(kind) = settings.provider_kind {
        cfg.provider.kind = kind;
    }
    cfg.certificate.thumbprint = settings.certificate_thumbprint;
    cfg.provider.base_url = settings.unifiedpost_address;
    cfg.sender.from_title = settings.from_title;
    cfg.sender.from_eadrese = settings.from_eadrese;

    config::store(&cfg).map_err(|e| e.to_string())?;
    tracing::info!("Settings updated");
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
}

/// Test connection to the configured service provider
#[tauri::command]
pub async fn test_connection() -> Result<ConnectionTestResult, String> {
    let cfg = config::load().unwrap_or_default();
    let provider_kind = cfg.provider.kind.as_str();

    match provider_kind {
        "mock" => {
            // Mock always succeeds - it doesn't require any connection
            Ok(ConnectionTestResult {
                success: true,
                message: "Mock provider is always available (no actual connection)".to_string(),
            })
        }
        "div" => {
            // Validate DIV configuration
            let base_url = match cfg.provider.base_url {
                Some(url) if !url.is_empty() => url,
                _ => {
                    return Ok(ConnectionTestResult {
                        success: false,
                        message: "Service address is required".to_string(),
                    });
                }
            };

            let cert_thumbprint = match cfg.certificate.thumbprint {
                Some(thumb) if !thumb.is_empty() => thumb,
                _ => {
                    return Ok(ConnectionTestResult {
                        success: false,
                        message: "Certificate thumbprint is required".to_string(),
                    });
                }
            };

            let sender_eaddress = match cfg.sender.from_eadrese {
                Some(addr) if !addr.is_empty() => addr,
                _ => {
                    return Ok(ConnectionTestResult {
                        success: false,
                        message: "Sender e-adrese is required".to_string(),
                    });
                }
            };

            // Try to create the client (validates configuration structure)
            match access_point::div_service::DivServiceClient::new(
                base_url.clone(),
                cert_thumbprint,
                sender_eaddress,
            ) {
                _client => {
                    // Client created successfully - configuration is valid
                    // Note: Actual network connection test would require:
                    // 1. Certificate loading from file/keychain
                    // 2. TLS client certificate setup
                    // 3. SOAP signing implementation
                    // For now, we only validate configuration completeness
                    
                    tracing::info!("DIV configuration validated successfully");
                    Ok(ConnectionTestResult {
                        success: true,
                        message: format!(
                            "Configuration validated. Note: Full connection test requires certificates and SOAP signing to be implemented."
                        ),
                    })
                }
            }
        }
        "unifiedpost" => {
            // Validate Unifiedpost configuration
            let base_url = match cfg.provider.base_url {
                Some(url) if !url.is_empty() => url,
                _ => {
                    return Ok(ConnectionTestResult {
                        success: false,
                        message: "Service address is required".to_string(),
                    });
                }
            };

            // Check if API key is available
            let has_api_key = std::env::var("UNIFIEDPOST_API_KEY")
                .or_else(|_| config::get_secret("unifiedpost_api_key"))
                .is_ok();

            // Check if OAuth2 credentials are available
            let has_oauth2 = if let Some(ref client_id) = cfg.provider.client_id {
                !client_id.is_empty()
                    && (std::env::var("UNIFIEDPOST_CLIENT_SECRET").is_ok()
                        || config::get_secret("unifiedpost_client_secret").is_ok())
            } else {
                false
            };

            if !has_api_key && !has_oauth2 {
                return Ok(ConnectionTestResult {
                    success: false,
                    message: "Authentication credentials required. Set UNIFIEDPOST_API_KEY or configure OAuth2 (client_id and UNIFIEDPOST_CLIENT_SECRET)".to_string(),
                });
            }

            // Try to create the client
            if has_api_key {
                let api_key = std::env::var("UNIFIEDPOST_API_KEY")
                    .or_else(|_| config::get_secret("unifiedpost_api_key"))
                    .map_err(|_| "Failed to retrieve API key".to_string())?;
                
                let auth = access_point::unifiedpost::UnifiedpostAuth::ApiKey { key: api_key };
                let _client = access_point::unifiedpost::UnifiedpostClient::new(base_url, auth);
                
                Ok(ConnectionTestResult {
                    success: true,
                    message: "Configuration validated with API key authentication".to_string(),
                })
            } else {
                let client_id = cfg.provider.client_id.unwrap();
                let client_secret = std::env::var("UNIFIEDPOST_CLIENT_SECRET")
                    .or_else(|_| config::get_secret("unifiedpost_client_secret"))
                    .map_err(|_| "Failed to retrieve client secret".to_string())?;
                
                let token_url = cfg.provider.token_url
                    .unwrap_or_else(|| format!("{}/oauth/token", base_url));
                
                let auth = access_point::unifiedpost::UnifiedpostAuth::OAuth2 {
                    client_id,
                    client_secret,
                    token_url,
                };
                let _client = access_point::unifiedpost::UnifiedpostClient::new(base_url, auth);
                
                Ok(ConnectionTestResult {
                    success: true,
                    message: "Configuration validated with OAuth2 authentication".to_string(),
                })
            }
        }
        _ => {
            Ok(ConnectionTestResult {
                success: false,
                message: format!("Unknown provider type: {}", provider_kind),
            })
        }
    }
}
