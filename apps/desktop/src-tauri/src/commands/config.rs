//! Configuration and API key management commands

use crate::domain::models::{ServiceConfig, ServiceType};
use crate::ports::storage::StoragePort;
use crate::utils::keychain::KeychainPort;
use crate::AppState;
use serde::{Deserialize, Serialize};

/// Request to save an API key
#[derive(Debug, Deserialize)]
pub struct SaveApiKeyRequest {
    pub service_type: String, // "asr" or "llm"
    pub provider: String,     // "deepgram", "assemblyai", "openai", etc.
    pub api_key: String,
}

/// Request to get an API key (returns masked version)
#[derive(Debug, Deserialize)]
pub struct GetApiKeyRequest {
    pub service_type: String,
    pub provider: String,
}

/// Response for API key status
#[derive(Debug, Serialize)]
pub struct ApiKeyStatus {
    pub has_key: bool,
    pub masked_key: Option<String>, // Shows last 4 chars: "sk-...abc123"
}

/// Request to save service configuration
#[derive(Debug, Deserialize)]
pub struct SaveServiceConfigRequest {
    pub service_type: String,
    pub provider: String,
    pub is_active: bool,
    pub settings: Option<String>, // JSON string of provider-specific settings
}

/// Response with service configuration
#[derive(Debug, Serialize)]
pub struct ServiceConfigResponse {
    pub id: Option<i64>,
    pub service_type: String,
    pub provider: String,
    pub is_active: bool,
    pub settings: Option<String>,
    pub has_api_key: bool,
}

/// Saves an API key to the OS keychain
///
/// The API key is stored securely using platform-specific mechanisms:
/// - Windows: Windows Credential Manager
/// - Linux: Secret Service (GNOME Keyring, KWallet)
#[tauri::command]
pub async fn save_api_key(
    state: tauri::State<'_, AppState>,
    request: SaveApiKeyRequest,
) -> Result<(), String> {
    state
        .keychain
        .save_api_key(&request.service_type, &request.provider, &request.api_key)
        .map_err(|e| e.to_string())
}

/// Checks if an API key exists and returns a masked version
///
/// For security, the full key is never returned. Instead, we return:
/// - `has_key`: true if key exists
/// - `masked_key`: Last 4 characters only (e.g., "...abc123")
#[tauri::command]
pub async fn get_api_key_status(
    state: tauri::State<'_, AppState>,
    request: GetApiKeyRequest,
) -> Result<ApiKeyStatus, String> {
    match state
        .keychain
        .get_api_key(&request.service_type, &request.provider)
    {
        Ok(key) => {
            // Mask the key - show only last 4 characters
            let masked = if key.len() > 4 {
                format!("...{}", &key[key.len() - 4..])
            } else {
                "...".to_string()
            };

            Ok(ApiKeyStatus {
                has_key: true,
                masked_key: Some(masked),
            })
        }
        Err(_) => Ok(ApiKeyStatus {
            has_key: false,
            masked_key: None,
        }),
    }
}

/// Deletes an API key from the OS keychain
#[tauri::command]
pub async fn delete_api_key(
    state: tauri::State<'_, AppState>,
    service_type: String,
    provider: String,
) -> Result<(), String> {
    state
        .keychain
        .delete_api_key(&service_type, &provider)
        .map_err(|e| e.to_string())
}

/// Saves service configuration to the database
///
/// This stores provider settings (model, language, etc.) but NOT API keys.
/// API keys are stored separately in the OS keychain.
#[tauri::command]
pub async fn save_service_config(
    state: tauri::State<'_, AppState>,
    request: SaveServiceConfigRequest,
) -> Result<i64, String> {
    // Parse service type
    let service_type = match request.service_type.as_str() {
        "asr" => ServiceType::Asr,
        "llm" => ServiceType::Llm,
        _ => {
            return Err(format!(
                "Invalid service type: {}. Must be 'asr' or 'llm'",
                request.service_type
            ))
        }
    };

    // Create service config
    let config = ServiceConfig::new(service_type, request.provider.clone())
        .with_active(request.is_active)
        .with_settings(request.settings);

    // Save to database
    state
        .storage
        .save_service_config(&config)
        .await
        .map_err(|e| e.to_string())
}

/// Gets a specific service configuration
#[tauri::command]
pub async fn get_service_config(
    state: tauri::State<'_, AppState>,
    service_type: String,
    provider: String,
) -> Result<Option<ServiceConfigResponse>, String> {
    let config = state
        .storage
        .get_service_config(&service_type, &provider)
        .await
        .map_err(|e| e.to_string())?;

    match config {
        Some(cfg) => {
            // Check if API key exists
            let has_api_key = state.keychain.has_api_key(&service_type, &provider);

            Ok(Some(ServiceConfigResponse {
                id: cfg.id,
                service_type: format!("{:?}", cfg.service_type).to_lowercase(),
                provider: cfg.provider,
                is_active: cfg.is_active,
                settings: cfg.settings,
                has_api_key,
            }))
        }
        None => Ok(None),
    }
}

/// Gets the currently active service configuration for a service type
#[tauri::command]
pub async fn get_active_service_config(
    state: tauri::State<'_, AppState>,
    service_type: String,
) -> Result<Option<ServiceConfigResponse>, String> {
    let config = state
        .storage
        .get_active_service_config(&service_type)
        .await
        .map_err(|e| e.to_string())?;

    match config {
        Some(cfg) => {
            let has_api_key = state.keychain.has_api_key(&service_type, &cfg.provider);

            Ok(Some(ServiceConfigResponse {
                id: cfg.id,
                service_type: format!("{:?}", cfg.service_type).to_lowercase(),
                provider: cfg.provider,
                is_active: cfg.is_active,
                settings: cfg.settings,
                has_api_key,
            }))
        }
        None => Ok(None),
    }
}

/// Lists all service configurations
#[tauri::command]
pub async fn list_service_configs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ServiceConfigResponse>, String> {
    let configs = state
        .storage
        .list_service_configs()
        .await
        .map_err(|e| e.to_string())?;

    let mut responses = Vec::new();
    for cfg in configs {
        let service_type_str = format!("{:?}", cfg.service_type).to_lowercase();
        let has_api_key = state.keychain.has_api_key(&service_type_str, &cfg.provider);

        responses.push(ServiceConfigResponse {
            id: cfg.id,
            service_type: service_type_str,
            provider: cfg.provider,
            is_active: cfg.is_active,
            settings: cfg.settings,
            has_api_key,
        });
    }

    Ok(responses)
}

/// Activates a specific service configuration (deactivates others of same type)
#[tauri::command]
pub async fn activate_service(
    state: tauri::State<'_, AppState>,
    service_type: String,
    provider: String,
) -> Result<(), String> {
    // Check if API key exists first
    if !state.keychain.has_api_key(&service_type, &provider) {
        return Err(format!(
            "Cannot activate service without API key. Please add an API key for {}:{}",
            service_type, provider
        ));
    }

    // Get or create the configuration
    let config = state
        .storage
        .get_service_config(&service_type, &provider)
        .await
        .map_err(|e| e.to_string())?;

    // If config doesn't exist, create a default one
    if config.is_none() {
        log::info!(
            "Creating default configuration for {}:{}",
            service_type,
            provider
        );

        let service_type_enum = match service_type.as_str() {
            "asr" => ServiceType::Asr,
            "llm" => ServiceType::Llm,
            _ => {
                return Err(format!(
                    "Invalid service type: {}. Must be 'asr' or 'llm'",
                    service_type
                ))
            }
        };

        let default_config =
            ServiceConfig::new(service_type_enum, provider.clone()).with_active(false); // Will be activated below

        state
            .storage
            .save_service_config(&default_config)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Deactivate all services of this type
    let all_configs = state
        .storage
        .list_service_configs()
        .await
        .map_err(|e| e.to_string())?;

    for mut cfg in all_configs {
        let cfg_type_str = format!("{:?}", cfg.service_type).to_lowercase();
        if cfg_type_str == service_type {
            if cfg.provider == provider {
                cfg.is_active = true;
            } else {
                cfg.is_active = false;
            }
            state
                .storage
                .save_service_config(&cfg)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
