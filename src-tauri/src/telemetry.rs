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
pub struct TopologyNode {
    pub id: String,
    pub label: String,
    pub site: String,
    pub role: String,
    pub x: f64,
    pub y: f64,
    pub status: String,
    pub cpu_pct: f64,
    pub vendor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyLink {
    pub source: String,
    pub target: String,
    pub status: String,
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
    pub nodes: Vec<TopologyNode>,
    pub links: Vec<TopologyLink>,
}

pub struct TelemetryEngine {
    events: RwLock<VecDeque<TelemetryEvent>>,
    health: RwLock<HealthSnapshot>,
    nodes: RwLock<Vec<TopologyNode>>,
    links: RwLock<Vec<TopologyLink>>,
}

impl TelemetryEngine {
    pub fn new() -> Self {
        let nodes = seed_nodes();
        let links = seed_links();
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
                nodes: nodes.clone(),
                links: links.clone(),
            }),
            nodes: RwLock::new(nodes),
            links: RwLock::new(links),
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
        snap.nodes = self.nodes.read().clone();
        snap.links = self.links.read().clone();
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
        h.last_event = Some(event.clone());

        // Drive topology node states from latest telemetry element
        {
            let mut nodes = self.nodes.write();
            for n in nodes.iter_mut() {
                if n.label == element || n.id.contains(&element) {
                    n.status = severity.to_string();
                    if metric == "cpu" {
                        n.cpu_pct = rounded;
                    } else {
                        n.cpu_pct = (n.cpu_pct * 0.8 + rng.gen_range(15.0..70.0) * 0.2)
                            .clamp(5.0, 99.0);
                    }
                } else if rng.gen_bool(0.15) {
                    n.cpu_pct = rng.gen_range(12.0..55.0);
                    n.status = if n.cpu_pct > 80.0 {
                        "warning".into()
                    } else {
                        "ok".into()
                    };
                }
            }
            let mut links = self.links.write();
            for l in links.iter_mut() {
                if l.source.contains(&element) || l.target.contains(&element) {
                    l.status = if severity == "critical" {
                        "degraded".into()
                    } else {
                        "up".into()
                    };
                }
            }
        }
    }
}

fn seed_nodes() -> Vec<TopologyNode> {
    vec![
        node("pe-router-01", "pe-router-01", "DC-East", "PE", 120.0, 180.0, "cisco"),
        node("amf-01", "amf-01", "DC-East", "AMF", 280.0, 120.0, "cisco"),
        node("upf-03", "upf-03", "DC-West", "UPF", 480.0, 140.0, "huawei"),
        node("agg-sw-02", "agg-sw-02", "RAN-North", "AGG", 200.0, 320.0, "huawei"),
        node("gnodeb-441", "gnodeb-441", "RAN-North", "RAN", 120.0, 420.0, "huawei"),
        node("firewall-edge", "firewall-edge", "Edge-POP-1", "FW", 400.0, 300.0, "cisco"),
        node("optics-mux-7", "optics-mux-7", "Core-Peering", "OPT", 560.0, 260.0, "generic"),
    ]
}

fn node(
    id: &str,
    label: &str,
    site: &str,
    role: &str,
    x: f64,
    y: f64,
    vendor: &str,
) -> TopologyNode {
    TopologyNode {
        id: id.into(),
        label: label.into(),
        site: site.into(),
        role: role.into(),
        x,
        y,
        status: "ok".into(),
        cpu_pct: 28.0,
        vendor: vendor.into(),
    }
}

fn seed_links() -> Vec<TopologyLink> {
    vec![
        link("pe-router-01", "amf-01"),
        link("pe-router-01", "agg-sw-02"),
        link("amf-01", "upf-03"),
        link("agg-sw-02", "gnodeb-441"),
        link("upf-03", "firewall-edge"),
        link("firewall-edge", "optics-mux-7"),
        link("pe-router-01", "optics-mux-7"),
    ]
}

fn link(source: &str, target: &str) -> TopologyLink {
    TopologyLink {
        source: source.into(),
        target: target.into(),
        status: "up".into(),
    }
}
