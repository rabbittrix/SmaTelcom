//! Universal Translator — vendor-specific northbound payloads.
//! Same intent → Cisco IOS-XE/XR · Huawei VRP · Nokia SROS templates.
//! agents = What · guardrails = If · drivers/adapters = How.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorBrand {
    CiscoIosXe,
    HuaweiVrp,
    NokiaSros,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryDevice {
    pub id: String,
    pub hostname: String,
    pub vendor: VendorBrand,
    pub site_class: String, // Core | Downtown | DataCenter | Edge | RAN
    pub role: String,
    pub mgmt_ip: String,
    pub protocol_hint: String, // netconf | gnmi | cli
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorPayload {
    pub vendor: VendorBrand,
    pub device_id: String,
    pub format: String, // cli | xml | json-rpc | gnmi
    pub body: String,
    pub summary: String,
}

/// Mock lab inventory used by Judge / Driver selection.
pub fn lab_inventory() -> Vec<InventoryDevice> {
    vec![
        InventoryDevice {
            id: "core-pe-01".into(),
            hostname: "pe-router-01".into(),
            vendor: VendorBrand::CiscoIosXe,
            site_class: "Core".into(),
            role: "PE".into(),
            mgmt_ip: "10.0.0.11".into(),
            protocol_hint: "netconf".into(),
        },
        InventoryDevice {
            id: "dc-amf-01".into(),
            hostname: "amf-01".into(),
            vendor: VendorBrand::CiscoIosXe,
            site_class: "DataCenter".into(),
            role: "AMF".into(),
            mgmt_ip: "10.0.10.21".into(),
            protocol_hint: "netconf".into(),
        },
        InventoryDevice {
            id: "dt-agg-02".into(),
            hostname: "agg-sw-02".into(),
            vendor: VendorBrand::HuaweiVrp,
            site_class: "Downtown".into(),
            role: "AGG".into(),
            mgmt_ip: "10.0.20.32".into(),
            protocol_hint: "netconf".into(),
        },
        InventoryDevice {
            id: "ran-gnb-441".into(),
            hostname: "gnodeb-441".into(),
            vendor: VendorBrand::HuaweiVrp,
            site_class: "RAN".into(),
            role: "RAN".into(),
            mgmt_ip: "10.0.30.44".into(),
            protocol_hint: "netconf".into(),
        },
        InventoryDevice {
            id: "dc-upf-03".into(),
            hostname: "upf-03".into(),
            vendor: VendorBrand::NokiaSros,
            site_class: "DataCenter".into(),
            role: "UPF".into(),
            mgmt_ip: "10.0.10.33".into(),
            protocol_hint: "gnmi".into(),
        },
        InventoryDevice {
            id: "edge-fw-01".into(),
            hostname: "firewall-edge".into(),
            vendor: VendorBrand::NokiaSros,
            site_class: "Edge".into(),
            role: "FW".into(),
            mgmt_ip: "10.0.40.51".into(),
            protocol_hint: "gnmi".into(),
        },
    ]
}

pub fn find_device(device_id_or_host: &str) -> Option<InventoryDevice> {
    let q = device_id_or_host.to_lowercase();
    lab_inventory().into_iter().find(|d| {
        d.id.to_lowercase() == q
            || d.hostname.to_lowercase() == q
            || d.hostname.to_lowercase().contains(&q)
            || d.id.to_lowercase().contains(&q)
    })
}

/// Infer target device from intent / command text using inventory keywords.
pub fn resolve_target(intent_or_command: &str) -> InventoryDevice {
    let t = intent_or_command.to_lowercase();
    for d in lab_inventory() {
        if t.contains(&d.hostname.to_lowercase())
            || t.contains(&d.id.to_lowercase())
            || t.contains(&d.site_class.to_lowercase())
            || t.contains(&d.role.to_lowercase())
        {
            return d;
        }
    }
    // Default Core PE (Cisco)
    lab_inventory().into_iter().next().unwrap()
}

/// Translate a high-level operational command into vendor-native payload.
pub fn translate_for_vendor(command: &str, device: &InventoryDevice) -> VendorPayload {
    match device.vendor {
        VendorBrand::CiscoIosXe => cisco_payload(command, device),
        VendorBrand::HuaweiVrp => huawei_payload(command, device),
        VendorBrand::NokiaSros => nokia_payload(command, device),
    }
}

/// Translate for all vendors (comparison / dry-run multi-view).
pub fn translate_all_vendors(command: &str) -> Vec<VendorPayload> {
    lab_inventory()
        .into_iter()
        .filter(|d| {
            matches!(
                d.vendor,
                VendorBrand::CiscoIosXe | VendorBrand::HuaweiVrp | VendorBrand::NokiaSros
            )
        })
        // One representative per vendor
        .fold(Vec::new(), |mut acc, d| {
            if !acc.iter().any(|p: &VendorPayload| p.vendor == d.vendor) {
                acc.push(translate_for_vendor(command, &d));
            }
            acc
        })
}

fn xml_esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn json_esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn cisco_payload(command: &str, device: &InventoryDevice) -> VendorPayload {
    let cmd = command.trim();
    let body = if cmd.to_lowercase().contains("capacity")
        || cmd.to_lowercase().contains("bandwidth")
        || cmd.to_lowercase().contains("qos")
    {
        format!(
            "!\n! Cisco IOS-XE/XR — {host} ({site})\n\
             configure terminal\n\
             interface GigabitEthernet0/0/1\n\
             bandwidth 1000000\n\
             service-policy output SMARTELCOM-QOS\n\
             ! intent: {cmd}\n\
             commit\n\
             end\n",
            host = device.hostname,
            site = device.site_class,
            cmd = cmd
        )
    } else {
        format!(
            "!\n! Cisco IOS-XE/XR NETCONF edit-config\n\
             <?xml version=\"1.0\"?>\n\
             <rpc message-id=\"cisco-1\" xmlns=\"urn:ietf:params:xml:ns:netconf:base:1.0\">\n\
               <edit-config>\n\
                 <target><candidate/></target>\n\
                 <config>\n\
                   <native xmlns=\"http://cisco.com/ns/yang/Cisco-IOS-XE-native\">\n\
                     <description>{esc}</description>\n\
                   </native>\n\
                 </config>\n\
               </edit-config>\n\
             </rpc>\n",
            esc = xml_esc(cmd)
        )
    };

    VendorPayload {
        vendor: VendorBrand::CiscoIosXe,
        device_id: device.id.clone(),
        format: if body.trim_start().starts_with("<?xml") {
            "xml".into()
        } else {
            "cli".into()
        },
        body,
        summary: format!("Cisco → {} @ {}", device.hostname, device.site_class),
    }
}

fn huawei_payload(command: &str, device: &InventoryDevice) -> VendorPayload {
    let esc = xml_esc(command);
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rpc message-id="hw-1" xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
  <edit-config>
    <target><candidate/></target>
    <config>
      <ifm xmlns="urn:huawei:yang:huawei-ifm">
        <interfaces>
          <interface>
            <ifName>GigabitEthernet0/0/1</ifName>
            <ifDescr>{esc}</ifDescr>
            <bandwidth>1000000</bandwidth>
          </interface>
        </interfaces>
      </ifm>
      <!-- Huawei VRP device={host} site={site} -->
    </config>
  </edit-config>
</rpc>
"#,
        esc = esc,
        host = device.hostname,
        site = device.site_class,
    );

    VendorPayload {
        vendor: VendorBrand::HuaweiVrp,
        device_id: device.id.clone(),
        format: "xml".into(),
        body,
        summary: format!("Huawei VRP → {} @ {}", device.hostname, device.site_class),
    }
}

fn nokia_payload(command: &str, device: &InventoryDevice) -> VendorPayload {
    let esc = json_esc(command);
    let body = if device.protocol_hint == "gnmi" {
        format!(
            r#"{{
  "prefix": {{ "target": "{host}", "origin": "openconfig" }},
  "update": [{{
    "path": {{ "elem": [{{ "name": "interfaces" }}, {{ "name": "interface", "key": {{ "name": "1/1/1" }} }}] }},
    "val": {{
      "json_ietf_val": "{{ \"config\": {{ \"description\": \"{esc}\", \"enabled\": true }} }}"
    }}
  }}],
  "nokia-sros-extension": {{ "site": "{site}", "commit": "confirmed" }}
}}"#,
            host = device.hostname,
            esc = esc,
            site = device.site_class,
        )
    } else {
        format!(
            r#"{{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "set",
  "params": {{
    "path": "/nokia-conf:configure/port[port-id=1/1/1]",
    "value": {{
      "description": "{esc}",
      "admin-state": "enable"
    }},
    "datastore": "candidate"
  }}
}}"#,
            esc = esc,
        )
    };

    VendorPayload {
        vendor: VendorBrand::NokiaSros,
        device_id: device.id.clone(),
        format: if device.protocol_hint == "gnmi" {
            "gnmi".into()
        } else {
            "json-rpc".into()
        },
        body,
        summary: format!("Nokia SROS → {} @ {}", device.hostname, device.site_class),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cisco_and_huawei_differ() {
        let inv = lab_inventory();
        let cisco = inv.iter().find(|d| d.vendor == VendorBrand::CiscoIosXe).unwrap();
        let hw = inv.iter().find(|d| d.vendor == VendorBrand::HuaweiVrp).unwrap();
        let a = translate_for_vendor("Set Interface Capacity 1G", cisco);
        let b = translate_for_vendor("Set Interface Capacity 1G", hw);
        assert_ne!(a.body, b.body);
        assert!(b.body.contains("huawei") || b.body.contains("ifm"));
    }

    #[test]
    fn nokia_gnmi_json() {
        let nokia = lab_inventory()
            .into_iter()
            .find(|d| d.vendor == VendorBrand::NokiaSros)
            .unwrap();
        let p = translate_for_vendor("monitor state", &nokia);
        assert!(p.body.contains("{"));
    }
}
