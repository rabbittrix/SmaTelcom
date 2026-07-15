//! Real-time ROI metrics: engineering hours saved + risks mitigated.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Assumed average human handling time per network intent (minutes).
pub const HUMAN_BASELINE_MINUTES: f64 = 45.0;
/// Phase-3 Impact Report: minutes saved per Low-risk auto-approve.
pub const AUTO_APPROVE_MINUTES_SAVED: f64 = 15.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoiSnapshot {
    pub engineering_hours_saved: f64,
    pub risks_mitigated: u64,
    pub intents_processed: u64,
    pub total_ai_ms: u64,
    pub human_baseline_minutes: f64,
    pub auto_approved: u64,
    pub critical_risks_averted: u64,
}

pub struct RoiTracker {
    inner: Mutex<RoiSnapshot>,
}

impl RoiTracker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RoiSnapshot {
                human_baseline_minutes: HUMAN_BASELINE_MINUTES,
                ..Default::default()
            }),
        }
    }

    pub fn record_intent(&self, ai_duration_ms: u64, blocked: bool) {
        let mut s = self.inner.lock();
        s.intents_processed += 1;
        s.total_ai_ms += ai_duration_ms;
        if blocked {
            s.risks_mitigated += 1;
        }
        let per_intent_saved_h =
            (HUMAN_BASELINE_MINUTES * 60_000.0 - ai_duration_ms as f64).max(0.0) / 3_600_000.0;
        s.engineering_hours_saved += per_intent_saved_h;
    }

    pub fn record_auto_approve(&self) {
        let mut s = self.inner.lock();
        s.auto_approved += 1;
        s.engineering_hours_saved += AUTO_APPROVE_MINUTES_SAVED / 60.0;
    }

    pub fn record_block(&self) {
        self.inner.lock().risks_mitigated += 1;
    }

    pub fn record_critical_block(&self) {
        let mut s = self.inner.lock();
        s.risks_mitigated += 1;
        s.critical_risks_averted += 1;
    }

    pub fn snapshot(&self) -> RoiSnapshot {
        self.inner.lock().clone()
    }
}
