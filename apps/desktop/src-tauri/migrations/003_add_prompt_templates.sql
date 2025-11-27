-- Prompt templates table for customizable LLM prompts
CREATE TABLE IF NOT EXISTS prompt_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    insight_type TEXT NOT NULL CHECK(insight_type IN ('summary', 'action_item', 'key_point', 'decision')),
    name TEXT NOT NULL,
    prompt_text TEXT NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(insight_type, name)
);

CREATE INDEX idx_prompt_templates_type ON prompt_templates(insight_type);
CREATE INDEX idx_prompt_templates_active ON prompt_templates(is_active);

-- Insert default templates for each insight type
INSERT INTO prompt_templates (insight_type, name, prompt_text, is_default, is_active) VALUES
('summary', 'Default Summary', 'You are an expert meeting summarizer. Analyze the following meeting transcript and create a concise summary.

Meeting Transcript:
{transcript}

{context}

Create a clear, concise summary in 3-5 bullet points covering:
- Main topics discussed
- Key decisions or conclusions
- Important highlights

Format your response as a bulleted list with each point starting with "- ".', 1, 1),

('action_item', 'Default Action Items', 'You are an expert at extracting action items from meetings. Analyze the following meeting transcript and identify all actionable tasks.

Meeting Transcript:
{transcript}

{context}

Extract all action items, decisions requiring follow-up, and tasks mentioned. For each action item, provide:
- The specific task or action
- Who is responsible (if mentioned)
- Any deadlines or timeframes (if mentioned)

Format each action item on a separate line starting with "- ". Be specific and actionable.', 1, 1),

('key_point', 'Default Key Points', 'You are an expert at identifying key discussion points from meetings. Analyze the following meeting transcript and extract the most important points.

Meeting Transcript:
{transcript}

{context}

Identify 3-7 key points, insights, or important statements from the meeting. Focus on:
- Critical information shared
- Important questions raised
- Significant agreements or disagreements
- Notable insights or revelations

Format your response as a bulleted list with each point starting with "- ".', 1, 1),

('decision', 'Default Decisions', 'You are an expert at identifying decisions made in meetings. Analyze the following meeting transcript and extract all decisions.

Meeting Transcript:
{transcript}

{context}

Identify all explicit or implicit decisions made during the meeting. For each decision, provide:
- What was decided
- The rationale or context (if provided)
- Who made or approved the decision (if mentioned)

Format each decision on a separate line starting with "- ". Focus on concrete decisions, not just discussions.', 1, 1);
