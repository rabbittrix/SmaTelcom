//! Northbound OSS/BSS integration — TM Forum Open API mocks (TMF641 / TMF637).
//! Simulates ServiceNow / Amdocs service orders hydrating AI context from inventory.

use crate::adapters::{self, InventoryDevice};
use serde::{Deserialize, Serialize};

/// TMF641-style Service Order (simplified).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOrder {
    pub id: String,
    pub service_type: String,
    pub priority: String,
    pub intent: String,
    #[serde(default)]
    pub related_party: Option<String>,
    #[serde(default)]
    pub geographic_site: Option<String>,
}

/// TMF637 Resource Inventory hydrate result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryHydration {
    pub order_id: String,
    pub matched_devices: Vec<InventoryDevice>,
    pub context_block: String,
    pub primary_target: Option<InventoryDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalIntentResult {
    pub order: ServiceOrder,
    pub hydration: InventoryHydration,
    /// Intent string enriched with inventory context for the multi-agent pipeline.
    pub enriched_intent: String,
}

/// Demo payload matching the Phase 6 brief (Dubai Mall / 5G slicing).
pub fn demo_service_order() -> ServiceOrder {
    ServiceOrder {
        id: "ORDER-123".into(),
        service_type: "5G_Slicing".into(),
        priority: "High".into(),
        intent: "Ensure low latency for Dubai Mall".into(),
        related_party: Some("ServiceNow / Amdocs OSS".into()),
        geographic_site: Some("Downtown".into()),
    }
}

/// TMF637 inventory bridge — hydrate AI context with Vendor / IP / Role before pipeline.
pub fn hydrate_inventory(order: &ServiceOrder) -> InventoryHydration {
    let inventory = adapters::lab_inventory();
    let site_hint = order
        .geographic_site
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    let intent_l = order.intent.to_lowercase();
    let stype = order.service_type.to_lowercase();

    let mut scored: Vec<(i32, InventoryDevice)> = inventory
        .into_iter()
        .map(|d| {
            let mut score = 0;
            let site = d.site_class.to_lowercase();
            if !site_hint.is_empty() && site.contains(&site_hint) {
                score += 40;
            }
            if intent_l.contains("dubai") || intent_l.contains("mall") || intent_l.contains("downtown")
            {
                if site.contains("downtown") {
                    score += 35;
                }
            }
            if intent_l.contains("latency") || intent_l.contains("sla") {
                if d.role == "PE" || d.role == "AGG" || d.role == "UPF" {
                    score += 20;
                }
            }
            if stype.contains("5g") || stype.contains("slic") {
                if d.role == "UPF" || d.role == "AMF" || d.role == "RAN" || site.contains("ran") {
                    score += 25;
                }
            }
            if intent_l.contains("core") && site.contains("core") {
                score += 30;
            }
            if intent_l.contains(&d.hostname.to_lowercase()) {
                score += 50;
            }
            (score, d)
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let matched: Vec<InventoryDevice> = scored
        .into_iter()
        .filter(|(s, _)| *s > 0)
        .take(4)
        .map(|(_, d)| d)
        .collect();

    let matched = if matched.is_empty() {
        adapters::lab_inventory().into_iter().take(3).collect()
    } else {
        matched
    };

    let primary = matched.first().cloned();
    let lines: Vec<String> = matched
        .iter()
        .map(|d| {
            format!(
                "- {} | vendor={:?} | role={} | site={} | mgmt={} | proto={}",
                d.hostname, d.vendor, d.role, d.site_class, d.mgmt_ip, d.protocol_hint
            )
        })
        .collect();

    let context_block = format!(
        "[TMF637 Resource Inventory]\nOrder {} · {} · priority={}\nMatched NEs:\n{}",
        order.id,
        order.service_type,
        order.priority,
        lines.join("\n")
    );

    InventoryHydration {
        order_id: order.id.clone(),
        matched_devices: matched,
        context_block,
        primary_target: primary,
    }
}

/// Build pipeline-ready intent from a TMF641 order + TMF637 hydrate.
pub fn enrich_intent(order: &ServiceOrder, hydration: &InventoryHydration) -> String {
    let target = hydration
        .primary_target
        .as_ref()
        .map(|d| {
            format!(
                "Primary target: {} ({}) @ {} [{}]",
                d.hostname, d.role, d.mgmt_ip, d.site_class
            )
        })
        .unwrap_or_else(|| "Primary target: lab inventory default".into());

    format!(
        "{}\n\n[TMF641 Service Order {}]\nserviceType={} priority={}\nsource={}\n{}\n\n{}",
        order.intent.trim(),
        order.id,
        order.service_type,
        order.priority,
        order
            .related_party
            .as_deref()
            .unwrap_or("external-oss"),
        target,
        hydration.context_block
    )
}

/// Full northbound ingress: parse order → hydrate inventory → enrich intent.
pub fn receive_external_intent(order: ServiceOrder) -> ExternalIntentResult {
    let hydration = hydrate_inventory(&order);
    let enriched_intent = enrich_intent(&order, &hydration);
    ExternalIntentResult {
        order,
        hydration,
        enriched_intent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dubai_order_prefers_downtown() {
        let order = demo_service_order();
        let h = hydrate_inventory(&order);
        assert!(!h.matched_devices.is_empty());
        let sites: Vec<_> = h
            .matched_devices
            .iter()
            .map(|d| d.site_class.as_str())
            .collect();
        assert!(
            sites.iter().any(|s| s.contains("Downtown") || s.contains("RAN") || s.contains("DataCenter")),
            "expected site-relevant devices, got {sites:?}"
        );
        let enriched = enrich_intent(&order, &h);
        assert!(enriched.contains("ORDER-123"));
        assert!(enriched.contains("Dubai Mall"));
    }
}
