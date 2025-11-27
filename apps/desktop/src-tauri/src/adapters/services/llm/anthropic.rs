//! Anthropic LLM service adapter
//!
//! Implements the LlmServicePort for Anthropic's API (Claude models)
//! Supports dynamic model fetching and customizable prompts.

use crate::domain::models::InsightType;
use crate::error::{AppError, Result};
use crate::ports::llm::{GeneratedInsight, InsightRequest, LlmConfig, LlmServicePort, ModelInfo};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Anthropic service implementation
pub struct AnthropicService {
    client: Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicModel {
    id: String,
    #[serde(rename = "type")]
    model_type: String,
    display_name: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
    has_more: bool,
    first_id: Option<String>,
    last_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

impl AnthropicService {
    /// Create a new Anthropic service with the given API key
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }

    /// Fetch available models from Anthropic API
    pub async fn list_models(&self) -> Result<Vec<AnthropicModel>> {
        log::info!("Fetching available models from Anthropic");

        let response = self
            .client
            .get(format!("{}/models", ANTHROPIC_API_BASE))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
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

        let models_response: AnthropicModelsResponse = response
            .json()
            .await
            .map_err(|e| AppError::LlmService(format!("Failed to parse models response: {}", e)))?;

        log::info!("Found {} Anthropic models", models_response.data.len());
        Ok(models_response.data)
    }

    /// Generate text using messages API
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

        let messages = vec![Message {
            role: "user".to_string(),
            content: formatted_prompt,
        }];

        // Anthropic requires max_tokens to be specified
        let max_tokens = config.max_tokens.unwrap_or(4096);

        let request_body = MessagesRequest {
            model: config.model.clone(),
            messages,
            max_tokens,
            temperature: config.temperature,
        };

        log::info!(
            "Calling Anthropic messages API with model: {}",
            config.model
        );

        let response = self
            .client
            .post(format!("{}/messages", ANTHROPIC_API_BASE))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::LlmService(format!("Messages request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::LlmService(format!(
                "Messages request failed: {}",
                error_text
            )));
        }

        let messages_response: MessagesResponse = response.json().await.map_err(|e| {
            AppError::LlmService(format!("Failed to parse messages response: {}", e))
        })?;

        if messages_response.content.is_empty() {
            return Err(AppError::LlmService(
                "No content blocks returned".to_string(),
            ));
        }

        let content = messages_response.content[0].text.clone();
        log::info!(
            "Anthropic completion successful, generated {} characters",
            content.len()
        );

        Ok(content)
    }

    /// Get estimated context window for a model
    /// Returns (context_window, is_fallback)
    fn get_context_window(model_id: &str) -> (usize, bool) {
        let (window, is_fallback) = if model_id.contains("claude-3-5-sonnet") {
            (200000, false) // Claude 3.5 Sonnet
        } else if model_id.contains("claude-3-opus") {
            (200000, false) // Claude 3 Opus
        } else if model_id.contains("claude-3-sonnet") {
            (200000, false) // Claude 3 Sonnet
        } else if model_id.contains("claude-3-haiku") {
            (200000, false) // Claude 3 Haiku
        } else if model_id.contains("claude-2.1") {
            (200000, false) // Claude 2.1
        } else if model_id.contains("claude-2") {
            (100000, false) // Claude 2
        } else {
            // Unknown model - use conservative fallback
            log::warn!(
                "Unknown Anthropic model '{}' - using fallback context window of 100000 tokens. \
                Consider configuring a custom context window for this model in settings.",
                model_id
            );
            (100000, true)
        };

        if is_fallback {
            log::info!(
                "Model '{}' may support a larger context window. \
                Check Anthropic documentation and configure override if needed.",
                model_id
            );
        }

        (window, is_fallback)
    }
}

#[async_trait]
impl LlmServicePort for AnthropicService {
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
                let (context_window, is_fallback) = Self::get_context_window(&m.id);
                ModelInfo {
                    id: m.id.clone(),
                    name: m.display_name,
                    provider: "anthropic".to_string(),
                    context_window,
                    is_fallback_context_window: if is_fallback { Some(true) } else { None },
                }
            })
            .collect())
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_service_creation() {
        let service = AnthropicService::new("test_api_key".to_string());
        assert_eq!(service.provider_name(), "anthropic");
        assert!(service.is_configured());
    }

    #[test]
    fn test_anthropic_service_not_configured() {
        let service = AnthropicService::new("".to_string());
        assert!(!service.is_configured());
    }

    #[test]
    fn test_context_window_estimation() {
        assert_eq!(
            AnthropicService::get_context_window("claude-3-5-sonnet-20241022"),
            (200000, false)
        );
        assert_eq!(
            AnthropicService::get_context_window("claude-3-opus-20240229"),
            (200000, false)
        );
        assert_eq!(
            AnthropicService::get_context_window("claude-3-haiku-20240307"),
            (200000, false)
        );
        assert_eq!(
            AnthropicService::get_context_window("claude-2.1"),
            (200000, false)
        );
        // Test fallback for unknown model
        assert_eq!(
            AnthropicService::get_context_window("claude-4-opus"),
            (100000, true)
        );
    }
}
