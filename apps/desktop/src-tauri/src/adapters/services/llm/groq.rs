//! Groq LLM service adapter
//!
//! Implements the LlmServicePort for Groq's API
//! Uses OpenAI-compatible API for easy integration
//! Supports dynamic model fetching and customizable prompts.

use crate::domain::models::InsightType;
use crate::error::{AppError, Result};
use crate::ports::llm::{GeneratedInsight, InsightRequest, LlmConfig, LlmServicePort, ModelInfo};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const GROQ_API_BASE: &str = "https://api.groq.com/openai/v1";

/// Groq service implementation
pub struct GroqService {
    client: Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct GroqModel {
    id: String,
    object: String,
    created: u64,
    owned_by: String,
    active: bool,
    context_window: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GroqModelsResponse {
    object: String,
    data: Vec<GroqModel>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    index: u32,
    message: ChatMessage,
    finish_reason: Option<String>,
}

impl GroqService {
    /// Create a new Groq service with the given API key
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }

    /// Fetch available models from Groq API
    pub async fn list_models(&self) -> Result<Vec<GroqModel>> {
        log::info!("Fetching available models from Groq");

        let response = self
            .client
            .get(format!("{}/models", GROQ_API_BASE))
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        let models_response: GroqModelsResponse = response
            .json()
            .await
            .map_err(|e| AppError::LlmService(format!("Failed to parse models response: {}", e)))?;

        // Filter to only active models
        let active_models: Vec<GroqModel> = models_response
            .data
            .into_iter()
            .filter(|m| m.active)
            .collect();

        log::info!("Found {} active Groq models", active_models.len());
        Ok(active_models)
    }

    /// Generate text using chat completion API (OpenAI-compatible)
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

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: formatted_prompt,
        }];

        let request_body = ChatCompletionRequest {
            model: config.model.clone(),
            messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        };

        log::info!("Calling Groq chat completion with model: {}", config.model);

        let response = self
            .client
            .post(format!("{}/chat/completions", GROQ_API_BASE))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::LlmService(format!("Chat completion request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::LlmService(format!(
                "Chat completion failed: {}",
                error_text
            )));
        }

        let completion_response: ChatCompletionResponse = response.json().await.map_err(|e| {
            AppError::LlmService(format!("Failed to parse completion response: {}", e))
        })?;

        if completion_response.choices.is_empty() {
            return Err(AppError::LlmService(
                "No completion choices returned".to_string(),
            ));
        }

        let content = completion_response.choices[0].message.content.clone();
        log::info!(
            "Groq completion successful, generated {} characters",
            content.len()
        );

        Ok(content)
    }

    /// Get estimated context window for a model
    fn get_context_window(model_id: &str, api_context_window: Option<u32>) -> usize {
        // Use API-provided context window if available
        if let Some(window) = api_context_window {
            return window as usize;
        }

        // Fall back to known context windows for common models
        if model_id.contains("llama-3.1-70b") || model_id.contains("llama-3.1-8b") {
            131072 // 128k tokens
        } else if model_id.contains("llama-3-70b") || model_id.contains("llama-3-8b") {
            8192 // 8k tokens
        } else if model_id.contains("mixtral-8x7b") {
            32768 // 32k tokens
        } else if model_id.contains("gemma-7b") {
            8192 // 8k tokens
        } else {
            8192 // Default
        }
    }
}

#[async_trait]
impl LlmServicePort for GroqService {
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
                let is_fallback = m.context_window.is_none();
                ModelInfo {
                    id: m.id.clone(),
                    name: m.id.clone(), // Groq doesn't provide separate display names
                    provider: "groq".to_string(),
                    context_window: Self::get_context_window(&m.id, m.context_window),
                    is_fallback_context_window: if is_fallback { Some(true) } else { None },
                }
            })
            .collect())
    }

    fn provider_name(&self) -> &str {
        "groq"
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groq_service_creation() {
        let service = GroqService::new("test_api_key".to_string());
        assert_eq!(service.provider_name(), "groq");
        assert!(service.is_configured());
    }

    #[test]
    fn test_groq_service_not_configured() {
        let service = GroqService::new("".to_string());
        assert!(!service.is_configured());
    }

    #[test]
    fn test_context_window_estimation() {
        assert_eq!(
            GroqService::get_context_window("llama-3.1-70b-versatile", None),
            131072
        );
        assert_eq!(
            GroqService::get_context_window("llama-3-70b-8192", None),
            8192
        );
        assert_eq!(
            GroqService::get_context_window("mixtral-8x7b-32768", None),
            32768
        );

        // Test with explicit context window from API
        assert_eq!(
            GroqService::get_context_window("llama-3.1-70b-versatile", Some(131072)),
            131072
        );
    }
}
