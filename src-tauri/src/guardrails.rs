//! Deterministic Safety Linter / Guardrails.
//! Runs BEFORE any Human-in-the-Loop notification.
//! Blacklist patterns are evaluated with Rust regex — no LLM involvement.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub allowed: bool,
    pub risk: RiskLevel,
    pub matched_rules: Vec<String>,
    pub reason: String,
    pub requires_hitl: bool,
    pub auto_approvable: bool,
}

struct BlacklistRule {
    id: &'static str,
    pattern: Regex,
    risk: RiskLevel,
    description: &'static str,
}

static BLACKLIST: Lazy<Vec<BlacklistRule>> = Lazy::new(|| {
    vec![
        rule(
            "BL-001",
            r"(?i)\b(shutdown|power[\s_-]?off|halt)\b.*\b(core[_-]?router|core[_-]?switch|bng|mme|amf)\b",
            RiskLevel::Critical,
            "Forbidden: shutting down core network elements",
        ),
        rule(
            "BL-002",
            r"(?i)\b(delete|rm|erase|wipe|purge)\b.*\b(config|configuration|running-config|startup-config)\b",
            RiskLevel::Critical,
            "Forbidden: deleting device configuration",
        ),
        rule(
            "BL-003",
            r"(?i)\b(format|mkfs|dd\s+if=)\b",
            RiskLevel::Critical,
            "Forbidden: destructive filesystem operations",
        ),
        rule(
            "BL-004",
            r"(?i)\b(disable|teardown)\b.*\b(firewall|acl|security[_-]?policy|ips|ids)\b",
            RiskLevel::Critical,
            "Forbidden: disabling security controls",
        ),
        rule(
            "BL-005",
            r"(?i)\b(factory[\s_-]?reset|write\s+erase|erase\s+startup)\b",
            RiskLevel::Critical,
            "Forbidden: factory reset / erase startup",
        ),
        rule(
            "BL-006",
            r"(?i)\b(clear|flush)\b.*\b(bgp|ospf|isis|mpls)\b.*\b(all|entire|full)\b",
            RiskLevel::High,
            "High risk: clearing entire routing protocol state",
        ),
        rule(
            "BL-007",
            r"(?i)\b(reload|reboot|restart)\b.*\b(chassis|supervisor|control[_-]?plane)\b",
            RiskLevel::High,
            "High risk: control-plane reload",
        ),
        rule(
            "BL-008",
            r"(?i)\b(open|permit)\b.*\b(any\s+any|0\.0\.0\.0/0)\b",
            RiskLevel::High,
            "High risk: overly permissive ACL",
        ),
        rule(
            "BL-009",
            r"(?i)\b(qos|traffic[\s_-]?shape|rate[\s_-]?limit|bandwidth)\b",
            RiskLevel::Medium,
            "Medium risk: QoS / traffic engineering change",
        ),
        rule(
            "BL-010",
            r"(?i)\b(optimize|tune|adjust)\b.*\b(threshold|timer|metric|weight)\b",
            RiskLevel::Low,
            "Low risk: optimization / tuning",
        ),
    ]
});

fn rule(id: &'static str, pattern: &str, risk: RiskLevel, description: &'static str) -> BlacklistRule {
    BlacklistRule {
        id,
        pattern: Regex::new(pattern).expect("invalid blacklist regex"),
        risk,
        description,
    }
}

/// Deterministic validation of an AI-proposed command string.
pub fn lint_command(command: &str) -> LintResult {
    let mut matched = Vec::new();
    let mut highest = RiskLevel::Low;

    for rule in BLACKLIST.iter() {
        if rule.pattern.is_match(command) {
            matched.push(format!("{}: {}", rule.id, rule.description));
            highest = max_risk(highest, rule.risk.clone());
        }
    }

    let blocked = matches!(highest, RiskLevel::Critical) && !matched.is_empty()
        && matched.iter().any(|m| m.starts_with("BL-00") && {
            // Critical blacklist IDs BL-001..BL-005 are hard blocks
            m.starts_with("BL-001")
                || m.starts_with("BL-002")
                || m.starts_with("BL-003")
                || m.starts_with("BL-004")
                || m.starts_with("BL-005")
        });

    // Graduated autonomy:
    // Low  → auto-approvable
    // Medium → HITL recommended
    // High/Critical → HITL mandatory; Critical blacklist → hard block
    let auto_approvable = matches!(highest, RiskLevel::Low) && !blocked;
    let requires_hitl = !auto_approvable;

    if blocked {
        LintResult {
            allowed: false,
            risk: RiskLevel::Critical,
            reason: matched
                .first()
                .cloned()
                .unwrap_or_else(|| "Blocked by safety blacklist".into()),
            matched_rules: matched,
            requires_hitl: true,
            auto_approvable: false,
        }
    } else if matched.is_empty() {
        LintResult {
            allowed: true,
            risk: RiskLevel::Low,
            reason: "No blacklist matches — eligible for graduated autonomy".into(),
            matched_rules: vec![],
            requires_hitl: false,
            auto_approvable: true,
        }
    } else {
        LintResult {
            allowed: true,
            risk: highest.clone(),
            reason: format!(
                "Matched {} rule(s); risk={:?}; HITL={}",
                matched.len(),
                highest,
                requires_hitl
            ),
            matched_rules: matched,
            requires_hitl,
            auto_approvable,
        }
    }
}

fn max_risk(a: RiskLevel, b: RiskLevel) -> RiskLevel {
    use RiskLevel::*;
    let rank = |r: &RiskLevel| match r {
        Low => 0,
        Medium => 1,
        High => 2,
        Critical => 3,
    };
    if rank(&b) > rank(&a) { b } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_core_router_shutdown() {
        let r = lint_command("shutdown core_router cr-01");
        assert!(!r.allowed);
        assert_eq!(r.risk, RiskLevel::Critical);
    }

    #[test]
    fn blocks_delete_config() {
        let r = lint_command("delete running-config on pe-west");
        assert!(!r.allowed);
    }

    #[test]
    fn allows_simple_optimize() {
        let r = lint_command("optimize threshold for cell edge throughput");
        assert!(r.allowed);
        assert!(r.auto_approvable);
    }
}
