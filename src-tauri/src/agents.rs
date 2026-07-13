//! Multi-agent decision pipeline:
//! Performance, Security, Topology agents → Judge Agent synthesis.
//! Safety linter runs before HITL exposure. Digital Twin predicts impact.

use crate::digital_twin::{self, PredictedImpact};
use crate::error::SmaResult;
use crate::guardrails::{self, LintResult, RiskLevel};
use crate::network_connector::{CommandTranslator, TranslatedCommand};
use crate::ollama::OllamaClient;
use crate::rag::KnowledgeBase;
use crate::telemetry::HealthSnapshot;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOpinion {
    pub agent: String,
    pub analysis: String,
    pub recommendation: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub id: String,
    pub intent: String,
    pub opinions: Vec<AgentOpinion>,
    pub judge_summary: String,
    pub proposed_command: String,
    pub decision_logic: String,
    pub risk: RiskLevel,
    pub lint: LintResult,
    pub status: String,
    pub knowledge_used: bool,
    pub predicted_impact: PredictedImpact,
    pub vendor_commands: Vec<TranslatedCommand>,
    pub duration_ms: u64,
}

pub async fn run_pipeline(
    ollama: &OllamaClient,
    kb: &KnowledgeBase,
    intent: &str,
    health: &HealthSnapshot,
    model_pref: Option<&str>,
) -> SmaResult<PipelineResult> {
    let started = Instant::now();
    let intent = intent.trim();
    if intent.is_empty() {
        return Err(crate::error::SmaError::InvalidIntent(
            "Network intent cannot be empty".into(),
        ));
    }

    let model = ollama.resolve_model(model_pref).await?;
    let rag_ctx = kb.context_for(intent, 3);
    let knowledge_used = !rag_ctx.contains("No relevant knowledge-base");
    let vendor_commands = CommandTranslator::translate_all(intent, kb);

    let health_ctx = format!(
        "Network health score={}, latency={}ms, loss={}%, throughput={}Gbps, alarms={}, sites={}/{}",
        health.overall_score,
        health.latency_ms,
        health.packet_loss_pct,
        health.throughput_gbps,
        health.active_alarms,
        health.sites_online,
        health.sites_total
    );

    let agents = [
        (
            "Performance Agent",
            "You are the Performance Agent for a telco AN Level-4 orchestrator. Focus on latency, throughput, congestion, QoS, and capacity. Be concise.",
        ),
        (
            "Security Agent",
            "You are the Security Agent for a telco AN Level-4 orchestrator. Focus on threats, ACL integrity, anomalous traffic, and blast radius. Be concise.",
        ),
        (
            "Topology Agent",
            "You are the Topology Agent for a telco AN Level-4 orchestrator. Focus on path diversity, site roles, dependency risk, and failover. Be concise.",
        ),
    ];

    let mut opinions = Vec::new();
    for (name, system) in agents {
        let prompt = format!(
            "{system}\n\nNETWORK STATE:\n{health_ctx}\n\nKNOWLEDGE BASE:\n{rag_ctx}\n\nNETWORK INTENT:\n{intent}\n\nRespond in this exact format:\nANALYSIS: <2-3 sentences>\nRECOMMENDATION: <one concrete network action>\nCONFIDENCE: <0.0-1.0>"
        );
        let raw = ollama.generate(&model, &prompt).await.unwrap_or_else(|e| {
            format!(
                "ANALYSIS: Agent offline ({e}). Using heuristic fallback.\nRECOMMENDATION: observe and collect more telemetry\nCONFIDENCE: 0.35"
            )
        });
        opinions.push(parse_opinion(name, &raw));
    }

    let opinions_blob = opinions
        .iter()
        .map(|o| {
            format!(
                "[{}] conf={:.2}\n{}\n=> {}",
                o.agent, o.confidence, o.analysis, o.recommendation
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let judge_prompt = format!(
        "You are the Judge Agent for SmaTelcom. Synthesize the three specialist agents into ONE final recommendation.\nNever propose shutdown of core routers, deleting configs, or disabling firewalls.\n\nINTENT: {intent}\n\nAGENT INPUTS:\n{opinions_blob}\n\nRespond in this exact format:\nSUMMARY: <synthesis>\nCOMMAND: <single operational command string>\nLOGIC: <why this decision>\nRISK: <Low|Medium|High|Critical>"
    );

    let judge_raw = ollama.generate(&model, &judge_prompt).await.unwrap_or_else(|_| {
        "SUMMARY: Fallback consensus — continue monitoring with light optimization.\nCOMMAND: optimize threshold for edge cell throughput\nLOGIC: Agents disagree or Ollama unavailable; choose lowest-risk action.\nRISK: Low".into()
    });

    let (judge_summary, proposed_command, decision_logic, risk_hint) = parse_judge(&judge_raw);

    let mut lint = guardrails::lint_command(&proposed_command);
    let risk = if lint.risk > risk_from_str(&risk_hint) {
        lint.risk.clone()
    } else {
        risk_from_str(&risk_hint)
    };

    if matches!(risk, RiskLevel::High | RiskLevel::Critical) {
        lint.requires_hitl = true;
        lint.auto_approvable = false;
    }

    let status = if !lint.allowed {
        "blocked_by_safety".into()
    } else if lint.auto_approvable {
        "auto_approved".into()
    } else {
        "pending_hitl".into()
    };

    let predicted_impact =
        digital_twin::predict_impact(ollama, &model, &proposed_command, health).await?;

    Ok(PipelineResult {
        id: Uuid::new_v4().to_string(),
        intent: intent.to_string(),
        opinions,
        judge_summary,
        proposed_command,
        decision_logic,
        risk,
        lint,
        status,
        knowledge_used,
        predicted_impact,
        vendor_commands,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

fn parse_opinion(agent: &str, raw: &str) -> AgentOpinion {
    let analysis = extract_field(raw, "ANALYSIS").unwrap_or_else(|| raw.chars().take(280).collect());
    let recommendation =
        extract_field(raw, "RECOMMENDATION").unwrap_or_else(|| "No recommendation".into());
    let confidence = extract_field(raw, "CONFIDENCE")
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);

    AgentOpinion {
        agent: agent.to_string(),
        analysis,
        recommendation,
        confidence,
    }
}

fn parse_judge(raw: &str) -> (String, String, String, String) {
    let summary = extract_field(raw, "SUMMARY").unwrap_or_else(|| raw.chars().take(300).collect());
    let command = extract_field(raw, "COMMAND")
        .unwrap_or_else(|| "optimize threshold for edge cell throughput".into());
    let logic = extract_field(raw, "LOGIC").unwrap_or_else(|| "Judge synthesis".into());
    let risk = extract_field(raw, "RISK").unwrap_or_else(|| "Medium".into());
    (summary, command, logic, risk)
}

fn extract_field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let prefix = format!("{key}:");
        if line.to_uppercase().starts_with(&prefix.to_uppercase()) {
            return Some(line[prefix.len()..].trim().to_string());
        }
    }
    None
}

fn risk_from_str(s: &str) -> RiskLevel {
    match s.trim().to_lowercase().as_str() {
        "low" => RiskLevel::Low,
        "medium" => RiskLevel::Medium,
        "high" => RiskLevel::High,
        "critical" => RiskLevel::Critical,
        _ => RiskLevel::Medium,
    }
}
