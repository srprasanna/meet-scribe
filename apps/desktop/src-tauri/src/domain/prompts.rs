//! Prompt templates for LLM insight generation
//!
//! Provides default prompt templates for each insight type and utilities
//! for prompt management.

use crate::domain::models::InsightType;

/// Default prompt templates for each insight type
pub struct PromptTemplates;

impl PromptTemplates {
    /// Get default prompt for summary generation
    pub fn summary() -> &'static str {
        r#"You are an expert meeting summarizer. Analyze the following meeting transcript and create a concise summary.

Meeting Transcript:
{transcript}

{context}

Create a clear, concise summary in 3-5 bullet points covering:
- Main topics discussed
- Key decisions or conclusions
- Important highlights

Format your response as a bulleted list with each point starting with "- "."#
    }

    /// Get default prompt for action items extraction
    pub fn action_items() -> &'static str {
        r#"You are an expert at extracting action items from meetings. Analyze the following meeting transcript and identify all actionable tasks.

Meeting Transcript:
{transcript}

{context}

Extract all action items, decisions requiring follow-up, and tasks mentioned. For each action item, provide:
- The specific task or action
- Who is responsible (if mentioned)
- Any deadlines or timeframes (if mentioned)

Format each action item on a separate line starting with "- ". Be specific and actionable."#
    }

    /// Get default prompt for key points extraction
    pub fn key_points() -> &'static str {
        r#"You are an expert at identifying key discussion points from meetings. Analyze the following meeting transcript and extract the most important points.

Meeting Transcript:
{transcript}

{context}

Identify 3-7 key points, insights, or important statements from the meeting. Focus on:
- Critical information shared
- Important questions raised
- Significant agreements or disagreements
- Notable insights or revelations

Format your response as a bulleted list with each point starting with "- "."#
    }

    /// Get default prompt for decisions extraction
    pub fn decisions() -> &'static str {
        r#"You are an expert at identifying decisions made in meetings. Analyze the following meeting transcript and extract all decisions.

Meeting Transcript:
{transcript}

{context}

Identify all explicit or implicit decisions made during the meeting. For each decision, provide:
- What was decided
- The rationale or context (if provided)
- Who made or approved the decision (if mentioned)

Format each decision on a separate line starting with "- ". Focus on concrete decisions, not just discussions."#
    }

    /// Get all default templates
    pub fn all() -> Vec<(InsightType, &'static str)> {
        vec![
            (InsightType::Summary, Self::summary()),
            (InsightType::ActionItem, Self::action_items()),
            (InsightType::KeyPoint, Self::key_points()),
            (InsightType::Decision, Self::decisions()),
        ]
    }

    /// Get default prompt for a specific insight type
    pub fn for_type(insight_type: &InsightType) -> &'static str {
        match insight_type {
            InsightType::Summary => Self::summary(),
            InsightType::ActionItem => Self::action_items(),
            InsightType::KeyPoint => Self::key_points(),
            InsightType::Decision => Self::decisions(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_exist() {
        let templates = PromptTemplates::all();
        assert_eq!(templates.len(), 4);
    }

    #[test]
    fn test_summary_template() {
        let prompt = PromptTemplates::summary();
        assert!(prompt.contains("{transcript}"));
        assert!(prompt.contains("{context}"));
    }

    #[test]
    fn test_for_type() {
        let summary = PromptTemplates::for_type(&InsightType::Summary);
        assert_eq!(summary, PromptTemplates::summary());
    }
}
