//! Tauri IPC commands exposed to the React frontend.

use crate::agents::{self, PipelineProgress, PipelineResult};
use crate::db::{AuditLogEntry, ImpactReport, NewAuditLog};
use crate::drivers::{self, DriverRequest, DriverResult, NorthboundProtocol};
use crate::guardrails::{self, LintResult, RiskLevel};
use crate::network_connector::{ExecResult, NetworkConnector, TranslatedCommand};
use crate::rag::DocumentChunk;
use crate::roi::RoiSnapshot;
use crate::telemetry::HealthSnapshot;
use crate::{AppState, PendingHitlAction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize)]
pub struct HitlOutcome {
    pub action_id: String,
    pub decision: String,
    pub message: String,
    pub command: Option<String>,
    pub lint: Option<LintResult>,
    pub driver: Option<DriverResult>,
}

fn decision_from_status(status: &str) -> &'static str {
    match status {
        "auto_approved" => "Auto-Approved",
        "pending_hitl" => "HITL-Pending",
        "blocked_by_safety" => "Blocked",
        _ => "Unknown",
    }
}

fn risk_label(risk: &RiskLevel) -> String {
    match risk {
        RiskLevel::Low => "Low".into(),
        RiskLevel::Medium => "Medium".into(),
        RiskLevel::High => "High".into(),
        RiskLevel::Critical => "Critical".into(),
    }
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("SmarTelcom online — welcome, {name}.")
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
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    request: AnalyzeRequest,
) -> Result<PipelineResult, String> {
    let health = state.telemetry.snapshot();
    let kb = state.rag.lock().await.clone();
    let result = agents::run_pipeline(
        &state.ollama,
        &kb,
        &request.intent,
        &health,
        request.model.as_deref(),
        |progress: PipelineProgress| {
            let _ = app.emit("pipeline-progress", &progress);
        },
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
    if blocked
        && matches!(
            result.lint.risk,
            RiskLevel::High | RiskLevel::Critical
        )
    {
        state.roi.record_critical_block();
    }

    if result.status == "pending_hitl" {
        state.pending_hitl.lock().insert(
            result.id.clone(),
            PendingHitlAction {
                id: result.id.clone(),
                proposed_command: result.proposed_command.clone(),
                intent: result.intent.clone(),
                decision_logic: result.decision_logic.clone(),
                risk: result.risk.clone(),
            },
        );
    } else if result.status == "auto_approved" {
        let savings = state.telemetry.record_auto_approve();
        state.roi.record_auto_approve();
        state
            .approved_commands
            .lock()
            .insert(result.proposed_command.clone());
        let _ = app.emit("autonomy-savings", &savings);
        let _ = app.emit("telemetry-tick", &state.telemetry.snapshot());
    }

    let conflict_notes = if result.conflict_resolution.conflict_detected {
        format!(
            "{} beat {} | {} | {}",
            result.conflict_resolution.winner,
            result.conflict_resolution.loser,
            result.conflict_resolution.rationale,
            result.conflict_resolution.policy_citation
        )
    } else {
        result.conflict_resolution.rationale.clone()
    };

    // Technical payload (How) generated for every decision — stored in audit_logs.
    let (_proto, payload_preview) =
        drivers::generate_technical_payload(&result.proposed_command, "lab-ne-01");

    let _ = state
        .audit
        .insert_log(&NewAuditLog {
            id: result.id.clone(),
            intent: result.intent.clone(),
            final_command: result.proposed_command.clone(),
            risk_level: risk_label(&result.risk),
            decision: decision_from_status(&result.status).into(),
            conflict_resolution: conflict_notes,
            payload_preview,
            agent_logs,
            policy_citation: result.conflict_resolution.policy_citation.clone(),
            ai_duration_ms: result.duration_ms as i64,
        })
        .await;

    Ok(result)
}

#[tauri::command]
pub async fn approve_action(
    state: tauri::State<'_, AppState>,
    action_id: String,
    protocol: Option<String>,
    dry_run: Option<bool>,
) -> Result<HitlOutcome, String> {
    let pending = {
        let mut map = state.pending_hitl.lock();
        map.remove(&action_id)
            .ok_or_else(|| format!("Unknown or already resolved HITL action {action_id}"))?
    };

    let lint = guardrails::lint_with_context(&pending.proposed_command, &pending.intent);
    if !lint.allowed {
        if matches!(lint.risk, RiskLevel::High | RiskLevel::Critical) {
            state.roi.record_critical_block();
        } else {
            state.roi.record_block();
        }
        let _ = state
            .audit
            .update_decision(&action_id, "Blocked", None)
            .await;
        return Err(format!(
            "Approval denied by Safety Linter: {}",
            lint.reason
        ));
    }

    let proto = match protocol.as_deref() {
        Some("gnmi") => NorthboundProtocol::Gnmi,
        Some("netconf") => NorthboundProtocol::Netconf,
        _ => drivers::generate_technical_payload(&pending.proposed_command, "lab-ne-01").0,
    };
    let driver = drivers::commit(&DriverRequest {
        command: pending.proposed_command.clone(),
        protocol: proto,
        target: "lab-ne-01".into(),
        dry_run: dry_run.unwrap_or(false),
        action_id: Some(action_id.clone()),
    });

    state
        .audit
        .update_decision(
            &action_id,
            "HITL-Approved",
            Some(&driver.payload.body),
        )
        .await?;

    state
        .approved_commands
        .lock()
        .insert(pending.proposed_command.clone());

    Ok(HitlOutcome {
        action_id: action_id.clone(),
        decision: "approved".into(),
        message: format!("HITL-Approved · {}", driver.message),
        command: Some(pending.proposed_command.clone()),
        lint: Some(lint),
        driver: Some(driver),
    })
}

#[tauri::command]
pub async fn reject_action(
    state: tauri::State<'_, AppState>,
    action_id: String,
) -> Result<HitlOutcome, String> {
    let pending = {
        let mut map = state.pending_hitl.lock();
        map.remove(&action_id)
            .ok_or_else(|| format!("Unknown or already resolved HITL action {action_id}"))?
    };

    state
        .audit
        .update_decision(&action_id, "Rejected", None)
        .await?;

    Ok(HitlOutcome {
        action_id,
        decision: "rejected".into(),
        message: format!(
            "Rejected — command discarded: {}",
            pending.proposed_command
        ),
        command: Some(pending.proposed_command),
        lint: None,
        driver: None,
    })
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
    let kb = state.rag.lock().await.clone();
    Ok(kb.search(&query, top_k.unwrap_or(5)))
}

#[tauri::command]
pub fn lint_command(state: tauri::State<'_, AppState>, command: String) -> LintResult {
    let result = guardrails::lint_command(&command);
    if !result.allowed {
        if matches!(result.risk, RiskLevel::High | RiskLevel::Critical) {
            state.roi.record_critical_block();
        } else {
            state.roi.record_block();
        }
    }
    result
}

#[tauri::command]
pub fn get_roi_snapshot(state: tauri::State<'_, AppState>) -> RoiSnapshot {
    state.roi.snapshot()
}

#[tauri::command]
pub async fn get_impact_report(
    state: tauri::State<'_, AppState>,
) -> Result<ImpactReport, String> {
    let mut report = state.audit.impact_report().await?;
    let roi = state.roi.snapshot();
    report.intents_processed = report.intents_processed.max(roi.intents_processed);
    report.auto_approved = report.auto_approved.max(roi.auto_approved);
    report.critical_risks_averted = report
        .critical_risks_averted
        .max(roi.critical_risks_averted);
    report.human_hours_saved =
        (report.auto_approved as f64 * report.minutes_per_auto_approve) / 60.0;
    Ok(report)
}

#[tauri::command]
pub async fn get_audit_trail(
    state: tauri::State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<AuditLogEntry>, String> {
    state.audit.get_audit_history(limit.unwrap_or(50)).await
}

#[tauri::command]
pub async fn get_audit_history(
    state: tauri::State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<AuditLogEntry>, String> {
    state.audit.get_audit_history(limit.unwrap_or(100)).await
}

#[derive(Debug, Deserialize)]
pub struct NorthboundRequest {
    pub command: String,
    pub protocol: Option<String>,
    pub target: Option<String>,
    pub dry_run: Option<bool>,
    pub action_id: Option<String>,
}

fn parse_protocol(s: Option<&str>) -> NorthboundProtocol {
    match s {
        Some("gnmi") => NorthboundProtocol::Gnmi,
        _ => NorthboundProtocol::Netconf,
    }
}

#[tauri::command]
pub fn northbound_dry_run(request: NorthboundRequest) -> Result<DriverResult, String> {
    let command = request.command.trim().to_string();
    if command.is_empty() {
        return Err("Command cannot be empty".into());
    }
    let lint = guardrails::lint_command(&command);
    if !lint.allowed {
        return Err(format!("Dry-run blocked by Safety Linter: {}", lint.reason));
    }
    Ok(drivers::commit(&DriverRequest {
        command,
        protocol: parse_protocol(request.protocol.as_deref()),
        target: request.target.unwrap_or_else(|| "lab-ne-01".into()),
        dry_run: true,
        action_id: request.action_id,
    }))
}

#[tauri::command]
pub async fn northbound_commit(
    state: tauri::State<'_, AppState>,
    request: NorthboundRequest,
) -> Result<DriverResult, String> {
    let command = request.command.trim().to_string();
    if command.is_empty() {
        return Err("Command cannot be empty".into());
    }
    let lint = guardrails::lint_command(&command);
    if !lint.allowed {
        if matches!(lint.risk, RiskLevel::High | RiskLevel::Critical) {
            state.roi.record_critical_block();
        }
        return Err(format!("Commit blocked by Safety Linter: {}", lint.reason));
    }
    if lint.requires_hitl {
        let cleared = state.approved_commands.lock().contains(&command);
        if !cleared {
            return Err(format!(
                "Commit requires HITL Approve first (risk={:?})",
                lint.risk
            ));
        }
    }

    let result = drivers::commit(&DriverRequest {
        command,
        protocol: parse_protocol(request.protocol.as_deref()),
        target: request.target.unwrap_or_else(|| "lab-ne-01".into()),
        dry_run: request.dry_run.unwrap_or(false),
        action_id: request.action_id.clone(),
    });

    if let Some(aid) = &request.action_id {
        if !result.payload.dry_run {
            let _ = state
                .audit
                .update_decision(aid, "HITL-Approved", Some(&result.payload.body))
                .await;
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn translate_intent(
    state: tauri::State<'_, AppState>,
    intent: String,
) -> Result<Vec<TranslatedCommand>, String> {
    let intent = intent.trim().to_string();
    if intent.is_empty() {
        return Err("Intent cannot be empty".into());
    }
    let kb = state.rag.lock().await.clone();
    let maps = crate::network_connector::CommandTranslator::translate_all(&intent, &kb);

    // Gate each vendor CLI through the Safety Linter before exposing to UI.
    for m in &maps {
        let lint = guardrails::lint_command(&m.cli);
        if !lint.allowed {
            state.roi.record_block();
            return Err(format!(
                "Vendor CLI blocked by Safety Linter ({}): {}",
                format!("{:?}", m.vendor).to_lowercase(),
                lint.reason
            ));
        }
    }
    Ok(maps)
}

#[tauri::command]
pub async fn simulate_vendor_exec(
    state: tauri::State<'_, AppState>,
    intent: String,
) -> Result<Vec<ExecResult>, String> {
    let intent = intent.trim().to_string();
    if intent.is_empty() {
        return Err("Intent cannot be empty".into());
    }
    let kb = state.rag.lock().await.clone();
    let pairs = NetworkConnector::translate_and_preview(&intent, &kb);

    let mut out = Vec::new();
    for (translated, exec) in pairs {
        let lint = guardrails::lint_command(&translated.cli);
        if !lint.allowed {
            state.roi.record_block();
            return Err(format!(
                "SSH sim blocked by Safety Linter: {}",
                lint.reason
            ));
        }
        if lint.requires_hitl {
            return Err(format!(
                "SSH sim requires HITL (risk={:?}): run Multi-Agent Pipeline and Approve first. {}",
                lint.risk, lint.reason
            ));
        }
        out.push(exec);
    }
    Ok(out)
}

#[tauri::command]
pub fn list_lab_devices() -> Vec<crate::network_connector::DeviceTarget> {
    NetworkConnector::default_lab_devices()
}

#[tauri::command]
pub fn ssh_exec_lab(
    state: tauri::State<'_, AppState>,
    target_id: String,
    command: String,
) -> Result<ExecResult, String> {
    // Safety-first: never execute lab SSH without deterministic lint.
    let lint = guardrails::lint_command(&command);
    if !lint.allowed {
        state.roi.record_block();
        return Err(format!("SSH blocked by Safety Linter: {}", lint.reason));
    }
    if lint.requires_hitl {
        let cleared = state.approved_commands.lock().contains(&command);
        if !cleared {
            return Err(format!(
                "SSH requires HITL Approve first (risk={:?}): {}",
                lint.risk, lint.reason
            ));
        }
    }

    let devices = NetworkConnector::default_lab_devices();
    let target = devices
        .iter()
        .find(|d| d.id == target_id)
        .ok_or_else(|| format!("Unknown target {target_id}"))?;
    NetworkConnector::exec(target, &command).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_inventory() -> Vec<crate::adapters::InventoryDevice> {
    crate::adapters::lab_inventory()
}

#[tauri::command]
pub fn translate_vendor_payloads(command: String) -> Result<Vec<crate::adapters::VendorPayload>, String> {
    let command = command.trim().to_string();
    if command.is_empty() {
        return Err("Command cannot be empty".into());
    }
    let lint = guardrails::lint_command(&command);
    if !lint.allowed {
        return Err(format!("Translation blocked by Safety Linter: {}", lint.reason));
    }
    Ok(crate::adapters::translate_all_vendors(&command))
}

#[tauri::command]
pub async fn execute_approved_action(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    action_id: String,
    command: String,
    device_id: Option<String>,
) -> Result<crate::executor::ExecutionReport, String> {
    let lint = guardrails::lint_with_context(&command, "");
    if !lint.allowed {
        return Err(format!("Execute blocked by Safety Linter: {}", lint.reason));
    }
    if lint.requires_hitl {
        let cleared = state.approved_commands.lock().contains(&command);
        let history = state.audit.get_audit_history(30).await.unwrap_or_default();
        let history_ok = history.iter().any(|e| {
            (e.id == action_id || e.final_command == command)
                && (e.decision == "HITL-Approved" || e.decision == "Auto-Approved")
        });
        if !cleared && !history_ok {
            return Err(
                "Execute requires Auto-Approved or HITL-Approved action first".into(),
            );
        }
    }

    state.approved_commands.lock().insert(command.clone());

    let tel = Arc::clone(&state.telemetry);
    let req = crate::executor::ExecutionRequest {
        action_id: action_id.clone(),
        command: command.clone(),
        device_id,
    };

    let report = crate::executor::execute_approved(&req, &tel, |line| {
        let _ = app.emit("console-feed", &line);
    })
    .await;

    let _ = app.emit("execution-complete", &report);
    let _ = app.emit("telemetry-tick", &state.telemetry.snapshot());

    // Persist verification note on audit row
    let note = format!(
        "Executed · {:?} · {}",
        report.verification.status, report.verification.message
    );
    let _ = state
        .audit
        .update_decision(
            &action_id,
            if report.verification.status == crate::executor::ValidationStatus::Verified {
                "HITL-Approved"
            } else {
                "HITL-Approved"
            },
            Some(&format!(
                "{}\n\n--- verification ---\n{}\n{}",
                report.vendor_payload.body,
                note,
                report.verification.follow_up.clone().unwrap_or_default()
            )),
        )
        .await;

    Ok(report)
}

#[tauri::command]
pub async fn search_audit_logs(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<crate::db::AuditLogEntry>, String> {
    state
        .audit
        .search_logs(&query, limit.unwrap_or(40))
        .await
}

/// TMF641 mock — receive OSS/BSS service order, hydrate TMF637 inventory, run pipeline.
#[derive(Debug, Serialize)]
pub struct NorthboundIngressResult {
    pub ingress: crate::northbound::ExternalIntentResult,
    pub pipeline: PipelineResult,
}

#[tauri::command]
pub async fn receive_external_intent(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    order: Option<crate::northbound::ServiceOrder>,
    model: Option<String>,
) -> Result<NorthboundIngressResult, String> {
    let order = order.unwrap_or_else(crate::northbound::demo_service_order);
    let ingress = crate::northbound::receive_external_intent(order);

    let _ = app.emit(
        "console-feed",
        &crate::executor::ConsoleLine {
            ts: chrono::Utc::now().to_rfc3339(),
            level: "info".into(),
            message: format!(
                "[DEBUG] TMF641 service order {} received ({} · {})",
                ingress.order.id, ingress.order.service_type, ingress.order.priority
            ),
        },
    );
    let _ = app.emit(
        "console-feed",
        &crate::executor::ConsoleLine {
            ts: chrono::Utc::now().to_rfc3339(),
            level: "info".into(),
            message: format!(
                "[DEBUG] TMF637 inventory hydrate → {} device(s)",
                ingress.hydration.matched_devices.len()
            ),
        },
    );

    let pipeline = analyze_network_intent(
        app.clone(),
        state,
        AnalyzeRequest {
            intent: ingress.enriched_intent.clone(),
            model,
        },
    )
    .await?;

    let _ = app.emit(
        "console-feed",
        &crate::executor::ConsoleLine {
            ts: chrono::Utc::now().to_rfc3339(),
            level: "ok".into(),
            message: format!(
                "[SUCCESS] Order {} → pipeline {} ({})",
                ingress.order.id, pipeline.id, pipeline.status
            ),
        },
    );

    Ok(NorthboundIngressResult { ingress, pipeline })
}

#[tauri::command]
pub fn get_demo_service_order() -> crate::northbound::ServiceOrder {
    crate::northbound::demo_service_order()
}

