//! Google Gemini LLM service adapter
//!
//! Implements the LlmServicePort for Google's Gemini API
//! Supports dynamic model fetching and customizable prompts.

use crate::domain::models::InsightType;
use crate::error::{AppError, Result};
use crate::ports::llm::{GeneratedInsight, InsightRequest, LlmConfig, LlmServicePort, ModelInfo};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const GOOGLE_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Google Gemini service implementation
pub struct GoogleService {
    client: Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct GoogleModel {
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
    description: Option<String>,
    #[serde(rename = "inputTokenLimit")]
    input_token_limit: Option<u32>,
    #[serde(rename = "outputTokenLimit")]
    output_token_limit: Option<u32>,
    #[serde(rename = "supportedGenerationMethods")]
    supported_generation_methods: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleModelsResponse {
    models: Vec<GoogleModel>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
    index: u32,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
    role: String,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

impl GoogleService {
    /// Create a new Google Gemini service with the given API key
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }

    /// Fetch available models from Google API
    pub async fn list_models(&self) -> Result<Vec<GoogleModel>> {
        log::info!("Fetching available models from Google");

        let response = self
            .client
            .get(format!("{}/models", GOOGLE_API_BASE))
            .query(&[("key", &self.api_key)])
            .send()
            .await
            .map_err(|e| AppError::LlmService(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::LlmService(format!(
                "Failed to fetch models: {}",
                error_text
            )));
        }

        let models_response: GoogleModelsResponse = response
            .json()
            .await
            .map_err(|e| AppError::LlmService(format!("Failed to parse models response: {}", e)))?;

        // Filter to only models that support generateContent
        let gemini_models: Vec<GoogleModel> = models_response
            .models
            .into_iter()
            .filter(|m| {
                m.name.contains("gemini")
                    && m.supported_generation_methods
                        .contains(&"generateContent".to_string())
            })
            .collect();

        log::info!("Found {} Google Gemini models", gemini_models.len());
        Ok(gemini_models)
    }

    /// Generate text using generateContent API
    async fn generate_with_prompt(
        &self,
        prompt: &str,
        transcript: &str,
        context: Option<&str>,
        config: &LlmConfig,
    ) -> Result<String> {
        // Replace placeholders in prompt
        let context_str = context.unwrap_or("");
        let formatted_prompt = prompt
            .replace("{transcript}", transcript)
            .replace("{context}", context_str);

        let contents = vec![Content {
            parts: vec![Part {
                text: formatted_prompt,
            }],
        }];

        let generation_config = Some(GenerationConfig {
            temperature: config.temperature,
            max_output_tokens: config.max_tokens,
        });

        let request_body = GenerateContentRequest {
            contents,
            generation_config,
        };

        // Extract model name from full path if needed (e.g., "models/gemini-pro" -> "gemini-pro")
        let model_name = if config.model.starts_with("models/") {
            config.model.clone()
        } else {
            format!("models/{}", config.model)
        };

        log::info!("Calling Google generateContent with model: {}", model_name);

        let response = self
            .client
            .post(format!(
                "{}/{}:generateContent",
                GOOGLE_API_BASE, model_name
            ))
            .query(&[("key", &self.api_key)])
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::LlmService(format!("GenerateContent request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::LlmService(format!(
                "GenerateContent failed: {}",
                error_text
            )));
        }

        let content_response: GenerateContentResponse = response.json().await.map_err(|e| {
            AppError::LlmService(format!("Failed to parse content response: {}", e))
        })?;

        if content_response.candidates.is_empty() {
            return Err(AppError::LlmService("No candidates returned".to_string()));
        }

        if content_response.candidates[0].content.parts.is_empty() {
            return Err(AppError::LlmService(
                "No content parts in response".to_string(),
            ));
        }

        let content = content_response.candidates[0].content.parts[0].text.clone();
        log::info!(
            "Google completion successful, generated {} characters",
            content.len()
        );

        Ok(content)
    }

    /// Get estimated context window for a model
    fn get_context_window(model_id: &str, input_limit: Option<u32>) -> usize {
        // Use provided input limit if available
        if let Some(limit) = input_limit {
            return limit as usize;
        }

        // Fall back to known context windows
        if model_id.contains("gemini-1.5-pro") {
            2097152 // 2M tokens
        } else if model_id.contains("gemini-1.5-flash") {
            1048576 // 1M tokens
        } else if model_id.contains("gemini-pro") {
            32768 // 32k tokens
        } else {
            32768 // Default
        }
    }
}

#[async_trait]
impl LlmServicePort for GoogleService {
    async fn generate_insights(
        &self,
        request: &InsightRequest,
        config: &LlmConfig,
        prompt_template: Option<&str>,
    ) -> Result<Vec<GeneratedInsight>> {
        let mut insights = Vec::new();

        for insight_type in &request.insight_types {
            // Use custom prompt or fall back to default
            let prompt = if let Some(template) = prompt_template {
                template.to_string()
            } else {
                crate::domain::PromptTemplates::for_type(insight_type).to_string()
            };

            let content = self
                .generate_with_prompt(
                    &prompt,
                    &request.transcript,
                    request.context.as_deref(),
                    config,
                )
                .await?;

            insights.push(GeneratedInsight {
                insight_type: insight_type.clone(),
                content,
                metadata: None,
            });
        }

        Ok(insights)
    }

    async fn generate_summary(
        &self,
        transcript: &str,
        context: Option<&str>,
        config: &LlmConfig,
        prompt_template: Option<&str>,
    ) -> Result<String> {
        let prompt = if let Some(template) = prompt_template {
            template.to_string()
        } else {
            crate::domain::PromptTemplates::summary().to_string()
        };

        self.generate_with_prompt(&prompt, transcript, context, config)
            .await
    }

    async fn fetch_available_models(&self) -> Result<Vec<ModelInfo>> {
        let models = self.list_models().await?;

        Ok(models
            .into_iter()
            .map(|m| {
                // Extract just the model name from "models/gemini-pro" format
                let id = m
                    .name
                    .strip_prefix("models/")
                    .unwrap_or(&m.name)
                    .to_string();

                let is_fallback = m.input_token_limit.is_none();
                ModelInfo {
                    id: id.clone(),
                    name: m.display_name,
                    provider: "google".to_string(),
                    context_window: Self::get_context_window(&id, m.input_token_limit),
                    is_fallback_context_window: if is_fallback { Some(true) } else { None },
                }
            })
            .collect())
    }

    fn provider_name(&self) -> &str {
        "google"
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_service_creation() {
        let service = GoogleService::new("test_api_key".to_string());
        assert_eq!(service.provider_name(), "google");
        assert!(service.is_configured());
    }

    #[test]
    fn test_google_service_not_configured() {
        let service = GoogleService::new("".to_string());
        assert!(!service.is_configured());
    }

    #[test]
    fn test_context_window_estimation() {
        assert_eq!(
            GoogleService::get_context_window("gemini-1.5-pro", None),
            2097152
        );
        assert_eq!(
            GoogleService::get_context_window("gemini-1.5-flash", None),
            1048576
        );
        assert_eq!(GoogleService::get_context_window("gemini-pro", None), 32768);

        // Test with explicit limit
        assert_eq!(
            GoogleService::get_context_window("gemini-pro", Some(100000)),
            100000
        );
    }
}
