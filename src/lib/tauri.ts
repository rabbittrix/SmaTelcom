import { invoke } from "@tauri-apps/api/core";
import type { HealthSnapshot, LintResult, PipelineResult } from "./types";

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

export async function approveAction(actionId: string): Promise<string> {
  return invoke<string>("approve_action", { actionId });
}

export async function rejectAction(actionId: string): Promise<string> {
  return invoke<string>("reject_action", { actionId });
}

export async function lintCommand(command: string): Promise<LintResult> {
  return invoke<LintResult>("lint_command", { command });
}

export async function reloadKnowledgeBase(): Promise<number> {
  return invoke<number>("reload_knowledge_base");
}
