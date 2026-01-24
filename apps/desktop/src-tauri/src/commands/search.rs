/// Search commands using FTS5 full-text search
use crate::domain::models::{InsightSearchResult, Meeting, SearchResults, TranscriptSearchResult};
use crate::ports::storage::StoragePort;
use crate::AppState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: Option<i32>,
}

fn default_limit() -> Option<i32> {
    Some(50)
}

/// Search across all entities (transcripts, insights, meetings)
#[tauri::command]
pub async fn search_all(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<i32>,
) -> Result<SearchResults, String> {
    log::info!("Searching all entities for: '{}'", query);

    if query.trim().is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    state
        .storage
        .search_all(&query, limit)
        .await
        .map_err(|e| format!("Search failed: {}", e))
}

/// Search only transcripts
#[tauri::command]
pub async fn search_transcripts(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<i32>,
) -> Result<Vec<TranscriptSearchResult>, String> {
    log::info!("Searching transcripts for: '{}'", query);

    if query.trim().is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    state
        .storage
        .search_transcripts(&query, limit)
        .await
        .map_err(|e| format!("Transcript search failed: {}", e))
}

/// Search only insights
#[tauri::command]
pub async fn search_insights(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<i32>,
) -> Result<Vec<InsightSearchResult>, String> {
    log::info!("Searching insights for: '{}'", query);

    if query.trim().is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    state
        .storage
        .search_insights(&query, limit)
        .await
        .map_err(|e| format!("Insight search failed: {}", e))
}

/// Search meetings by title and platform (includes all meetings, even those without titles)
#[tauri::command]
pub async fn search_meetings(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<i32>,
) -> Result<Vec<Meeting>, String> {
    log::info!("Searching meetings for: '{}'", query);

    if query.trim().is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    state
        .storage
        .search_meetings(&query, limit)
        .await
        .map_err(|e| format!("Meeting search failed: {}", e))
}
