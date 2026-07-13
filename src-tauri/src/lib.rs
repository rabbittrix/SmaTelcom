//! SmaTelcom — Local-first AI Orchestrator for Telecommunications Networks (AN Level 4)
//! Author: Roberto de Souza <rabbittrix@hotmail.com>

mod agents;
mod audit;
mod commands;
mod digital_twin;
mod error;
mod guardrails;
mod network_connector;
mod ollama;
mod rag;
mod roi;
mod telemetry;

use audit::AuditTrail;
use roi::RoiTracker;
use std::sync::Arc;
use telemetry::TelemetryEngine;
use tokio::sync::Mutex;

pub struct AppState {
    pub telemetry: Arc<TelemetryEngine>,
    pub ollama: ollama::OllamaClient,
    pub rag: Arc<Mutex<rag::KnowledgeBase>>,
    pub audit: AuditTrail,
    pub roi: Arc<RoiTracker>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let telemetry = Arc::new(TelemetryEngine::new());
    let telemetry_bg = Arc::clone(&telemetry);
    telemetry_bg.start(std::time::Duration::from_secs(5));

    let knowledge = rag::KnowledgeBase::load_from_dir("../knowledge_base")
        .or_else(|_| rag::KnowledgeBase::load_from_dir("knowledge_base"))
        .unwrap_or_else(|e| {
            tracing::warn!("Knowledge base not loaded: {e}");
            rag::KnowledgeBase::empty()
        });

    let audit = tauri::async_runtime::block_on(async {
        match AuditTrail::open("../data/audit.db").await {
            Ok(a) => a,
            Err(_) => match AuditTrail::open("data/audit.db").await {
                Ok(a) => a,
                Err(e1) => match AuditTrail::open(
                    std::env::temp_dir().join("smatelcom_audit.db"),
                )
                .await
                {
                    Ok(a) => a,
                    Err(e2) => panic!("Failed to open audit SQLite DB: {e1} / {e2}"),
                },
            },
        }
    });

    let state = AppState {
        telemetry,
        ollama: ollama::OllamaClient::new("http://127.0.0.1:11434"),
        rag: Arc::new(Mutex::new(knowledge)),
        audit,
        roi: Arc::new(RoiTracker::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
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
            commands::translate_intent,
            commands::simulate_vendor_exec,
            commands::list_lab_devices,
            commands::ssh_exec_lab,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SmaTelcom");
}
