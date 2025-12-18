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
        r#"You are an expert meeting summarizer. Analyze the following meeting transcript and create a well-structured summary.

Meeting Transcript:
{transcript}

{context}

Create a structured summary with the following format:

## Meeting Summary

Organize the content by **key topics discussed**. For each topic:
- Use a ### heading for the topic name
- Under each topic, provide 2-4 bullet points with key details
- Use **bold** for important terms, names, decisions, metrics, or key concepts
- Include relevant timestamps in format `[HH:MM:SS]` or `[MM:SS]` where applicable
- Highlight any decisions made, action items identified, or important conclusions

Example format:
### Budget Planning
- Discussed **Q1 budget allocation** totaling **$2.5M** `[00:05:30]`
- **Sarah** proposed increasing marketing budget by **15%**
- Team agreed to review vendor contracts by **next week**

### Product Roadmap
- Reviewed upcoming **feature releases** for v2.0 `[00:15:45]`
- **John** presented timeline showing launch in **March 2024**
- Key priorities: **performance optimization** and **user experience**"#
    }

    /// Get default prompt for action items extraction
    pub fn action_items() -> &'static str {
        r#"You are an expert at extracting action items from meetings. Analyze the following meeting transcript and identify all actionable tasks.

Meeting Transcript:
{transcript}

{context}

Extract all action items, decisions requiring follow-up, and tasks mentioned.

## Action Items

For each action item, format as follows:
- **[Owner Name]** - [Clear description of task] - **Due: [Date/Timeframe]** `[Timestamp if mentioned]`
- If no owner is mentioned, use **[Unassigned]**
- If no deadline is mentioned, use **Due: TBD** or **Due: ASAP** if urgent
- Include relevant timestamp where the action item was discussed

Group action items by category if applicable (e.g., Technical, Marketing, Operations)

Example format:

### Technical Tasks
- **John** - Complete **API integration** with payment gateway - **Due: Friday, Dec 15** `[00:12:30]`
- **Sarah** - Review and merge **authentication PR** - **Due: End of week** `[00:18:45]`

### Marketing Tasks
- **Michael** - Prepare **Q1 campaign proposal** with budget breakdown - **Due: Next Monday** `[00:25:10]`
- **Unassigned** - Schedule meeting with design team - **Due: This week**

### Follow-ups
- **Team** - Review vendor contracts and provide feedback - **Due: Before next meeting**"#
    }

    /// Get default prompt for key points extraction
    pub fn key_points() -> &'static str {
        r#"You are an expert at identifying key discussion points from meetings. Analyze the following meeting transcript and extract the most important points.

Meeting Transcript:
{transcript}

{context}

Extract 5-10 key points, insights, or important statements from the meeting.

## Key Discussion Points

Organize key points by theme or topic. For each point:
- Use ### headings for major themes
- Under each theme, list 2-4 specific points as bullet items
- Use **bold** for critical terms, metrics, names, important concepts, or key stakeholders
- Include relevant timestamps in format `[HH:MM:SS]` or `[MM:SS]`
- Highlight any significant agreements, disagreements, concerns, or insights

Focus on:
- Critical information shared
- Important questions raised
- Significant agreements or disagreements
- Notable insights or revelations
- Key data points or metrics mentioned

Example format:

### Performance & Metrics
- Current system handles **5,000 requests/second** but needs to scale to **20,000** `[00:08:15]`
- **Database optimization** reduced query time by **40%**, noted by **Alex**
- Need to improve **API response time** from 200ms to under 100ms

### Customer Feedback
- Received **250+ feature requests** for mobile app in past month `[00:15:30]`
- **Top request**: offline mode for traveling users
- **Sarah** highlighted that **enterprise customers** are asking for SSO integration

### Technical Challenges
- Current architecture won't support planned **traffic increase** `[00:22:10]`
- Team debating between **microservices** vs **monolith** approach
- **John** raised concerns about **deployment complexity**"#
    }

    /// Get default prompt for decisions extraction
    pub fn decisions() -> &'static str {
        r#"You are an expert at identifying decisions made in meetings. Analyze the following meeting transcript and extract all decisions.

Meeting Transcript:
{transcript}

{context}

Identify all explicit or implicit decisions made during the meeting.

## Decisions Made

For each decision, provide:
- **Decision**: Clear statement of what was decided (use **bold** for the decision)
- **Rationale**: Why this decision was made (if mentioned)
- **Decision Maker**: Who made or approved it (use **bold** for names)
- **Timestamp**: When discussed, in format `[HH:MM:SS]` or `[MM:SS]` if available

Group related decisions under themed headings if applicable.

Example format:

### Infrastructure & Technical
**Decision**: Migrate to **AWS cloud infrastructure** by **Q2 2024**
- **Rationale**: Projected **30% cost savings** and improved scalability
- **Decision Maker**: Approved by **CTO Sarah** and **Engineering team**
- **Impact**: Will require 2-month migration period
- `[00:10:30]`

**Decision**: Adopt **microservices architecture** for new features
- **Rationale**: Better scalability and team independence
- **Decision Maker**: **Engineering leadership**
- **Impact**: Requires additional DevOps resources
- `[00:18:45]`

### Product & Features
**Decision**: Postpone **mobile app redesign** to next quarter
- **Rationale**: Focus resources on **API stability** first
- **Decision Maker**: **Product team** with **CEO** approval
- **Impact**: Delay in customer-facing improvements
- `[00:25:15]`

### Budget & Resources
**Decision**: Increase **marketing budget** by **$50,000** for Q1
- **Rationale**: Support new product launch campaign
- **Decision Maker**: **CFO** and **Marketing Director**
- `[00:32:00]`

Focus on concrete, actionable decisions rather than ongoing discussions."#
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
