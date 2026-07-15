//! Mock northbound executor — simulated SSH / RESTCONF push + closed-loop verification.
//! Emits live console lines to the frontend via Tauri events.

use crate::adapters::{self, InventoryDevice, VendorBrand, VendorPayload};
use crate::telemetry::{HealthSnapshot, TelemetryEngine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLine {
    pub ts: String,
    pub level: String, // info | apply | ok | warn | error
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub action_id: String,
    pub command: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Verified,
    DriftDetected,
    Pending,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub status: ValidationStatus,
    pub device_id: String,
    pub before_score: u8,
    pub after_score: u8,
    pub message: String,
    pub follow_up: Option<String>, // Rollback | Refinement plan
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub execution_id: String,
    pub action_id: String,
    pub device: InventoryDevice,
    pub vendor_payload: VendorPayload,
    pub console: Vec<ConsoleLine>,
    pub success: bool,
    pub verification: VerificationResult,
}

fn line(level: &str, message: impl Into<String>) -> ConsoleLine {
    ConsoleLine {
        ts: Utc::now().to_rfc3339(),
        level: level.into(),
        message: message.into(),
    }
}

/// Simulate SSH/RESTCONF push with staged console feed. Callback receives live lines.
pub async fn execute_approved<F>(
    req: &ExecutionRequest,
    telemetry: &Arc<TelemetryEngine>,
    mut on_console: F,
) -> ExecutionReport
where
    F: FnMut(ConsoleLine),
{
    let device = req
        .device_id
        .as_deref()
        .and_then(adapters::find_device)
        .unwrap_or_else(|| adapters::resolve_target(&req.command));

    let vendor_payload = adapters::translate_for_vendor(&req.command, &device);
    let execution_id = Uuid::new_v4().to_string();
    let mut console = Vec::new();

    let emit = |console: &mut Vec<ConsoleLine>, on_console: &mut F, level: &str, msg: String| {
        let l = line(level, msg);
        on_console(l.clone());
        console.push(l);
    };

    emit(
        &mut console,
        &mut on_console,
        "info",
        format!(
            "[DEBUG] Connecting to {} ({})…",
            device.hostname, device.mgmt_ip
        ),
    );
    tokio::time::sleep(Duration::from_millis(400)).await;

    let proto_label = match device.vendor {
        VendorBrand::CiscoIosXe => "XML RPC (NETCONF)",
        VendorBrand::HuaweiVrp => "XML NETCONF (VRP)",
        VendorBrand::NokiaSros => "JSON-RPC / gNMI",
    };
    emit(
        &mut console,
        &mut on_console,
        "apply",
        format!("[DEBUG] Sending {proto_label}…"),
    );
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Echo first lines of vendor payload into console
    for (i, pline) in vendor_payload.body.lines().take(6).enumerate() {
        emit(
            &mut console,
            &mut on_console,
            "info",
            format!("  ▸ [{i}] {pline}"),
        );
        tokio::time::sleep(Duration::from_millis(80)).await;
    }

    emit(
        &mut console,
        &mut on_console,
        "apply",
        "[DEBUG] Validating candidate datastore…".into(),
    );
    tokio::time::sleep(Duration::from_millis(450)).await;

    let txn: u32 = (Uuid::new_v4().as_u128() % 9000 + 1000) as u32;
    emit(
        &mut console,
        &mut on_console,
        "ok",
        format!("[SUCCESS] Commit confirmed. Transaction ID: {txn}"),
    );
    tokio::time::sleep(Duration::from_millis(300)).await;

    emit(
        &mut console,
        &mut on_console,
        "info",
        "[DEBUG] Waiting 5s for post-execution telemetry verification…".into(),
    );

    let before = telemetry.snapshot();
    let before_score = score_for_device(&before, &device);

    // Closed-loop wait
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Nudge telemetry toward improvement for this device (simulates successful push)
    telemetry.apply_execution_effect(&device.hostname, true);
    let after = telemetry.snapshot();
    let after_score = score_for_device(&after, &device);

    let verification = if after_score >= before_score {
        emit(
            &mut console,
            &mut on_console,
            "ok",
            format!(
                "Post-Execution Validation VERIFIED — health {before_score} → {after_score}"
            ),
        );
        VerificationResult {
            status: ValidationStatus::Verified,
            device_id: device.id.clone(),
            before_score,
            after_score,
            message: "Telemetry shows expected improvement.".into(),
            follow_up: None,
        }
    } else {
        emit(
            &mut console,
            &mut on_console,
            "warn",
            format!(
                "Drift detected — health {before_score} → {after_score}. Generating refinement…"
            ),
        );
        let follow = if after_score + 5 < before_score {
            Some(format!(
                "ROLLBACK: restore prior candidate on {} and re-run Safety Linter",
                device.hostname
            ))
        } else {
            Some(format!(
                "REFINEMENT: retune QoS weights on {} (site={})",
                device.hostname, device.site_class
            ))
        };
        emit(
            &mut console,
            &mut on_console,
            "warn",
            follow.clone().unwrap_or_default(),
        );
        VerificationResult {
            status: ValidationStatus::DriftDetected,
            device_id: device.id.clone(),
            before_score,
            after_score,
            message: "Telemetry did not improve as predicted — closed-loop follow-up required."
                .into(),
            follow_up: follow,
        }
    };

    emit(
        &mut console,
        &mut on_console,
        "ok",
        format!("Session closed · execution_id={execution_id}"),
    );

    ExecutionReport {
        execution_id,
        action_id: req.action_id.clone(),
        device,
        vendor_payload,
        console,
        success: true,
        verification,
    }
}

fn score_for_device(snap: &HealthSnapshot, device: &InventoryDevice) -> u8 {
    if let Some(n) = snap.nodes.iter().find(|n| {
        n.label.eq_ignore_ascii_case(&device.hostname) || n.id.contains(&device.id)
    }) {
        let base = 100.0 - n.cpu_pct * 0.5;
        return base.clamp(40.0, 99.0) as u8;
    }
    snap.overall_score
}

/// Force a verification-only check (no push) for UI re-validate.
#[allow(dead_code)]
pub fn verify_only(
    telemetry: &TelemetryEngine,
    device: &InventoryDevice,
    before_score: u8,
) -> VerificationResult {
    let after = telemetry.snapshot();
    let after_score = score_for_device(&after, device);
    if after_score >= before_score {
        VerificationResult {
            status: ValidationStatus::Verified,
            device_id: device.id.clone(),
            before_score,
            after_score,
            message: "Re-check OK.".into(),
            follow_up: None,
        }
    } else {
        VerificationResult {
            status: ValidationStatus::DriftDetected,
            device_id: device.id.clone(),
            before_score,
            after_score,
            message: "Drift still present.".into(),
            follow_up: Some("REFINEMENT: schedule Judge re-analysis".into()),
        }
    }
}
