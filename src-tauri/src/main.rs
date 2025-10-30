#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;

use access_point::{
    mock::MockClient,
    unifiedpost::{UnifiedpostAuth, UnifiedpostClient},
    AccessPointClient,
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tauri=info".to_string());
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(env_filter))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn create_access_point_client() -> anyhow::Result<Arc<dyn AccessPointClient>> {
    let cfg = config::load().unwrap_or_default();

    match cfg.provider.kind.as_str() {
        "unifiedpost" => {
            let base_url = cfg
                .provider
                .base_url
                .ok_or_else(|| anyhow::anyhow!("Unifiedpost base_url not configured"))?;

            // Try API key first from env or keychain
            if let Ok(api_key) = std::env::var("UNIFIEDPOST_API_KEY")
                .or_else(|_| config::get_secret("unifiedpost_api_key"))
            {
                tracing::info!("Using Unifiedpost with API key auth");
                let auth = UnifiedpostAuth::ApiKey { key: api_key };
                return Ok(UnifiedpostClient::new(base_url, auth));
            }

            // Fall back to OAuth2
            let client_id = cfg
                .provider
                .client_id
                .ok_or_else(|| anyhow::anyhow!("Unifiedpost client_id not configured"))?;

            let client_secret = std::env::var("UNIFIEDPOST_CLIENT_SECRET")
                .or_else(|_| config::get_secret("unifiedpost_client_secret"))
                .map_err(|_| {
                    anyhow::anyhow!("Unifiedpost client_secret not found in env or keychain")
                })?;

            let token_url = cfg
                .provider
                .token_url
                .unwrap_or_else(|| format!("{}/oauth/token", base_url));

            tracing::info!("Using Unifiedpost with OAuth2 auth");
            let auth = UnifiedpostAuth::OAuth2 {
                client_id,
                client_secret,
                token_url,
            };
            Ok(UnifiedpostClient::new(base_url, auth))
        }
        _ => {
            tracing::info!("Using mock access point");
            Ok(MockClient::new())
        }
    }
}

fn main() {
    init_tracing();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::pick_folder,
            commands::scan_folder,
            commands::validate_invoices,
            commands::enqueue_send,
            commands::list_status,
            commands::get_settings,
            commands::update_settings
        ])
        .setup(|_app| {
            let client = create_access_point_client()?;
            queue::init(client)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
