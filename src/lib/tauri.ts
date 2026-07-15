import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AuditRecord,
  ConsoleLine,
  DeviceTarget,
  DocumentChunk,
  DriverResult,
  ExecResult,
  ExecutionReport,
  HealthSnapshot,
  HitlOutcome,
  ImpactReport,
  InventoryDevice,
  LintResult,
  NorthboundIngressResult,
  PipelineProgress,
  PipelineResult,
  RoiSnapshot,
  ServiceOrder,
  TranslatedCommand,
  VendorPayload,
} from "./types";

export async function checkOllama(): Promise<boolean> {
  return invoke<boolean>("check_ollama");
}

export async function listModels(): Promise<string[]> {
  return invoke<string[]>("list_models");
}

export async function getTelemetry(): Promise<HealthSnapshot> {
  return invoke<HealthSnapshot>("get_telemetry_snapshot");
}

export async function analyzeIntent(
  intent: string,
  model?: string,
): Promise<PipelineResult> {
  return invoke<PipelineResult>("analyze_network_intent", {
    request: { intent, model: model ?? null },
  });
}

export async function approveAction(
  actionId: string,
  opts?: { protocol?: "netconf" | "gnmi"; dryRun?: boolean },
): Promise<HitlOutcome> {
  return invoke<HitlOutcome>("approve_action", {
    actionId,
    protocol: opts?.protocol ?? null,
    dryRun: opts?.dryRun ?? false,
  });
}

export async function rejectAction(actionId: string): Promise<HitlOutcome> {
  return invoke<HitlOutcome>("reject_action", { actionId });
}

export async function lintCommand(command: string): Promise<LintResult> {
  return invoke<LintResult>("lint_command", { command });
}

export async function reloadKnowledgeBase(): Promise<number> {
  return invoke<number>("reload_knowledge_base");
}

export async function searchKnowledge(
  query: string,
  topK = 5,
): Promise<DocumentChunk[]> {
  return invoke<DocumentChunk[]>("search_knowledge", { query, topK });
}

export async function getRoi(): Promise<RoiSnapshot> {
  return invoke<RoiSnapshot>("get_roi_snapshot");
}

export async function getImpactReport(): Promise<ImpactReport> {
  return invoke<ImpactReport>("get_impact_report");
}

export async function getAuditTrail(limit = 50): Promise<AuditRecord[]> {
  return invoke<AuditRecord[]>("get_audit_history", { limit });
}

export async function getAuditHistory(limit = 100): Promise<AuditRecord[]> {
  return invoke<AuditRecord[]>("get_audit_history", { limit });
}

export async function translateIntent(intent: string): Promise<TranslatedCommand[]> {
  return invoke<TranslatedCommand[]>("translate_intent", { intent });
}

export async function simulateVendorExec(intent: string): Promise<ExecResult[]> {
  return invoke<ExecResult[]>("simulate_vendor_exec", { intent });
}

export async function listLabDevices(): Promise<DeviceTarget[]> {
  return invoke<DeviceTarget[]>("list_lab_devices");
}

export async function sshExecLab(
  targetId: string,
  command: string,
): Promise<ExecResult> {
  return invoke<ExecResult>("ssh_exec_lab", { targetId, command });
}

export async function northboundDryRun(args: {
  command: string;
  protocol?: "netconf" | "gnmi";
  target?: string;
  actionId?: string;
}): Promise<DriverResult> {
  return invoke<DriverResult>("northbound_dry_run", {
    request: {
      command: args.command,
      protocol: args.protocol ?? "netconf",
      target: args.target ?? "lab-ne-01",
      dryRun: true,
      actionId: args.actionId ?? null,
    },
  });
}

export async function northboundCommit(args: {
  command: string;
  protocol?: "netconf" | "gnmi";
  target?: string;
  dryRun?: boolean;
  actionId?: string;
}): Promise<DriverResult> {
  return invoke<DriverResult>("northbound_commit", {
    request: {
      command: args.command,
      protocol: args.protocol ?? "netconf",
      target: args.target ?? "lab-ne-01",
      dryRun: args.dryRun ?? false,
      actionId: args.actionId ?? null,
    },
  });
}

export async function onTelemetryTick(
  handler: (snap: HealthSnapshot) => void,
): Promise<UnlistenFn> {
  return listen<HealthSnapshot>("telemetry-tick", (event) => handler(event.payload));
}

export async function onPipelineProgress(
  handler: (progress: PipelineProgress) => void,
): Promise<UnlistenFn> {
  return listen<PipelineProgress>("pipeline-progress", (event) =>
    handler(event.payload),
  );
}

export async function onAutonomySavings(
  handler: (count: number) => void,
): Promise<UnlistenFn> {
  return listen<number>("autonomy-savings", (event) => handler(event.payload));
}

export async function listInventory(): Promise<InventoryDevice[]> {
  return invoke<InventoryDevice[]>("list_inventory");
}

export async function translateVendorPayloads(
  command: string,
): Promise<VendorPayload[]> {
  return invoke<VendorPayload[]>("translate_vendor_payloads", { command });
}

export async function executeApprovedAction(args: {
  actionId: string;
  command: string;
  deviceId?: string;
}): Promise<ExecutionReport> {
  return invoke<ExecutionReport>("execute_approved_action", {
    actionId: args.actionId,
    command: args.command,
    deviceId: args.deviceId ?? null,
  });
}

export async function searchAuditLogs(
  query: string,
  limit = 40,
): Promise<AuditRecord[]> {
  return invoke<AuditRecord[]>("search_audit_logs", { query, limit });
}

export async function onConsoleFeed(
  handler: (line: ConsoleLine) => void,
): Promise<UnlistenFn> {
  return listen<ConsoleLine>("console-feed", (event) => handler(event.payload));
}

export async function onExecutionComplete(
  handler: (report: ExecutionReport) => void,
): Promise<UnlistenFn> {
  return listen<ExecutionReport>("execution-complete", (event) =>
    handler(event.payload),
  );
}

/** TMF641 northbound ingress — optional order; defaults to demo ORDER-123. */
export async function receiveExternalIntent(
  order?: ServiceOrder | null,
  model?: string,
): Promise<NorthboundIngressResult> {
  return invoke<NorthboundIngressResult>("receive_external_intent", {
    order: order ?? null,
    model: model ?? null,
  });
}

export async function getDemoServiceOrder(): Promise<ServiceOrder> {
  return invoke<ServiceOrder>("get_demo_service_order");
}
