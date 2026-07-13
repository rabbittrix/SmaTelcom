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
}

export interface ActivityEntry {
  id: string;
  ts: string;
  level: "info" | "agent" | "judge" | "safety" | "hitl" | "ok" | "error";
  message: string;
}
