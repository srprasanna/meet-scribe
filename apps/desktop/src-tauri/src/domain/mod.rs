/// Domain layer - core business models
///
/// These models are platform-agnostic and represent core business entities.
pub mod models;
pub mod prompts;

pub use models::{
    Insight, InsightType, Meeting, ModelOverride, Participant, Platform, ServiceConfig,
    ServiceType, Transcript,
};
pub use prompts::PromptTemplates;
