/// LLM service port trait
///
/// Defines the interface for Large Language Model services.
/// Implementations: OpenAI, Anthropic, etc.
use crate::domain::models::InsightType;
use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Represents a generated insight from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedInsight {
    pub insight_type: InsightType,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

/// Request to generate insights from a transcript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightRequest {
    /// The full transcript text
    pub transcript: String,

    /// Optional meeting context (title, participants, etc.)
    pub context: Option<String>,

    /// Types of insights to generate
    pub insight_types: Vec<InsightType>,
}

/// Configuration for LLM requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Model name (e.g., "gpt-4", "claude-3-sonnet")
    pub model: String,

    /// Temperature for generation (0.0 to 1.0)
    pub temperature: Option<f32>,

    /// Maximum tokens in response
    pub max_tokens: Option<u32>,

    /// Provider-specific settings as JSON
    pub additional_settings: Option<serde_json::Value>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            temperature: Some(0.3), // Lower temperature for more focused outputs
            max_tokens: Some(2000),
            additional_settings: None,
        }
    }
}

/// Model information from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_fallback_context_window: Option<bool>, // True if context window is estimated
}

/// Port trait for LLM services
#[async_trait]
pub trait LlmServicePort: Send + Sync {
    /// Generate insights from a meeting transcript with optional custom prompt
    async fn generate_insights(
        &self,
        request: &InsightRequest,
        config: &LlmConfig,
        prompt_template: Option<&str>,
    ) -> Result<Vec<GeneratedInsight>>;

    /// Generate a summary from a transcript with optional custom prompt
    async fn generate_summary(
        &self,
        transcript: &str,
        context: Option<&str>,
        config: &LlmConfig,
        prompt_template: Option<&str>,
    ) -> Result<String>;

    /// Fetch available models from provider API
    async fn fetch_available_models(&self) -> Result<Vec<ModelInfo>>;

    /// Get the provider name
    fn provider_name(&self) -> &str;

    /// Check if the service is configured (has API key)
    fn is_configured(&self) -> bool;
}
