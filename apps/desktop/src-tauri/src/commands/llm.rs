//! LLM and prompt management Tauri commands
//!
//! Provides IPC interface for:
//! - Fetching available models from LLM providers
//! - Managing LLM service configurations
//! - Managing custom prompt templates
//! - Generating insights from transcripts

use crate::adapters::services::llm::{AnthropicService, GoogleService, GroqService, OpenAIService};
use crate::domain::models::InsightType;
use crate::domain::PromptTemplates;
use crate::ports::llm::{InsightRequest, LlmConfig, LlmServicePort, ModelInfo};
use crate::utils::keychain::KeychainPort;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Request to fetch models from a specific provider
#[derive(Debug, Deserialize)]
pub struct FetchModelsRequest {
    pub provider: String, // "openai", "anthropic", "google", "groq"
}

/// Response containing available models
#[derive(Debug, Serialize)]
pub struct FetchModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// Request to save API key for a provider
#[derive(Debug, Deserialize)]
pub struct SaveApiKeyRequest {
    pub provider: String,
    pub api_key: String,
}

/// Request to generate insights
#[derive(Debug, Deserialize)]
pub struct GenerateInsightsRequest {
    pub provider: String,
    pub model: String,
    pub transcript: String,
    pub context: Option<String>,
    pub insight_types: Vec<InsightType>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub custom_prompt: Option<String>,
}

/// Response containing generated insights
#[derive(Debug, Serialize)]
pub struct GenerateInsightsResponse {
    pub insights: Vec<InsightResponse>,
}

#[derive(Debug, Serialize)]
pub struct InsightResponse {
    pub insight_type: InsightType,
    pub content: String,
}

/// Request to get default prompts
#[derive(Debug, Deserialize)]
pub struct GetDefaultPromptsRequest {
    pub insight_type: Option<InsightType>,
}

/// Response containing default prompts
#[derive(Debug, Serialize)]
pub struct GetDefaultPromptsResponse {
    pub prompts: Vec<PromptInfo>,
}

#[derive(Debug, Serialize)]
pub struct PromptInfo {
    pub insight_type: InsightType,
    pub prompt: String,
}

/// Fetch available models from a specific LLM provider
#[tauri::command]
pub async fn fetch_llm_models(
    request: FetchModelsRequest,
    state: State<'_, AppState>,
) -> Result<FetchModelsResponse, String> {
    log::info!("Fetching models for provider: {}", request.provider);

    // Get API key from keychain
    let api_key = state
        .keychain
        .get_api_key("llm", &request.provider)
        .map_err(|e| e.to_string())?;

    // Create service based on provider
    let models = match request.provider.as_str() {
        "openai" => {
            let service = OpenAIService::new(api_key);
            service
                .fetch_available_models()
                .await
                .map_err(|e| e.to_string())?
        }
        "anthropic" => {
            let service = AnthropicService::new(api_key);
            service
                .fetch_available_models()
                .await
                .map_err(|e| e.to_string())?
        }
        "google" => {
            let service = GoogleService::new(api_key);
            service
                .fetch_available_models()
                .await
                .map_err(|e| e.to_string())?
        }
        "groq" => {
            let service = GroqService::new(api_key);
            service
                .fetch_available_models()
                .await
                .map_err(|e| e.to_string())?
        }
        _ => {
            return Err(format!("Unknown provider: {}", request.provider));
        }
    };

    log::info!(
        "Successfully fetched {} models for {}",
        models.len(),
        request.provider
    );

    Ok(FetchModelsResponse { models })
}

/// Save API key for an LLM provider
#[tauri::command]
pub async fn save_llm_api_key(
    request: SaveApiKeyRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Saving API key for provider: {}", request.provider);

    state
        .keychain
        .save_api_key("llm", &request.provider, &request.api_key)
        .map_err(|e| e.to_string())?;

    log::info!("API key saved successfully for {}", request.provider);
    Ok(())
}

/// Check if API key exists for a provider
#[tauri::command]
pub async fn check_llm_api_key(
    provider: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    log::info!("Checking API key for provider: {}", provider);

    match state.keychain.get_api_key("llm", &provider) {
        Ok(key) => Ok(!key.is_empty()),
        Err(_) => Ok(false),
    }
}

/// Delete API key for a provider
#[tauri::command]
pub async fn delete_llm_api_key(
    provider: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Deleting API key for provider: {}", provider);

    state
        .keychain
        .delete_api_key("llm", &provider)
        .map_err(|e| e.to_string())?;

    log::info!("API key deleted successfully for {}", provider);
    Ok(())
}

/// Generate insights from a transcript
#[tauri::command]
pub async fn generate_insights(
    request: GenerateInsightsRequest,
    state: State<'_, AppState>,
) -> Result<GenerateInsightsResponse, String> {
    log::info!(
        "Generating insights with provider: {}, model: {}",
        request.provider,
        request.model
    );

    // Get API key from keychain
    let api_key = state
        .keychain
        .get_api_key("llm", &request.provider)
        .map_err(|e| e.to_string())?;

    // Create LLM config
    let config = LlmConfig {
        model: request.model.clone(),
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        additional_settings: None,
    };

    // Create insight request
    let insight_request = InsightRequest {
        transcript: request.transcript,
        context: request.context,
        insight_types: request.insight_types,
    };

    // Generate insights based on provider
    let insights = match request.provider.as_str() {
        "openai" => {
            let service = OpenAIService::new(api_key);
            service
                .generate_insights(&insight_request, &config, request.custom_prompt.as_deref())
                .await
                .map_err(|e| e.to_string())?
        }
        "anthropic" => {
            let service = AnthropicService::new(api_key);
            service
                .generate_insights(&insight_request, &config, request.custom_prompt.as_deref())
                .await
                .map_err(|e| e.to_string())?
        }
        "google" => {
            let service = GoogleService::new(api_key);
            service
                .generate_insights(&insight_request, &config, request.custom_prompt.as_deref())
                .await
                .map_err(|e| e.to_string())?
        }
        "groq" => {
            let service = GroqService::new(api_key);
            service
                .generate_insights(&insight_request, &config, request.custom_prompt.as_deref())
                .await
                .map_err(|e| e.to_string())?
        }
        _ => {
            return Err(format!("Unknown provider: {}", request.provider));
        }
    };

    log::info!("Successfully generated {} insights", insights.len());

    Ok(GenerateInsightsResponse {
        insights: insights
            .into_iter()
            .map(|i| InsightResponse {
                insight_type: i.insight_type,
                content: i.content,
            })
            .collect(),
    })
}

/// Get default prompt templates
#[tauri::command]
pub async fn get_default_prompts(
    request: GetDefaultPromptsRequest,
) -> Result<GetDefaultPromptsResponse, String> {
    log::info!("Getting default prompts");

    let prompts = if let Some(insight_type) = request.insight_type {
        // Get specific prompt
        vec![PromptInfo {
            insight_type: insight_type.clone(),
            prompt: PromptTemplates::for_type(&insight_type).to_string(),
        }]
    } else {
        // Get all prompts
        PromptTemplates::all()
            .into_iter()
            .map(|(insight_type, prompt)| PromptInfo {
                insight_type,
                prompt: prompt.to_string(),
            })
            .collect()
    };

    Ok(GetDefaultPromptsResponse { prompts })
}

/// List all supported LLM providers
#[tauri::command]
pub async fn list_llm_providers() -> Result<Vec<String>, String> {
    Ok(vec![
        "openai".to_string(),
        "anthropic".to_string(),
        "google".to_string(),
        "groq".to_string(),
    ])
}

/// Request to generate and store insights for a meeting
#[derive(Debug, Deserialize)]
pub struct GenerateMeetingInsightsRequest {
    pub meeting_id: i64,
    pub provider: String,
    pub model: String,
    pub insight_types: Vec<InsightType>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// Response containing stored insights
#[derive(Debug, Serialize)]
pub struct MeetingInsightsResponse {
    pub insights: Vec<StoredInsight>,
}

#[derive(Debug, Serialize)]
pub struct StoredInsight {
    pub id: i64,
    pub meeting_id: i64,
    pub insight_type: InsightType,
    pub content: String,
    pub created_at: i64,
}

/// Generate insights for a meeting and store them in the database
#[tauri::command]
pub async fn generate_meeting_insights(
    request: GenerateMeetingInsightsRequest,
    state: State<'_, AppState>,
) -> Result<MeetingInsightsResponse, String> {
    use crate::domain::models::Insight;
    use crate::ports::storage::StoragePort;

    log::info!(
        "Generating insights for meeting {} with provider: {}, model: {}",
        request.meeting_id,
        request.provider,
        request.model
    );

    // Get transcripts for the meeting
    let transcripts = state
        .storage
        .get_transcripts(request.meeting_id)
        .await
        .map_err(|e| format!("Failed to get transcripts: {}", e))?;

    if transcripts.is_empty() {
        return Err("No transcripts found for this meeting".to_string());
    }

    // Reconstruct full transcript with speaker labels
    let full_transcript = transcripts
        .iter()
        .map(|t| {
            // Prefer participant_name over speaker_label
            if let Some(name) = &t.participant_name {
                format!("[{}]: {}", name, t.text)
            } else if let Some(speaker) = &t.speaker_label {
                format!("[{}]: {}", speaker, t.text)
            } else {
                t.text.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Get API key from keychain
    let api_key = state
        .keychain
        .get_api_key("llm", &request.provider)
        .map_err(|e| e.to_string())?;

    // Create LLM config
    let config = LlmConfig {
        model: request.model.clone(),
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        additional_settings: None,
    };

    // Create insight request
    let insight_request = InsightRequest {
        transcript: full_transcript,
        context: None,
        insight_types: request.insight_types.clone(),
    };

    // Generate insights based on provider
    let generated_insights = match request.provider.as_str() {
        "openai" => {
            let service = OpenAIService::new(api_key);
            service
                .generate_insights(&insight_request, &config, None)
                .await
                .map_err(|e| e.to_string())?
        }
        "anthropic" => {
            let service = AnthropicService::new(api_key);
            service
                .generate_insights(&insight_request, &config, None)
                .await
                .map_err(|e| e.to_string())?
        }
        "google" => {
            let service = GoogleService::new(api_key);
            service
                .generate_insights(&insight_request, &config, None)
                .await
                .map_err(|e| e.to_string())?
        }
        "groq" => {
            let service = GroqService::new(api_key);
            service
                .generate_insights(&insight_request, &config, None)
                .await
                .map_err(|e| e.to_string())?
        }
        _ => {
            return Err(format!("Unknown provider: {}", request.provider));
        }
    };

    // Store insights in database
    let mut stored_insights = Vec::new();
    for insight in generated_insights {
        let domain_insight = Insight::new(
            request.meeting_id,
            insight.insight_type.clone(),
            insight.content.clone(),
        );

        let id = state
            .storage
            .create_insight(&domain_insight)
            .await
            .map_err(|e| format!("Failed to store insight: {}", e))?;

        stored_insights.push(StoredInsight {
            id,
            meeting_id: request.meeting_id,
            insight_type: insight.insight_type,
            content: insight.content,
            created_at: domain_insight.created_at,
        });
    }

    log::info!(
        "Successfully generated and stored {} insights for meeting {}",
        stored_insights.len(),
        request.meeting_id
    );

    Ok(MeetingInsightsResponse {
        insights: stored_insights,
    })
}

/// Get stored insights for a meeting
#[tauri::command]
pub async fn get_meeting_insights(
    meeting_id: i64,
    state: State<'_, AppState>,
) -> Result<MeetingInsightsResponse, String> {
    use crate::ports::storage::StoragePort;

    log::info!("Getting insights for meeting {}", meeting_id);

    let insights = state
        .storage
        .get_insights(meeting_id)
        .await
        .map_err(|e| format!("Failed to get insights: {}", e))?;

    Ok(MeetingInsightsResponse {
        insights: insights
            .into_iter()
            .map(|i| StoredInsight {
                id: i.id.unwrap_or(0),
                meeting_id: i.meeting_id,
                insight_type: i.insight_type,
                content: i.content,
                created_at: i.created_at,
            })
            .collect(),
    })
}

/// Update an existing insight's content
///
/// This allows users to edit and refine AI-generated insights.
#[tauri::command]
pub async fn update_insight(
    insight_id: i64,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::ports::storage::StoragePort;

    log::info!("Updating insight {}", insight_id);

    state
        .storage
        .update_insight_content(insight_id, &content)
        .await
        .map_err(|e| format!("Failed to update insight: {}", e))
}

/// Delete all insights for a meeting
///
/// This allows regenerating insights by first deleting existing ones.
#[tauri::command]
pub async fn delete_meeting_insights(
    meeting_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::ports::storage::StoragePort;

    log::info!("Deleting insights for meeting {}", meeting_id);
    state
        .storage
        .delete_insights(meeting_id)
        .await
        .map_err(|e| format!("Failed to delete insights: {}", e))
}
