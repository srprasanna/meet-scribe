//! ASR (Automatic Speech Recognition) service adapters
//!
//! This module provides adapters for different ASR providers:
//! - AssemblyAI: File upload with polling (batch) and WebSocket (streaming)
//! - Deepgram: REST API (batch) and WebSocket (streaming)

pub mod assemblyai;
pub mod deepgram;
mod deepgram_streaming;

pub use assemblyai::AssemblyAIService;
pub use deepgram::DeepgramService;

use crate::adapters::storage::SqliteStorage;
use crate::error::{AppError, Result};
use crate::ports::storage::StoragePort;
use crate::ports::transcription::TranscriptionServicePort;
use crate::utils::keychain::KeychainManager;
use keyring::Entry;

/// Get the active ASR service based on service configuration
///
/// Queries the database for the active ASR provider and creates the appropriate service
/// with the API key from the keychain.
pub async fn get_active_asr_service(
    storage: &SqliteStorage,
    _keychain: &KeychainManager,
) -> Result<Box<dyn TranscriptionServicePort>> {
    // Query for active ASR service
    let configs = storage.list_service_configs().await?;
    let asr_config = configs
        .iter()
        .find(|c| c.service_type.to_string() == "asr" && c.is_active)
        .ok_or_else(|| AppError::Config("No active ASR service configured".to_string()))?;

    // Get API key from keychain
    let keychain_key = format!("asr_{}", asr_config.provider);
    let entry = Entry::new("com.srprasanna.meet-scribe", &keychain_key)
        .map_err(|e| AppError::Config(format!("Failed to access keychain: {}", e)))?;
    let api_key = entry
        .get_password()
        .map_err(|e| AppError::Config(format!("ASR API key not found: {}", e)))?;

    // Create appropriate service instance
    match asr_config.provider.as_str() {
        "assemblyai" => Ok(Box::new(AssemblyAIService::new(api_key))),
        "deepgram" => Ok(Box::new(DeepgramService::new(api_key))),
        _ => Err(AppError::Config(format!(
            "Unknown ASR provider: {}",
            asr_config.provider
        ))),
    }
}
