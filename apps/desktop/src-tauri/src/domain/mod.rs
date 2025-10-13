/// Domain layer - core business models
///
/// These models are platform-agnostic and represent core business entities.
pub mod models;

pub use models::{
    Insight, InsightType, Meeting, Participant, Platform, ServiceConfig, ServiceType, Transcript,
};
