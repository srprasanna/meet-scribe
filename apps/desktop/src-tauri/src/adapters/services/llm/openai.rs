//! OpenAI LLM service adapter
//!
//! Implements the LlmServicePort for OpenAI's API (GPT-4, GPT-3.5-turbo, etc.)
//! Supports dynamic model fetching and customizable prompts.

use crate::domain::models::InsightType;
use crate::error::{AppError, Result};
use crate::ports::llm::{GeneratedInsight, InsightRequest, LlmConfig, LlmServicePort, ModelInfo};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const OPENAI_API_BASE: &str = "https://api.openai.com/v1";

/// OpenAI service implementation
pub struct OpenAIService {
    client: Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    object: String,
    created: u64,
    owned_by: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
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

impl OpenAIService {
    /// Create a new OpenAI service with the given API key
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }

    /// Fetch available models from OpenAI API
    pub async fn list_models(&self) -> Result<Vec<OpenAIModel>> {
        log::info!("Fetching available models from OpenAI");

        let response = self
            .client
            .get(format!("{}/models", OPENAI_API_BASE))
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

        let models_response: OpenAIModelsResponse = response
            .json()
            .await
            .map_err(|e| AppError::LlmService(format!("Failed to parse models response: {}", e)))?;

        log::info!("Found {} OpenAI models", models_response.data.len());
        Ok(models_response.data)
    }

    /// Generate text using chat completion API
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

        log::info!(
            "Calling OpenAI chat completion with model: {}",
            config.model
        );

        let response = self
            .client
            .post(format!("{}/chat/completions", OPENAI_API_BASE))
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
            "OpenAI completion successful, generated {} characters",
            content.len()
        );

        Ok(content)
    }

    /// Get estimated context window for a model
    /// Returns (context_window, is_fallback)
    fn get_context_window(model_id: &str) -> (usize, bool) {
        let (window, is_fallback) =
            if model_id.contains("gpt-4-turbo") || model_id.contains("gpt-4-1106") {
                (128000, false)
            } else if model_id.contains("gpt-4-32k") {
                (32768, false)
            } else if model_id.contains("gpt-4") {
                (8192, false)
            } else if model_id.contains("gpt-3.5-turbo-16k") {
                (16384, false)
            } else if model_id.contains("gpt-3.5-turbo") {
                (4096, false)
            } else {
                // Unknown model - use conservative fallback
                log::warn!(
                    "Unknown OpenAI model '{}' - using fallback context window of 4096 tokens. \
                Consider configuring a custom context window for this model in settings.",
                    model_id
                );
                (4096, true)
            };

        if is_fallback {
            log::info!(
                "Model '{}' may support a larger context window. \
                Check OpenAI documentation and configure override if needed.",
                model_id
            );
        }

        (window, is_fallback)
    }
}

#[async_trait]
impl LlmServicePort for OpenAIService {
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
                    name: m.id.clone(),
                    provider: "openai".to_string(),
                    context_window,
                    is_fallback_context_window: if is_fallback { Some(true) } else { None },
                }
            })
            .collect())
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_service_creation() {
        let service = OpenAIService::new("test_api_key".to_string());
        assert_eq!(service.provider_name(), "openai");
        assert!(service.is_configured());
    }

    #[test]
    fn test_openai_service_not_configured() {
        let service = OpenAIService::new("".to_string());
        assert!(!service.is_configured());
    }

    #[test]
    fn test_context_window_estimation() {
        assert_eq!(
            OpenAIService::get_context_window("gpt-4-turbo"),
            (128000, false)
        );
        assert_eq!(OpenAIService::get_context_window("gpt-4"), (8192, false));
        assert_eq!(
            OpenAIService::get_context_window("gpt-3.5-turbo"),
            (4096, false)
        );
        // Test fallback for unknown model
        assert_eq!(
            OpenAIService::get_context_window("gpt-5-ultra"),
            (4096, true)
        );
    }
}
