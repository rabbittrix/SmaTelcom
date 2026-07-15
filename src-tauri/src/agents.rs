//! Multi-agent decision pipeline:
//! Performance, Security, Topology agents → Judge Agent conflict resolution.
//! Priority: Security/Stability > Compliance > Performance (RAG-cited).
//! Safety linter runs before HITL exposure. Digital Twin predicts impact.

use crate::digital_twin::{self, PredictedImpact};
use crate::error::SmaResult;
use crate::guardrails::{self, LintResult, RiskLevel};
use crate::network_connector::{CommandTranslator, TranslatedCommand};
use crate::ollama::OllamaClient;
use crate::rag::{DocumentChunk, KnowledgeBase};
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
pub struct ConflictResolution {
    pub conflict_detected: bool,
    pub winner: String,
    pub loser: String,
    pub priority_applied: String,
    pub policy_citation: String,
    pub rationale: String,
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
    pub conflict_resolution: ConflictResolution,
    /// RAG chunks that justified the Judge decision ("Show Your Source").
    pub evidence_sources: Vec<DocumentChunk>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PipelineProgress {
    pub stage: String,
    pub level: String,
    pub message: String,
}

const PERF_SYSTEM: &str = "You are the Performance Agent for SmarTelcom (telco AN Level-4). \
Focus on latency, throughput, congestion, QoS, Tx power, and capacity. Be concise. \
Propose ONE concrete operational action. Prefer measurable optimizations.";

const SEC_SYSTEM: &str = "You are the Security Agent for SmarTelcom (telco AN Level-4). \
Focus on threats, ACL integrity, interference risk, blast radius, and compliance. Be concise. \
Flag risks explicitly. Prefer containment over aggressive performance gains.";

const TOPO_SYSTEM: &str = "You are the Topology Agent for SmarTelcom (telco AN Level-4). \
Focus on path diversity, site roles (Core/Edge/RAN), dependency risk, and failover. Be concise.";

pub async fn run_pipeline<F>(
    ollama: &OllamaClient,
    kb: &KnowledgeBase,
    intent: &str,
    health: &HealthSnapshot,
    model_pref: Option<&str>,
    mut on_progress: F,
) -> SmaResult<PipelineResult>
where
    F: FnMut(PipelineProgress),
{
    let started = Instant::now();
    let intent = intent.trim();
    if intent.is_empty() {
        return Err(crate::error::SmaError::InvalidIntent(
            "Network intent cannot be empty".into(),
        ));
    }

    on_progress(PipelineProgress {
        stage: "rag".into(),
        level: "info".into(),
        message: "Grounding intent in local knowledge_base…".into(),
    });

    let model = ollama.resolve_model(model_pref).await?;
    let evidence_sources = kb.search(intent, 3);
    let rag_ctx = if evidence_sources.is_empty() {
        kb.context_for(intent, 4)
    } else {
        evidence_sources
            .iter()
            .map(|c| format!("[{}]\n{}", c.source, c.content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    };
    let knowledge_used = !evidence_sources.is_empty()
        && !rag_ctx.contains("No relevant knowledge-base");
    let vendor_commands = CommandTranslator::translate_all(intent, kb);
    let policy_excerpt = extract_policy_citation(&rag_ctx);

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
        ("Performance Agent", PERF_SYSTEM),
        ("Security Agent", SEC_SYSTEM),
        ("Topology Agent", TOPO_SYSTEM),
    ];

    let mut opinions = Vec::new();
    for (name, system) in agents {
        on_progress(PipelineProgress {
            stage: "agent".into(),
            level: "agent".into(),
            message: format!("{name} analyzing…"),
        });
        let prompt = format!(
            "{system}\n\nNETWORK STATE:\n{health_ctx}\n\nPOLICY / KNOWLEDGE BASE (RAG):\n{rag_ctx}\n\nNETWORK INTENT:\n{intent}\n\nRespond in this exact format:\nANALYSIS: <2-3 sentences>\nRECOMMENDATION: <one concrete network action>\nCONFIDENCE: <0.0-1.0>"
        );
        let raw = ollama.generate(&model, &prompt).await.unwrap_or_else(|e| {
            format!(
                "ANALYSIS: Agent offline ({e}). Using heuristic fallback.\nRECOMMENDATION: observe and collect more telemetry\nCONFIDENCE: 0.35"
            )
        });
        let opinion = parse_opinion(name, &raw);
        on_progress(PipelineProgress {
            stage: "agent".into(),
            level: "agent".into(),
            message: format!(
                "{name}: {} (conf {:.0}%)",
                opinion.recommendation,
                opinion.confidence * 100.0
            ),
        });
        opinions.push(opinion);
    }

    // Deterministic conflict resolution BEFORE / WITH Judge (Safety-first hierarchy).
    let conflict = resolve_perf_security_conflict(&opinions, &rag_ctx, &policy_excerpt);

    on_progress(PipelineProgress {
        stage: "judge".into(),
        level: "judge".into(),
        message: if conflict.conflict_detected {
            format!(
                "Judge resolving conflict — {} wins over {} (Security/Stability > Performance)",
                conflict.winner, conflict.loser
            )
        } else {
            "Judge Agent synthesizing specialist opinions…".into()
        },
    });

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
        r#"You are the Judge Agent for SmarTelcom (Autonomous Networks Level 4).
You do NOT merely summarize. You RESOLVE CONFLICTS between specialist agents.

PRIORITY HIERARCHY (mandatory, highest first):
1) Security / Stability
2) Compliance (policy manuals from RAG)
3) Performance

CONFLICT RULE:
If Performance proposes an aggressive change (e.g. increase Tx power, boost throughput aggressively)
AND Security flags risk (interference, ACL breach, blast radius, compliance violation),
you MUST side with Security/Stability unless RAG policy explicitly authorizes the Performance action.
Cite the specific policy text from RAG in LOGIC.

DETERMINISTIC PRE-RESOLUTION (already computed — honor this):
conflict_detected={conflict_detected}
winner={winner}
loser={loser}
priority_applied={priority}
policy_citation={citation}
rationale={rationale}

INTENT: {intent}

RAG POLICY CONTEXT:
{rag_ctx}

AGENT INPUTS:
{opinions_blob}

Never propose shutdown of core routers, deleting configs, disabling firewalls, disabling logging, modifying root certs, or bypassing 2FA.

Respond in this exact format:
SUMMARY: <who won and the final operational posture>
COMMAND: <single operational command string — must align with the winner when conflict_detected=true>
LOGIC: <explicitly state why you chose one agent over another and quote/cite the RAG policy>
RISK: <Low|Medium|High|Critical>
WINNER: <Performance Agent|Security Agent|Topology Agent|Consensus>"#,
        conflict_detected = conflict.conflict_detected,
        winner = conflict.winner,
        loser = conflict.loser,
        priority = conflict.priority_applied,
        citation = conflict.policy_citation,
        rationale = conflict.rationale,
        intent = intent,
        rag_ctx = rag_ctx,
        opinions_blob = opinions_blob,
    );

    let judge_raw = ollama.generate(&model, &judge_prompt).await.unwrap_or_else(|_| {
        if conflict.conflict_detected {
            let sec = opinions
                .iter()
                .find(|o| o.agent.contains("Security"))
                .map(|o| o.recommendation.clone())
                .unwrap_or_else(|| "observe and collect more telemetry".into());
            format!(
                "SUMMARY: Conflict resolved in favor of Security/Stability over Performance.\n\
                 COMMAND: {sec}\n\
                 LOGIC: {rationale} Policy: {citation}\n\
                 RISK: Medium\n\
                 WINNER: Security Agent",
                rationale = conflict.rationale,
                citation = conflict.policy_citation,
            )
        } else {
            "SUMMARY: Fallback consensus — continue monitoring with light optimization.\n\
             COMMAND: optimize threshold for edge cell throughput\n\
             LOGIC: Agents aligned or Ollama unavailable; choose lowest-risk Performance-safe action.\n\
             RISK: Low\n\
             WINNER: Consensus"
                .into()
        }
    });

    let (mut judge_summary, mut proposed_command, mut decision_logic, risk_hint, winner_field) =
        parse_judge(&judge_raw);

    // Enforce deterministic winner when conflict was detected (LLM cannot override hierarchy).
    let mut conflict_resolution = conflict.clone();
    if conflict_resolution.conflict_detected {
        if let Some(sec) = opinions.iter().find(|o| o.agent.contains("Security")) {
            proposed_command = sec.recommendation.clone();
        }
        decision_logic = format!(
            "{}\n\n[Deterministic hierarchy] {} beat {} because {}. Policy citation: \"{}\"",
            decision_logic,
            conflict_resolution.winner,
            conflict_resolution.loser,
            conflict_resolution.rationale,
            conflict_resolution.policy_citation
        );
        if !judge_summary.to_lowercase().contains("security") {
            judge_summary = format!(
                "Conflict resolved: {} preferred over {}. {}",
                conflict_resolution.winner, conflict_resolution.loser, judge_summary
            );
        }
        if conflict_resolution.winner == "Security Agent" {
            // keep
        } else if !winner_field.is_empty() {
            conflict_resolution.winner = winner_field;
        }
    } else if !winner_field.is_empty() {
        conflict_resolution.winner = winner_field;
    }

    on_progress(PipelineProgress {
        stage: "judge".into(),
        level: "judge".into(),
        message: format!(
            "Judge: {} · winner={}",
            judge_summary.chars().take(120).collect::<String>(),
            conflict_resolution.winner
        ),
    });

    on_progress(PipelineProgress {
        stage: "safety".into(),
        level: "safety".into(),
        message: format!("Safety Linter validating: {proposed_command}"),
    });

    // CRITICAL: deterministic linter ALWAYS runs before HITL — include intent for blast radius.
    let mut lint = guardrails::lint_with_context(&proposed_command, intent);
    let risk = if lint.risk > risk_from_str(&risk_hint) {
        lint.risk.clone()
    } else {
        risk_from_str(&risk_hint)
    };

    if matches!(
        risk,
        RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical
    ) {
        lint.requires_hitl = true;
        lint.auto_approvable = false;
        lint.risk = max_risk_level(lint.risk.clone(), risk.clone());
    }

    let status = if !lint.allowed {
        "blocked_by_safety".into()
    } else if lint.requires_hitl || !lint.auto_approvable {
        "pending_hitl".into()
    } else {
        "auto_approved".into()
    };

    on_progress(PipelineProgress {
        stage: "safety".into(),
        level: "safety".into(),
        message: format!("Linter: {} · status={status}", lint.reason),
    });

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
        conflict_resolution,
        evidence_sources,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Detect Performance vs Security conflict and apply hierarchy using RAG policy text.
fn resolve_perf_security_conflict(
    opinions: &[AgentOpinion],
    rag_ctx: &str,
    policy_excerpt: &str,
) -> ConflictResolution {
    let perf = opinions.iter().find(|o| o.agent.contains("Performance"));
    let sec = opinions.iter().find(|o| o.agent.contains("Security"));

    let (Some(perf), Some(sec)) = (perf, sec) else {
        return ConflictResolution {
            conflict_detected: false,
            winner: "Consensus".into(),
            loser: "—".into(),
            priority_applied: "Security/Stability > Compliance > Performance".into(),
            policy_citation: policy_excerpt.to_string(),
            rationale: "Insufficient agent outputs for conflict analysis.".into(),
        };
    };

    let perf_aggressive = is_aggressive_performance(&perf.recommendation)
        || is_aggressive_performance(&perf.analysis);
    let sec_flags_risk = flags_security_risk(&sec.recommendation) || flags_security_risk(&sec.analysis);

    let rag_favors_security = rag_prefers_security(rag_ctx);
    let rag_allows_perf = rag_explicitly_allows_performance(rag_ctx);

    if perf_aggressive && sec_flags_risk && !(rag_allows_perf && !rag_favors_security) {
        return ConflictResolution {
            conflict_detected: true,
            winner: "Security Agent".into(),
            loser: "Performance Agent".into(),
            priority_applied: "Security/Stability > Compliance > Performance".into(),
            policy_citation: policy_excerpt.to_string(),
            rationale: format!(
                "Performance proposed an aggressive change ({}) while Security flagged risk ({}). \
                 Per AN Level-4 hierarchy, Security/Stability wins unless RAG explicitly authorizes \
                 the performance action. RAG does not authorize it here.",
                truncate(&perf.recommendation, 80),
                truncate(&sec.analysis, 80)
            ),
        };
    }

    if perf_aggressive && sec_flags_risk && rag_allows_perf {
        return ConflictResolution {
            conflict_detected: true,
            winner: "Performance Agent".into(),
            loser: "Security Agent".into(),
            priority_applied: "Compliance exception via RAG policy".into(),
            policy_citation: policy_excerpt.to_string(),
            rationale: format!(
                "Performance change is aggressive, but RAG policy explicitly permits controlled \
                 optimization under the stated conditions. Citing: {}",
                truncate(policy_excerpt, 160)
            ),
        };
    }

    ConflictResolution {
        conflict_detected: false,
        winner: "Consensus".into(),
        loser: "—".into(),
        priority_applied: "Security/Stability > Compliance > Performance".into(),
        policy_citation: policy_excerpt.to_string(),
        rationale: "No hard Performance↔Security conflict detected; Judge may synthesize.".into(),
    }
}

fn is_aggressive_performance(text: &str) -> bool {
    let t = text.to_lowercase();
    [
        "tx power",
        "transmit power",
        "increase power",
        "boost",
        "aggressive",
        "max throughput",
        "raise power",
        "open bandwidth",
        "disable qos",
        "remove rate limit",
        "permit any",
    ]
    .iter()
    .any(|k| t.contains(k))
}

fn flags_security_risk(text: &str) -> bool {
    let t = text.to_lowercase();
    [
        "interference",
        "acl",
        "breach",
        "risk",
        "threat",
        "blast radius",
        "compliance",
        "unauthorized",
        "deny",
        "unsafe",
        "expose",
        "attack",
        "violate",
    ]
    .iter()
    .any(|k| t.contains(k))
}

fn rag_prefers_security(rag: &str) -> bool {
    let t = rag.to_lowercase();
    [
        "must not",
        "forbidden",
        "prohibited",
        "safety policy",
        "do not disable",
        "require approval",
        "hitl",
        "security first",
        "stability",
    ]
    .iter()
    .any(|k| t.contains(k))
}

fn rag_explicitly_allows_performance(rag: &str) -> bool {
    let t = rag.to_lowercase();
    // Narrow: only when policy clearly authorizes tuning / congestion relief.
    (t.contains("authorized") || t.contains("permitted") || t.contains("may optimize"))
        && (t.contains("congestion") || t.contains("qos") || t.contains("threshold"))
        && !t.contains("must not increase")
}

fn extract_policy_citation(rag_ctx: &str) -> String {
    let cleaned = rag_ctx
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("No relevant"))
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.is_empty() {
        "No specific policy chunk retrieved — default to Security/Stability priority.".into()
    } else {
        truncate(&cleaned, 280)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

fn max_risk_level(a: RiskLevel, b: RiskLevel) -> RiskLevel {
    if a > b { a } else { b }
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

fn parse_judge(raw: &str) -> (String, String, String, String, String) {
    let summary = extract_field(raw, "SUMMARY").unwrap_or_else(|| raw.chars().take(300).collect());
    let command = extract_field(raw, "COMMAND")
        .unwrap_or_else(|| "optimize threshold for edge cell throughput".into());
    let logic = extract_field(raw, "LOGIC").unwrap_or_else(|| "Judge synthesis".into());
    let risk = extract_field(raw, "RISK").unwrap_or_else(|| "Medium".into());
    let winner = extract_field(raw, "WINNER").unwrap_or_default();
    (summary, command, logic, risk, winner)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_wins_on_aggressive_perf() {
        let opinions = vec![
            AgentOpinion {
                agent: "Performance Agent".into(),
                analysis: "Congestion high".into(),
                recommendation: "Increase Tx Power on RAN-North".into(),
                confidence: 0.8,
            },
            AgentOpinion {
                agent: "Security Agent".into(),
                analysis: "Potential interference and ACL breach risk".into(),
                recommendation: "Cap power and monitor interference".into(),
                confidence: 0.9,
            },
        ];
        let rag = "Core safety policy: must not increase transmit power without HITL approval.";
        let r = resolve_perf_security_conflict(&opinions, rag, rag);
        assert!(r.conflict_detected);
        assert_eq!(r.winner, "Security Agent");
    }
}
