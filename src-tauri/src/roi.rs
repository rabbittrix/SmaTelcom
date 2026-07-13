//! Real-time ROI metrics: engineering hours saved + risks mitigated.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Assumed average human handling time per network intent (minutes).
pub const HUMAN_BASELINE_MINUTES: f64 = 45.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoiSnapshot {
    pub engineering_hours_saved: f64,
    pub risks_mitigated: u64,
    pub intents_processed: u64,
    pub total_ai_ms: u64,
    pub human_baseline_minutes: f64,
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
        let human_ms = HUMAN_BASELINE_MINUTES * 60_000.0;
        let saved_ms = (human_ms - ai_duration_ms as f64).max(0.0) * s.intents_processed as f64
            / s.intents_processed.max(1) as f64;
        // Cumulative: each intent saves (human - ai)
        let per_intent_saved_h =
            (HUMAN_BASELINE_MINUTES * 60_000.0 - ai_duration_ms as f64).max(0.0) / 3_600_000.0;
        s.engineering_hours_saved += per_intent_saved_h;
        let _ = saved_ms;
    }

    pub fn record_block(&self) {
        self.inner.lock().risks_mitigated += 1;
    }

    pub fn snapshot(&self) -> RoiSnapshot {
        self.inner.lock().clone()
    }
}
