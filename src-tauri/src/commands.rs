//! Tauri IPC commands exposed to the React frontend.

use crate::agents::{self, PipelineResult};
use crate::audit::AuditRecord;
use crate::guardrails::{self, LintResult};
use crate::network_connector::{ExecResult, NetworkConnector, TranslatedCommand};
use crate::rag::DocumentChunk;
use crate::roi::RoiSnapshot;
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
    let result = agents::run_pipeline(
        &state.ollama,
        &kb,
        &request.intent,
        &health,
        request.model.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    let agent_logs = result
        .opinions
        .iter()
        .map(|o| {
            format!(
                "{} | conf={:.2} | {} => {}",
                o.agent, o.confidence, o.analysis, o.recommendation
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let blocked = !result.lint.allowed;
    state.roi.record_intent(result.duration_ms, blocked);

    let _ = state
        .audit
        .log_interaction(
            &result.intent,
            &agent_logs,
            &format!(
                "{} | {} | id={}",
                result.judge_summary, result.proposed_command, result.id
            ),
            &result.status,
            &format!("{:?}", result.risk),
            result.duration_ms as i64,
        )
        .await;

    Ok(result)
}

#[tauri::command]
pub async fn approve_action(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<String, String> {
    state
        .audit
        .set_human_decision(&action_id, "operator", "approved")
        .await?;
    Ok(format!("Action {action_id} APPROVED by human operator."))
}

#[tauri::command]
pub async fn reject_action(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<String, String> {
    state
        .audit
        .set_human_decision(&action_id, "operator", "rejected")
        .await?;
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
pub fn lint_command(state: tauri::State<'_, AppState>, command: String) -> LintResult {
    let result = guardrails::lint_command(&command);
    if !result.allowed {
        state.roi.record_block();
    }
    result
}

#[tauri::command]
pub fn get_roi_snapshot(state: tauri::State<'_, AppState>) -> RoiSnapshot {
    state.roi.snapshot()
}

#[tauri::command]
pub async fn get_audit_trail(
    state: tauri::State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<AuditRecord>, String> {
    state.audit.recent(limit.unwrap_or(50)).await
}

#[tauri::command]
pub async fn translate_intent(
    state: tauri::State<'_, AppState>,
    intent: String,
) -> Result<Vec<TranslatedCommand>, String> {
    let kb = state.rag.lock().await;
    Ok(crate::network_connector::CommandTranslator::translate_all(
        &intent, &kb,
    ))
}

#[tauri::command]
pub async fn simulate_vendor_exec(
    state: tauri::State<'_, AppState>,
    intent: String,
) -> Result<Vec<ExecResult>, String> {
    let kb = state.rag.lock().await;
    let pairs = NetworkConnector::translate_and_preview(&intent, &kb);
    Ok(pairs.into_iter().map(|(_, e)| e).collect())
}

#[tauri::command]
pub fn list_lab_devices() -> Vec<crate::network_connector::DeviceTarget> {
    NetworkConnector::default_lab_devices()
}

#[tauri::command]
pub fn ssh_exec_lab(
    target_id: String,
    command: String,
) -> Result<ExecResult, String> {
    let devices = NetworkConnector::default_lab_devices();
    let target = devices
        .iter()
        .find(|d| d.id == target_id)
        .ok_or_else(|| format!("Unknown target {target_id}"))?;
    NetworkConnector::exec(target, &command).map_err(|e| e.to_string())
}
