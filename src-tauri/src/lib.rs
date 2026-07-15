//! SmarTelcom — Local-first AI Orchestrator for Telecommunications Networks (AN Level 4)
//! Author: Roberto de Souza <rabbittrix@hotmail.com>

mod agents;
mod adapters;
mod audit;
mod commands;
mod db;
mod digital_twin;
mod drivers;
mod error;
mod executor;
mod guardrails;
mod network_connector;
mod northbound;
mod ollama;
mod rag;
mod roi;
mod telemetry;

use db::AuditDb;
use guardrails::RiskLevel;
use parking_lot::Mutex as SyncMutex;
use roi::RoiTracker;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use telemetry::TelemetryEngine;
use tokio::sync::Mutex;
use tauri::Emitter;

#[derive(Debug, Clone)]
pub struct PendingHitlAction {
    pub id: String,
    pub proposed_command: String,
    pub intent: String,
    pub decision_logic: String,
    pub risk: RiskLevel,
}

pub struct AppState {
    pub telemetry: Arc<TelemetryEngine>,
    pub ollama: ollama::OllamaClient,
    pub rag: Arc<Mutex<rag::KnowledgeBase>>,
    pub audit: AuditDb,
    pub roi: Arc<RoiTracker>,
    pub pending_hitl: Arc<SyncMutex<HashMap<String, PendingHitlAction>>>,
    pub approved_commands: Arc<SyncMutex<HashSet<String>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let telemetry = Arc::new(TelemetryEngine::new());

    let knowledge = rag::KnowledgeBase::load_from_dir("../knowledge_base")
        .or_else(|_| rag::KnowledgeBase::load_from_dir("knowledge_base"))
        .unwrap_or_else(|e| {
            tracing::warn!("Knowledge base not loaded: {e}");
            rag::KnowledgeBase::empty()
        });

    let audit = tauri::async_runtime::block_on(async {
        db::open_default()
            .await
            .unwrap_or_else(|e| panic!("Failed to open audit.db: {e}"))
    });

    let state = AppState {
        telemetry: Arc::clone(&telemetry),
        ollama: ollama::OllamaClient::new("http://127.0.0.1:11434"),
        rag: Arc::new(Mutex::new(knowledge)),
        audit,
        roi: Arc::new(RoiTracker::new()),
        pending_hitl: Arc::new(SyncMutex::new(HashMap::new())),
        approved_commands: Arc::new(SyncMutex::new(HashSet::new())),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(move |app| {
            let handle = app.handle().clone();
            let tel = Arc::clone(&telemetry);
            tel.start(std::time::Duration::from_secs(5), move |snap| {
                let _ = handle.emit("telemetry-tick", &snap);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::check_ollama,
            commands::list_models,
            commands::get_telemetry_snapshot,
            commands::analyze_network_intent,
            commands::approve_action,
            commands::reject_action,
            commands::reload_knowledge_base,
            commands::search_knowledge,
            commands::lint_command,
            commands::get_roi_snapshot,
            commands::get_audit_trail,
            commands::get_audit_history,
            commands::translate_intent,
            commands::simulate_vendor_exec,
            commands::list_lab_devices,
            commands::ssh_exec_lab,
            commands::northbound_dry_run,
            commands::northbound_commit,
            commands::get_impact_report,
            commands::list_inventory,
            commands::translate_vendor_payloads,
            commands::execute_approved_action,
            commands::search_audit_logs,
            commands::receive_external_intent,
            commands::get_demo_service_order,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SmarTelcom");
}
