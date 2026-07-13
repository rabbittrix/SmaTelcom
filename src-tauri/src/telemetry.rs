//! Mock network telemetry simulator — emits JSON events every N seconds.

use chrono::Utc;
use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const MAX_EVENTS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub id: String,
    pub timestamp: String,
    pub site: String,
    pub element: String,
    pub metric: String,
    pub value: f64,
    pub unit: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub overall_score: u8,
    pub latency_ms: f64,
    pub packet_loss_pct: f64,
    pub throughput_gbps: f64,
    pub active_alarms: u32,
    pub sites_online: u32,
    pub sites_total: u32,
    pub last_event: Option<TelemetryEvent>,
    pub recent_events: Vec<TelemetryEvent>,
}

pub struct TelemetryEngine {
    events: RwLock<VecDeque<TelemetryEvent>>,
    health: RwLock<HealthSnapshot>,
}

impl TelemetryEngine {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(MAX_EVENTS)),
            health: RwLock::new(HealthSnapshot {
                overall_score: 92,
                latency_ms: 18.0,
                packet_loss_pct: 0.12,
                throughput_gbps: 42.5,
                active_alarms: 1,
                sites_online: 47,
                sites_total: 48,
                last_event: None,
                recent_events: vec![],
            }),
        }
    }

    pub fn start(self: Arc<Self>, interval: Duration) {
        std::thread::spawn(move || {
            loop {
                self.tick();
                std::thread::sleep(interval);
            }
        });
    }

    pub fn snapshot(&self) -> HealthSnapshot {
        let mut snap = self.health.read().clone();
        snap.recent_events = self.events.read().iter().rev().take(40).cloned().collect();
        snap.last_event = snap.recent_events.first().cloned();
        snap
    }

    fn tick(&self) {
        let mut rng = rand::thread_rng();
        let sites = [
            "DC-East",
            "DC-West",
            "RAN-North",
            "RAN-South",
            "Edge-POP-1",
            "Edge-POP-2",
            "Core-Peering",
        ];
        let elements = [
            "pe-router-01",
            "agg-sw-02",
            "gnodeb-441",
            "upf-03",
            "amf-01",
            "firewall-edge",
            "optics-mux-7",
        ];
        let metrics: [(&str, &str, f64, f64); 6] = [
            ("latency", "ms", 5.0, 80.0),
            ("packet_loss", "%", 0.0, 2.5),
            ("cpu", "%", 12.0, 95.0),
            ("throughput", "Gbps", 1.0, 100.0),
            ("jitter", "ms", 0.5, 15.0),
            ("optical_power", "dBm", -12.0, -2.0),
        ];

        let (metric, unit, lo, hi) = metrics[rng.gen_range(0..metrics.len())];
        let value: f64 = rng.gen_range(lo..hi);
        let severity = if value > hi * 0.85 || (metric == "packet_loss" && value > 1.0) {
            "warning"
        } else if value > hi * 0.95 {
            "critical"
        } else {
            "info"
        };

        let site = sites[rng.gen_range(0..sites.len())].to_string();
        let element = elements[rng.gen_range(0..elements.len())].to_string();
        let rounded = (value * 100.0).round() / 100.0;

        let event = TelemetryEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            site: site.clone(),
            element: element.clone(),
            metric: metric.to_string(),
            value: rounded,
            unit: unit.to_string(),
            severity: severity.to_string(),
            message: format!("{element}@{site} {metric}={rounded:.2}{unit}"),
       };

        {
            let mut q = self.events.write();
            if q.len() >= MAX_EVENTS {
                q.pop_front();
            }
            q.push_back(event.clone());
        }

        let mut h = self.health.write();
        h.latency_ms = rng.gen_range(12.0..45.0);
        h.packet_loss_pct = rng.gen_range(0.01..1.8);
        h.throughput_gbps = rng.gen_range(28.0..95.0);
        h.active_alarms = if severity == "critical" { 3 } else if severity == "warning" { 2 } else { 1 };
        h.sites_online = if rng.gen_bool(0.08) { 46 } else { 47 };
        h.overall_score = (100.0
            - h.latency_ms * 0.4
            - h.packet_loss_pct * 12.0
            - h.active_alarms as f64 * 4.0)
            .clamp(40.0, 99.0) as u8;
        h.last_event = Some(event);
    }
}
