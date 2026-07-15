//! Deterministic Safety Linter / Guardrails.
//! Runs BEFORE any Human-in-the-Loop notification.
//! Blacklist patterns are evaluated with Rust regex — no LLM involvement.
//! Dubai-grade blast-radius awareness escalates Core / Downtown / DataCenter targets.

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
    /// When true, matching this rule hard-blocks the command.
    hard_block: bool,
}

static BLACKLIST: Lazy<Vec<BlacklistRule>> = Lazy::new(|| {
    vec![
        rule(
            "BL-001",
            r"(?i)\b(shutdown|power[\s_-]?off|halt)\b.*\b(core[_-]?router|core[_-]?switch|bng|mme|amf)\b",
            RiskLevel::Critical,
            "Forbidden: shutting down core network elements",
            true,
        ),
        rule(
            "BL-002",
            r"(?i)\b(delete|rm|erase|wipe|purge)\b.*\b(config|configuration|running-config|startup-config)\b",
            RiskLevel::Critical,
            "Forbidden: deleting device configuration",
            true,
        ),
        rule(
            "BL-003",
            r"(?i)\b(format|mkfs|dd\s+if=)\b",
            RiskLevel::Critical,
            "Forbidden: destructive filesystem operations",
            true,
        ),
        rule(
            "BL-004",
            r"(?i)\b(disable|teardown)\b.*\b(firewall|acl|security[_-]?policy|ips|ids)\b",
            RiskLevel::Critical,
            "Forbidden: disabling security controls",
            true,
        ),
        rule(
            "BL-005",
            r"(?i)\b(factory[\s_-]?reset|write\s+erase|erase\s+startup)\b",
            RiskLevel::Critical,
            "Forbidden: factory reset / erase startup",
            true,
        ),
        rule(
            "BL-006",
            r"(?i)\b(clear|flush)\b.*\b(bgp|ospf|isis|mpls)\b.*\b(all|entire|full)\b",
            RiskLevel::High,
            "High risk: clearing entire routing protocol state",
            false,
        ),
        rule(
            "BL-007",
            r"(?i)\b(reload|reboot|restart)\b.*\b(chassis|supervisor|control[_-]?plane)\b",
            RiskLevel::High,
            "High risk: control-plane reload",
            false,
        ),
        rule(
            "BL-008",
            r"(?i)\b(open|permit)\b.*\b(any\s+any|0\.0\.0\.0/0)\b",
            RiskLevel::High,
            "High risk: overly permissive ACL",
            false,
        ),
        rule(
            "BL-009",
            r"(?i)\b(qos|traffic[\s_-]?shape|rate[\s_-]?limit|bandwidth)\b",
            RiskLevel::Medium,
            "Medium risk: QoS / traffic engineering change",
            false,
        ),
        rule(
            "BL-010",
            r"(?i)\b(optimize|tune|adjust)\b.*\b(threshold|timer|metric|weight)\b",
            RiskLevel::Low,
            "Low risk: optimization / tuning",
            false,
        ),
        // Dubai-grade hard-blocks
        rule(
            "BL-011",
            r"(?i)\b(no\s+logging|disable\s+logging|logging\s+disable|undebug\s+all|no\s+debug)\b",
            RiskLevel::Critical,
            "Forbidden: disabling logging / audit trail",
            true,
        ),
        rule(
            "BL-012",
            r"(?i)\b(root[\s_-]?cert|trustpoint|crypto\s+pki|ca[\s_-]?certificate|remove\s+certificate)\b",
            RiskLevel::Critical,
            "Forbidden: modifying root / PKI certificates",
            true,
        ),
        rule(
            "BL-013",
            r"(?i)\b(bypass\s*2fa|disable\s*(mfa|2fa|two[\s_-]?factor)|skip\s*(mfa|2fa)|authentication\s+bypass|no\s+aaa\s+authentication)\b",
            RiskLevel::Critical,
            "Forbidden: bypassing 2FA / MFA authentication layer",
            true,
        ),
    ]
});

/// Critical site / blast-radius markers — escalate regardless of AI risk rating.
static CRITICAL_SITE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(core|downtown|data[\s_-]?center|datacenter|dc[\s_-]?(east|west|central)|core[\s_-]?peering|bng[\s_-]?core)\b",
    )
    .expect("critical site regex")
});

fn rule(
    id: &'static str,
    pattern: &str,
    risk: RiskLevel,
    description: &'static str,
    hard_block: bool,
) -> BlacklistRule {
    BlacklistRule {
        id,
        pattern: Regex::new(pattern).expect("invalid blacklist regex"),
        risk,
        description,
        hard_block,
    }
}

/// Deterministic validation of an AI-proposed command string.
pub fn lint_command(command: &str) -> LintResult {
    lint_with_context(command, "")
}

/// Lint command + optional intent/context (blast-radius site awareness).
pub fn lint_with_context(command: &str, context: &str) -> LintResult {
    let blob = if context.is_empty() {
        command.to_string()
    } else {
        format!("{command}\n{context}")
    };

    let mut matched = Vec::new();
    let mut highest = RiskLevel::Low;
    let mut hard_blocked = false;

    for rule in BLACKLIST.iter() {
        if rule.pattern.is_match(&blob) {
            matched.push(format!("{}: {}", rule.id, rule.description));
            highest = max_risk(highest, rule.risk.clone());
            if rule.hard_block {
                hard_blocked = true;
            }
        }
    }

    // Blast-radius escalation: Core / Downtown / DataCenter → at least High.
    if CRITICAL_SITE.is_match(&blob) {
        let before = highest.clone();
        highest = max_risk(highest, RiskLevel::High);
        matched.push(format!(
            "BR-001: Critical-site blast radius — target mentions Core/Downtown/DataCenter \
             (escalated {:?} → {:?})",
            before, highest
        ));
        // Aggressive change on a critical site → Critical + HITL (still allowed unless hard-block).
        if AGGRESSIVE_CHANGE.is_match(&blob) {
            highest = RiskLevel::Critical;
            matched.push(
                "BR-002: Aggressive change on critical site — escalated to Critical".into(),
            );
        }
    }

    let auto_approvable = matches!(highest, RiskLevel::Low) && !hard_blocked;
    let requires_hitl = !auto_approvable;

    if hard_blocked {
        LintResult {
            allowed: false,
            risk: RiskLevel::Critical,
            reason: matched
                .iter()
                .find(|m| m.starts_with("BL-"))
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

static AGGRESSIVE_CHANGE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(increase\s+(tx\s*)?power|boost|aggressive|shutdown|reload|clear\s+bgp|permit\s+any)\b",
    )
    .expect("aggressive change regex")
});

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

    #[test]
    fn blocks_disable_logging() {
        let r = lint_command("no logging on pe-edge");
        assert!(!r.allowed);
        assert!(r.matched_rules.iter().any(|m| m.starts_with("BL-011")));
    }

    #[test]
    fn blocks_2fa_bypass() {
        let r = lint_command("bypass 2fa for lab operator");
        assert!(!r.allowed);
    }

    #[test]
    fn escalates_downtown_site() {
        let r = lint_with_context(
            "optimize threshold for cell edge",
            "Reduce congestion Downtown RAN sector",
        );
        assert!(r.allowed);
        assert!(r.risk >= RiskLevel::High);
        assert!(r.requires_hitl);
        assert!(!r.auto_approvable);
    }

    #[test]
    fn escalates_core_aggressive() {
        let r = lint_command("increase tx power on core pe-router-01");
        assert_eq!(r.risk, RiskLevel::Critical);
        assert!(r.requires_hitl);
    }
}
