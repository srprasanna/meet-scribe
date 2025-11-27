//! LLM service adapters
//!
//! Implementations of the LlmServicePort trait for various providers:
//! - OpenAI (GPT-4, GPT-3.5-turbo)
//! - Anthropic (Claude)
//! - Google (Gemini)
//! - Groq (Llama, Mixtral, Gemma)

pub mod anthropic;
pub mod google;
pub mod groq;
pub mod openai;

pub use anthropic::AnthropicService;
pub use google::GoogleService;
pub use groq::GroqService;
pub use openai::OpenAIService;
