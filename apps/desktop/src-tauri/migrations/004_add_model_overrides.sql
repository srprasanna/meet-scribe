-- Model configuration overrides table
-- Allows users to customize model-specific settings like context window
CREATE TABLE IF NOT EXISTS model_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,  -- 'openai', 'anthropic', 'google', 'groq'
    model_id TEXT NOT NULL,  -- Model identifier (e.g., 'gpt-4', 'claude-3-opus-20240229')
    context_window INTEGER,  -- User-configured context window (overrides default)
    notes TEXT,              -- User notes about this model
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(provider, model_id)
);

CREATE INDEX idx_model_overrides_provider ON model_overrides(provider);
CREATE INDEX idx_model_overrides_model ON model_overrides(model_id);
