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
  duration_ms: number;
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
}

export interface AuditRecord {
  id: number;
  timestamp: string;
  intent: string;
  agent_logs: string;
  final_decision: string;
  human_approver: string | null;
  execution_status: string;
  risk: string;
  ai_duration_ms: number;
}

export interface ExecResult {
  target_id: string;
  vendor: string;
  command: string;
  output: string;
  simulated: boolean;
  success: boolean;
}
