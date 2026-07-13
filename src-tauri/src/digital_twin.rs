//! Digital Twin Lite — pre-execution predicted impact (Judge AI JSON).

use crate::error::SmaResult;
use crate::ollama::OllamaClient;
use crate::telemetry::HealthSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedImpact {
    pub cpu_pct_before: f64,
    pub cpu_pct_after: f64,
    pub latency_ms_before: f64,
    pub latency_ms_after: f64,
    pub throughput_gbps_before: f64,
    pub throughput_gbps_after: f64,
    pub packet_loss_pct_before: f64,
    pub packet_loss_pct_after: f64,
    pub blast_radius: String,
    pub summary: String,
}

pub async fn predict_impact(
    ollama: &OllamaClient,
    model: &str,
    proposed_command: &str,
    health: &HealthSnapshot,
) -> SmaResult<PredictedImpact> {
    let prompt = format!(
        "You are the Judge Agent Digital Twin for a telco network.\n\
         Given CURRENT metrics and a PROPOSED CHANGE, output ONLY valid JSON with keys:\n\
         cpu_pct_before, cpu_pct_after, latency_ms_before, latency_ms_after,\n\
         throughput_gbps_before, throughput_gbps_after,\n\
         packet_loss_pct_before, packet_loss_pct_after, blast_radius, summary.\n\
         Numbers must be realistic floats. No markdown.\n\n\
         CURRENT: latency_ms={}, loss_pct={}, throughput_gbps={}, score={}, alarms={}\n\
         PROPOSED: {}\n",
        health.latency_ms,
        health.packet_loss_pct,
        health.throughput_gbps,
        health.overall_score,
        health.active_alarms,
        proposed_command
    );

    let raw = ollama.generate(model, &prompt).await.unwrap_or_default();
    if let Some(parsed) = parse_json_impact(&raw, health) {
        return Ok(parsed);
    }
    Ok(heuristic_impact(health, proposed_command))
}

fn parse_json_impact(raw: &str, health: &HealthSnapshot) -> Option<PredictedImpact> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    let slice = &raw[start..=end];
    let mut v: PredictedImpact = serde_json::from_str(slice).ok()?;
    if v.latency_ms_before == 0.0 {
        v.latency_ms_before = health.latency_ms;
    }
    if v.throughput_gbps_before == 0.0 {
        v.throughput_gbps_before = health.throughput_gbps;
    }
    Some(v)
}

fn heuristic_impact(health: &HealthSnapshot, cmd: &str) -> PredictedImpact {
    let milder = cmd.to_lowercase().contains("optimize")
        || cmd.to_lowercase().contains("shape")
        || cmd.to_lowercase().contains("threshold");
    let delta_lat = if milder { -2.5 } else { 1.5 };
    let delta_tp = if milder { 3.0 } else { -1.0 };
    PredictedImpact {
        cpu_pct_before: 42.0,
        cpu_pct_after: if milder { 45.0 } else { 58.0 },
        latency_ms_before: health.latency_ms,
        latency_ms_after: (health.latency_ms + delta_lat).max(1.0),
        throughput_gbps_before: health.throughput_gbps,
        throughput_gbps_after: (health.throughput_gbps + delta_tp).max(1.0),
        packet_loss_pct_before: health.packet_loss_pct,
        packet_loss_pct_after: (health.packet_loss_pct * if milder { 0.7 } else { 1.1 }).max(0.01),
        blast_radius: if milder {
            "Single edge / RAN cell".into()
        } else {
            "Aggregation + dependent sites".into()
        },
        summary: "Heuristic digital-twin estimate (Judge JSON unavailable).".into(),
    }
}
