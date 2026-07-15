//! Northbound protocol drivers — "How" the orchestrator talks to lab NEs.
//! `agents` = What · `guardrails` = If · `drivers` = How.
//! Simulated only — no live sockets to production hardware.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NorthboundProtocol {
    Netconf,
    Gnmi,
}

/// Trait for protocol payload generation (Clean Architecture "How").
pub trait NetworkDriver: Send + Sync {
    fn protocol_name(&self) -> &'static str;
    fn generate_payload(&self, command: &str) -> String;
    fn content_type(&self) -> &'static str;
}

pub struct NetconfDriver {
    pub target: String,
}

impl NetworkDriver for NetconfDriver {
    fn protocol_name(&self) -> &'static str {
        "netconf"
    }

    fn content_type(&self) -> &'static str {
        "application/yang-data+xml"
    }

    fn generate_payload(&self, command: &str) -> String {
        let cmd = xml_escape(command);
        let tgt = xml_escape(&self.target);
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rpc message-id="101" xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
  <edit-config>
    <target><candidate/></target>
    <default-operation>merge</default-operation>
    <config>
      <smartelcom-intent xmlns="urn:smartelcom:an4:1.0">
        <device>{tgt}</device>
        <operation>{cmd}</operation>
        <origin>judge-agent</origin>
        <safety-gate>passed</safety-gate>
      </smartelcom-intent>
    </config>
  </edit-config>
</rpc>
<!-- Followed by: <rpc><commit><confirmed/><confirm-timeout>60</confirm-timeout></commit></rpc> -->
"#
        )
    }
}

pub struct GnmiDriver {
    pub target: String,
}

impl NetworkDriver for GnmiDriver {
    fn protocol_name(&self) -> &'static str {
        "gnmi"
    }

    fn content_type(&self) -> &'static str {
        "application/json"
    }

    fn generate_payload(&self, command: &str) -> String {
        let cmd = json_escape(command);
        let tgt = json_escape(&self.target);
        format!(
            r#"{{
  "prefix": {{ "target": "{tgt}", "origin": "smartelcom" }},
  "update": [{{
    "path": {{
      "elem": [
        {{ "name": "system" }},
        {{ "name": "config" }},
        {{ "name": "intent-action" }}
      ]
    }},
    "val": {{
      "json_ietf_val": "{{ \"command\": \"{cmd}\", \"safety_gate\": \"passed\", \"source\": \"judge-agent\" }}"
    }}
  }}],
  "extension": {{ "atomic": true, "replace": true }}
}}"#
        )
    }
}

/// Pick NETCONF for config changes; gNMI for telemetry/state-oriented commands.
pub fn select_driver(command: &str, target: &str) -> Box<dyn NetworkDriver> {
    let t = command.to_lowercase();
    let telemetry_ish = [
        "telemetry",
        "sensor",
        "counter",
        "state",
        "monitor",
        "observe",
        "get ",
        "show ",
        "subscribe",
    ]
    .iter()
    .any(|k| t.contains(k));

    if telemetry_ish {
        Box::new(GnmiDriver {
            target: target.to_string(),
        })
    } else {
        Box::new(NetconfDriver {
            target: target.to_string(),
        })
    }
}

pub fn generate_technical_payload(command: &str, target: &str) -> (NorthboundProtocol, String) {
    let driver = select_driver(command, target);
    let body = driver.generate_payload(command);
    let proto = if driver.protocol_name() == "gnmi" {
        NorthboundProtocol::Gnmi
    } else {
        NorthboundProtocol::Netconf
    };
    (proto, body)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverRequest {
    pub command: String,
    pub protocol: NorthboundProtocol,
    pub target: String,
    pub dry_run: bool,
    pub action_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverPayload {
    pub id: String,
    pub protocol: NorthboundProtocol,
    pub target: String,
    pub content_type: String,
    pub body: String,
    pub dry_run: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverResult {
    pub payload: DriverPayload,
    pub status: String,
    pub message: String,
    pub commit_id: Option<String>,
    pub simulated: bool,
}

pub fn build_payload(req: &DriverRequest) -> DriverPayload {
    let created_at = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let driver: Box<dyn NetworkDriver> = match req.protocol {
        NorthboundProtocol::Netconf => Box::new(NetconfDriver {
            target: req.target.clone(),
        }),
        NorthboundProtocol::Gnmi => Box::new(GnmiDriver {
            target: req.target.clone(),
        }),
    };
    DriverPayload {
        id,
        protocol: req.protocol,
        target: req.target.clone(),
        content_type: driver.content_type().into(),
        body: driver.generate_payload(&req.command),
        dry_run: req.dry_run,
        created_at,
    }
}

pub fn commit(req: &DriverRequest) -> DriverResult {
    let payload = build_payload(req);
    if req.dry_run {
        return DriverResult {
            payload,
            status: "dry_run".into(),
            message: "Dry Run — payload generated; no NETCONF/gNMI session opened.".into(),
            commit_id: None,
            simulated: true,
        };
    }

    let commit_id = format!("cfg-{}", &Uuid::new_v4().to_string()[..8]);
    let (status, message) = match req.protocol {
        NorthboundProtocol::Netconf => (
            "commit_confirm".into(),
            format!(
                "NETCONF <commit/> confirmed on {} — candidate → running (simulated). commit-id={}",
                req.target, commit_id
            ),
        ),
        NorthboundProtocol::Gnmi => (
            "commit_success".into(),
            format!(
                "gNMI SetResponse OK on {} — atomic replace applied (simulated). commit-id={}",
                req.target, commit_id
            ),
        ),
    };

    DriverResult {
        payload,
        status,
        message,
        commit_id: Some(commit_id),
        simulated: true,
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netconf_trait_wraps_edit_config() {
        let d = NetconfDriver {
            target: "pe-01".into(),
        };
        let xml = d.generate_payload("optimize threshold");
        assert!(xml.contains("<edit-config>"));
        assert!(xml.contains("optimize threshold"));
    }

    #[test]
    fn gnmi_trait_json() {
        let d = GnmiDriver {
            target: "amf-01".into(),
        };
        let j = d.generate_payload("observe telemetry counters");
        assert!(j.contains("intent-action"));
    }

    #[test]
    fn select_gnmi_for_monitor() {
        let (p, _) = generate_technical_payload("monitor state on edge", "ne-1");
        assert_eq!(p, NorthboundProtocol::Gnmi);
    }

    #[test]
    fn dry_run_does_not_commit() {
        let req = DriverRequest {
            command: "optimize threshold for edge cell".into(),
            protocol: NorthboundProtocol::Netconf,
            target: "pe-router-01".into(),
            dry_run: true,
            action_id: None,
        };
        let r = commit(&req);
        assert_eq!(r.status, "dry_run");
        assert!(r.commit_id.is_none());
    }
}
