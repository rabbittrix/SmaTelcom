export type RiskLevel = "low" | "medium" | "high" | "critical";

export interface TelemetryEvent {
  id: string;
  timestamp: string;
  site: string;
  element: string;
  metric: string;
  value: number;
  unit: string;
  severity: string;
  message: string;
}

export interface TopologyNode {
  id: string;
  label: string;
  site: string;
  role: string;
  x: number;
  y: number;
  status: string;
  cpu_pct: number;
  vendor: string;
}

export interface TopologyLink {
  source: string;
  target: string;
  status: string;
}

export interface HealthSnapshot {
  overall_score: number;
  latency_ms: number;
  packet_loss_pct: number;
  throughput_gbps: number;
  active_alarms: number;
  sites_online: number;
  sites_total: number;
  autonomy_savings: number;
  last_event: TelemetryEvent | null;
  recent_events: TelemetryEvent[];
  nodes: TopologyNode[];
  links: TopologyLink[];
}

export interface AgentOpinion {
  agent: string;
  analysis: string;
  recommendation: string;
  confidence: number;
}

export interface LintResult {
  allowed: boolean;
  risk: RiskLevel;
  matched_rules: string[];
  reason: string;
  requires_hitl: boolean;
  auto_approvable: boolean;
}

export interface PredictedImpact {
  cpu_pct_before: number;
  cpu_pct_after: number;
  latency_ms_before: number;
  latency_ms_after: number;
  throughput_gbps_before: number;
  throughput_gbps_after: number;
  packet_loss_pct_before: number;
  packet_loss_pct_after: number;
  blast_radius: string;
  summary: string;
}

export interface TranslatedCommand {
  intent: string;
  vendor: "cisco_ios" | "huawei_vrp" | "generic";
  cli: string;
  rag_sources: string[];
  confidence: number;
}

export interface ConflictResolution {
  conflict_detected: boolean;
  winner: string;
  loser: string;
  priority_applied: string;
  policy_citation: string;
  rationale: string;
}

export interface DocumentChunk {
  source: string;
  content: string;
}

export interface PipelineResult {
  id: string;
  intent: string;
  opinions: AgentOpinion[];
  judge_summary: string;
  proposed_command: string;
  decision_logic: string;
  risk: RiskLevel;
  lint: LintResult;
  status: string;
  knowledge_used: boolean;
  predicted_impact: PredictedImpact;
  vendor_commands: TranslatedCommand[];
  conflict_resolution: ConflictResolution;
  evidence_sources: DocumentChunk[];
  duration_ms: number;
}

export interface PipelineProgress {
  stage: string;
  level: string;
  message: string;
}

export interface DriverPayload {
  id: string;
  protocol: "netconf" | "gnmi";
  target: string;
  content_type: string;
  body: string;
  dry_run: boolean;
  created_at: string;
}

export interface DriverResult {
  payload: DriverPayload;
  status: string;
  message: string;
  commit_id: string | null;
  simulated: boolean;
}

export interface HitlOutcome {
  action_id: string;
  decision: string;
  message: string;
  command: string | null;
  lint: LintResult | null;
  driver: DriverResult | null;
}

export interface ActivityEntry {
  id: string;
  ts: string;
  level: "info" | "agent" | "judge" | "safety" | "hitl" | "ok" | "error";
  message: string;
}

export interface RoiSnapshot {
  engineering_hours_saved: number;
  risks_mitigated: number;
  intents_processed: number;
  total_ai_ms: number;
  human_baseline_minutes: number;
  auto_approved: number;
  critical_risks_averted: number;
}

export interface AuditLogEntry {
  id: string;
  timestamp: string;
  intent: string;
  final_command: string;
  risk_level: string;
  decision: string;
  conflict_resolution: string | null;
  payload_preview: string | null;
  agent_logs: string | null;
  policy_citation: string | null;
  ai_duration_ms: number;
}

/** @deprecated Use AuditLogEntry — kept for gradual migration */
export type AuditRecord = AuditLogEntry;

export interface ImpactReport {
  intents_processed: number;
  auto_approved: number;
  hitl_pending_or_resolved: number;
  blocked: number;
  human_hours_saved: number;
  critical_risks_averted: number;
  minutes_per_auto_approve: number;
}

export interface ExecResult {
  target_id: string;
  vendor: string;
  command: string;
  output: string;
  simulated: boolean;
  success: boolean;
}

export interface ConsoleLine {
  ts: string;
  level: string;
  message: string;
}

export interface InventoryDevice {
  id: string;
  hostname: string;
  vendor: "cisco_ios_xe" | "huawei_vrp" | "nokia_sros";
  site_class: string;
  role: string;
  mgmt_ip: string;
  protocol_hint: string;
}

export interface VendorPayload {
  vendor: "cisco_ios_xe" | "huawei_vrp" | "nokia_sros";
  device_id: string;
  format: string;
  body: string;
  summary: string;
}

export interface VerificationResult {
  status: "verified" | "drift_detected" | "pending" | "skipped";
  device_id: string;
  before_score: number;
  after_score: number;
  message: string;
  follow_up: string | null;
}

export interface ExecutionReport {
  execution_id: string;
  action_id: string;
  device: InventoryDevice;
  vendor_payload: VendorPayload;
  console: ConsoleLine[];
  success: boolean;
  verification: VerificationResult;
}

export interface DeviceTarget {
  id: string;
  hostname: string;
  vendor: string;
  host: string;
  port: number;
  simulate: boolean;
}

/** TMF641 Service Order (camelCase from OSS/BSS). */
export interface ServiceOrder {
  id: string;
  serviceType: string;
  priority: string;
  intent: string;
  relatedParty?: string | null;
  geographicSite?: string | null;
}

export interface InventoryHydration {
  order_id: string;
  matched_devices: InventoryDevice[];
  context_block: string;
  primary_target: InventoryDevice | null;
}

export interface ExternalIntentResult {
  order: ServiceOrder;
  hydration: InventoryHydration;
  enriched_intent: string;
}

export interface NorthboundIngressResult {
  ingress: ExternalIntentResult;
  pipeline: PipelineResult;
}
