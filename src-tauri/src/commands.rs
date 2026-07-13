//! Tauri IPC commands exposed to the React frontend.

use crate::agents::{self, PipelineResult};
use crate::guardrails::{self, LintResult};
use crate::rag::DocumentChunk;
use crate::telemetry::HealthSnapshot;
use crate::AppState;
use serde::Deserialize;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("SmaTelcom online — welcome, {name}.")
}

#[tauri::command]
pub async fn check_ollama(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    state.ollama.health().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_models(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    state.ollama.list_models().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_telemetry_snapshot(state: tauri::State<'_, AppState>) -> HealthSnapshot {
    state.telemetry.snapshot()
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    pub intent: String,
    pub model: Option<String>,
}

#[tauri::command]
pub async fn analyze_network_intent(
    state: tauri::State<'_, AppState>,
    request: AnalyzeRequest,
) -> Result<PipelineResult, String> {
    let health = state.telemetry.snapshot();
    let kb = state.rag.lock().await;
    agents::run_pipeline(
        &state.ollama,
        &kb,
        &request.intent,
        &health,
        request.model.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn approve_action(action_id: String) -> Result<String, String> {
    Ok(format!("Action {action_id} APPROVED by human operator."))
}

#[tauri::command]
pub fn reject_action(action_id: String) -> Result<String, String> {
    Ok(format!("Action {action_id} REJECTED by human operator."))
}

#[tauri::command]
pub async fn reload_knowledge_base(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    let kb = crate::rag::KnowledgeBase::load_from_dir("../knowledge_base")
        .or_else(|_| crate::rag::KnowledgeBase::load_from_dir("knowledge_base"))
        .map_err(|e| e.to_string())?;
    let count = kb.chunks.len();
    *state.rag.lock().await = kb;
    Ok(count)
}

#[tauri::command]
pub async fn search_knowledge(
    state: tauri::State<'_, AppState>,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<DocumentChunk>, String> {
    let kb = state.rag.lock().await;
    Ok(kb.search(&query, top_k.unwrap_or(5)))
}

#[tauri::command]
pub fn lint_command(command: String) -> LintResult {
    guardrails::lint_command(&command)
}
