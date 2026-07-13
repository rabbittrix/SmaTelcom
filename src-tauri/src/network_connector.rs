//! Multi-vendor network connector with SSH simulation + Command Translator.
//! Maps generic intents (RAG-backed) to Cisco IOS / Huawei VRP CLI syntax.
//! Author: Roberto de Souza <rabbittrix@hotmail.com>

use crate::error::{SmaError, SmaResult};
use crate::rag::KnowledgeBase;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Vendor {
    CiscoIos,
    HuaweiVrp,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTarget {
    pub id: String,
    pub hostname: String,
    pub vendor: Vendor,
    pub host: String,
    pub port: u16,
    /// When true, never opens a real TCP/SSH socket — returns canned CLI.
    pub simulate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedCommand {
    pub intent: String,
    pub vendor: Vendor,
    pub cli: String,
    pub rag_sources: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub target_id: String,
    pub vendor: Vendor,
    pub command: String,
    pub output: String,
    pub simulated: bool,
    pub success: bool,
}

/// Local command-translator “vector” store: keyword RAG over vendor manuals.
pub struct CommandTranslator;

impl CommandTranslator {
    pub fn translate(intent: &str, vendor: Vendor, kb: &KnowledgeBase) -> TranslatedCommand {
        let vendor_key = match vendor {
            Vendor::CiscoIos => "cisco ios",
            Vendor::HuaweiVrp => "huawei vrp",
            Vendor::Generic => "generic",
        };
        let query = format!("{intent} {vendor_key} cli interface");
        let hits = kb.search(&query, 4);
        let rag_sources: Vec<String> = hits.iter().map(|h| h.source.clone()).collect();
        let rag_blob = hits
            .iter()
            .map(|h| h.content.to_lowercase())
            .collect::<Vec<_>>()
            .join("\n");

        let intent_l = intent.to_lowercase();
        let (cli, confidence) = match vendor {
            Vendor::CiscoIos => cisco_map(&intent_l, &rag_blob),
            Vendor::HuaweiVrp => huawei_map(&intent_l, &rag_blob),
            Vendor::Generic => (
                format!("# generic intent: {intent}\nshow status"),
                0.4,
            ),
        };

        let boost = if !hits.is_empty() { 0.15 } else { 0.0 };
        TranslatedCommand {
            intent: intent.to_string(),
            vendor,
            cli,
            rag_sources,
            confidence: (confidence + boost).min(0.99),
        }
    }

    pub fn translate_all(intent: &str, kb: &KnowledgeBase) -> Vec<TranslatedCommand> {
        [Vendor::CiscoIos, Vendor::HuaweiVrp]
            .into_iter()
            .map(|v| Self::translate(intent, v, kb))
            .collect()
    }
}

fn cisco_map(intent: &str, rag: &str) -> (String, f32) {
    if intent.contains("interface down") || intent.contains("shut interface") || rag.contains("shutdown")
    {
        (
            "configure terminal\ninterface GigabitEthernet0/0/1\nshutdown\nend\nwrite memory".into(),
            0.82,
        )
    } else if intent.contains("interface up") || intent.contains("no shut") {
        (
            "configure terminal\ninterface GigabitEthernet0/0/1\nno shutdown\nend\nwrite memory".into(),
            0.82,
        )
    } else if intent.contains("congestion") || intent.contains("qos") || intent.contains("shape") {
        (
            "configure terminal\npolicy-map PM-EDGE-SHAPE\n class class-default\n  shape average 500000000\ninterface GigabitEthernet0/0/1\n service-policy output PM-EDGE-SHAPE\nend".into(),
            0.78,
        )
    } else if intent.contains("bgp") && intent.contains("clear") {
        ("clear ip bgp * soft".into(), 0.7)
    } else {
        (
            format!("! Cisco IOS mapping for: {intent}\nshow ip interface brief\nshow interfaces status"),
            0.55,
        )
    }
}

fn huawei_map(intent: &str, rag: &str) -> (String, f32) {
    if intent.contains("interface down") || intent.contains("shut interface") || rag.contains("shutdown")
    {
        (
            "system-view\ninterface GigabitEthernet0/0/1\nshutdown\nquit\nquit\nsave".into(),
            0.82,
        )
    } else if intent.contains("interface up") || intent.contains("undo shut") {
        (
            "system-view\ninterface GigabitEthernet0/0/1\nundo shutdown\nquit\nquit\nsave".into(),
            0.82,
        )
    } else if intent.contains("congestion") || intent.contains("qos") || intent.contains("shape") {
        (
            "system-view\ntraffic behavior TB-EDGE\n car cir 500000 cbs 100000 green pass\nquit\ntraffic policy TP-EDGE\n classifier default behavior TB-EDGE\nquit\ninterface GigabitEthernet0/0/1\n traffic-policy TP-EDGE outbound\nquit\nquit".into(),
            0.78,
        )
    } else if intent.contains("bgp") && intent.contains("clear") {
        ("reset bgp all soft".into(), 0.7)
    } else {
        (
            format!("# Huawei VRP mapping for: {intent}\ndisplay interface brief\ndisplay ip interface brief"),
            0.55,
        )
    }
}

pub struct NetworkConnector;

impl NetworkConnector {
    pub fn default_lab_devices() -> Vec<DeviceTarget> {
        vec![
            DeviceTarget {
                id: "pe-cisco-01".into(),
                hostname: "pe-router-01".into(),
                vendor: Vendor::CiscoIos,
                host: "127.0.0.1".into(),
                port: 22022,
                simulate: true,
            },
            DeviceTarget {
                id: "agg-huawei-02".into(),
                hostname: "agg-sw-02".into(),
                vendor: Vendor::HuaweiVrp,
                host: "127.0.0.1".into(),
                port: 22023,
                simulate: true,
            },
        ]
    }

    /// Execute CLI on a target. Simulation is default for lab safety.
    pub fn exec(target: &DeviceTarget, command: &str) -> SmaResult<ExecResult> {
        if target.simulate {
            return Ok(Self::simulate(target, command));
        }
        Self::ssh_exec(target, command, "lab", "lab")
    }

    fn simulate(target: &DeviceTarget, command: &str) -> ExecResult {
        let banner = match target.vendor {
            Vendor::CiscoIos => format!(
                "{}\nCisco IOS Software, Simulator\nHostname: {}\n",
                target.hostname, target.hostname
            ),
            Vendor::HuaweiVrp => format!(
                "<{}> Huawei Versatile Routing Platform Simulation\n",
                target.hostname
            ),
            Vendor::Generic => format!("[{}] generic cli sim\n", target.hostname),
        };
        let body = if command.to_lowercase().contains("show")
            || command.to_lowercase().contains("display")
        {
            "GigabitEthernet0/0/1   up   up   10.0.0.1/30\nLoopback0              up   up   1.1.1.1/32\n".into()
        } else {
            format!("OK — applied ({})", command.lines().next().unwrap_or("cmd"))
        };
        ExecResult {
            target_id: target.id.clone(),
            vendor: target.vendor,
            command: command.to_string(),
            output: format!("{banner}{body}"),
            simulated: true,
            success: true,
        }
    }

    /// Real SSH via `ssh2` (only when `simulate=false`). Lab credentials only.
    fn ssh_exec(
        target: &DeviceTarget,
        command: &str,
        user: &str,
        password: &str,
    ) -> SmaResult<ExecResult> {
        let addr = format!("{}:{}", target.host, target.port);
        let tcp = TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|e| SmaError::Internal(format!("bad addr {addr}: {e}")))?,
            Duration::from_secs(3),
        )
        .map_err(|e| {
            SmaError::Internal(format!(
                "SSH TCP connect failed to {addr}: {e}. Enable simulate=true for lab."
            ))
        })?;
        tcp.set_read_timeout(Some(Duration::from_secs(8))).ok();
        tcp.set_write_timeout(Some(Duration::from_secs(8))).ok();

        let mut session = ssh2::Session::new()
            .map_err(|e| SmaError::Internal(format!("ssh2 session: {e}")))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| SmaError::Internal(format!("ssh handshake: {e}")))?;
        session
            .userauth_password(user, password)
            .map_err(|e| SmaError::Internal(format!("ssh auth: {e}")))?;

        let mut channel = session
            .channel_session()
            .map_err(|e| SmaError::Internal(format!("ssh channel: {e}")))?;
        channel
            .exec(command)
            .map_err(|e| SmaError::Internal(format!("ssh exec: {e}")))?;
        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| SmaError::Internal(format!("ssh read: {e}")))?;
        let _ = channel.wait_close();

        Ok(ExecResult {
            target_id: target.id.clone(),
            vendor: target.vendor,
            command: command.to_string(),
            output,
            simulated: false,
            success: true,
        })
    }

    pub fn translate_and_preview(
        intent: &str,
        kb: &KnowledgeBase,
    ) -> Vec<(TranslatedCommand, ExecResult)> {
        let devices = Self::default_lab_devices();
        CommandTranslator::translate_all(intent, kb)
            .into_iter()
            .filter_map(|t| {
                let device = devices.iter().find(|d| d.vendor == t.vendor)?;
                let exec = Self::simulate(device, &t.cli);
                Some((t, exec))
            })
            .collect()
    }
}
