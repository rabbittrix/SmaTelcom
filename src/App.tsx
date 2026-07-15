import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Loader2, Play, Search, Shield, Sparkles } from "lucide-react";
import { Sidebar, type NavId } from "./components/layout/Sidebar";
import { NetworkHealth } from "./components/dashboard/NetworkHealth";
import { ActivityLog } from "./components/dashboard/ActivityLog";
import { ReasoningSidebar } from "./components/dashboard/ReasoningSidebar";
import { RoiCalculator } from "./components/dashboard/RoiCalculator";
import { TopologyMap } from "./components/TopologyMap";
import { CriticalAlert } from "./components/hitl/CriticalAlert";
import { EvidencePanel } from "./components/dashboard/EvidencePanel";
import { ImpactReportView } from "./components/dashboard/ImpactReport";
import { NorthboundDrivers } from "./components/dashboard/NorthboundDrivers";
import { AuditTrailView } from "./components/dashboard/AuditTrailView";
import { LiveConsole, ValidationBadge } from "./components/dashboard/LiveConsole";
import { CommandPalette } from "./components/layout/CommandPalette";
import {
  CommandInput,
  type CommandInputHandle,
} from "./components/cli/CommandInput";
import { ThemeProvider } from "./hooks/useTheme";
import {
  analyzeIntent,
  approveAction,
  checkOllama,
  executeApprovedAction,
  getAuditHistory,
  getImpactReport,
  getRoi,
  getTelemetry,
  lintCommand,
  onConsoleFeed,
  onExecutionComplete,
  onPipelineProgress,
  onTelemetryTick,
  receiveExternalIntent,
  rejectAction,
  reloadKnowledgeBase,
  searchKnowledge,
  simulateVendorExec,
  translateIntent,
  translateVendorPayloads,
} from "./lib/tauri";
import type {
  ActivityEntry,
  AuditLogEntry,
  ConsoleLine,
  DocumentChunk,
  ExecResult,
  HealthSnapshot,
  ImpactReport,
  LintResult,
  PipelineResult,
  RoiSnapshot,
  TranslatedCommand,
  VendorPayload,
  VerificationResult,
} from "./lib/types";

function nowTs() {
  return new Date().toLocaleTimeString();
}

function toActivityLevel(level: string): ActivityEntry["level"] {
  switch (level) {
    case "agent":
    case "judge":
    case "safety":
    case "hitl":
    case "ok":
    case "error":
      return level;
    default:
      return "info";
  }
}

function AppInner() {
  const [nav, setNav] = useState<NavId>("dashboard");
  const [health, setHealth] = useState<HealthSnapshot | null>(null);
  const [autonomyGlow, setAutonomyGlow] = useState(false);
  const [ollamaOk, setOllamaOk] = useState<boolean | null>(null);
  const [intent, setIntent] = useState(
    "Reduce congestion on RAN-North while preserving SLA for enterprise slice",
  );
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<PipelineResult | null>(null);
  const [hitl, setHitl] = useState<PipelineResult | null>(null);
  const [activity, setActivity] = useState<ActivityEntry[]>([]);
  const [lintDemo, setLintDemo] = useState("optimize threshold for edge cell throughput");
  const [lintResult, setLintResult] = useState<LintResult | null>(null);
  const [kbCount, setKbCount] = useState<number | null>(null);
  const [kbQuery, setKbQuery] = useState("RAN congestion QoS");
  const [kbHits, setKbHits] = useState<DocumentChunk[]>([]);
  const [kbBusy, setKbBusy] = useState(false);
  const [roi, setRoi] = useState<RoiSnapshot | null>(null);
  const [vendorMaps, setVendorMaps] = useState<TranslatedCommand[]>([]);
  const [vendorExec, setVendorExec] = useState<ExecResult[]>([]);
  const [vendorBusy, setVendorBusy] = useState(false);
  const [vendorError, setVendorError] = useState<string | null>(null);
  const [audit, setAudit] = useState<AuditLogEntry[]>([]);
  const [impact, setImpact] = useState<ImpactReport | null>(null);
  const [consoleLines, setConsoleLines] = useState<ConsoleLine[]>([]);
  const [validation, setValidation] = useState<VerificationResult | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [vendorPayloads, setVendorPayloads] = useState<VendorPayload[]>([]);
  const [execBusy, setExecBusy] = useState(false);
  const commandInputRef = useRef<CommandInputHandle>(null);

  const pushLog = useCallback((level: ActivityEntry["level"], message: string) => {
    setActivity((prev) =>
      [{ id: crypto.randomUUID(), ts: nowTs(), level, message }, ...prev].slice(0, 80),
    );
  }, []);

  const refreshRoi = useCallback(() => {
    getRoi()
      .then(setRoi)
      .catch(() => undefined);
  }, []);

  const refreshImpact = useCallback(() => {
    getImpactReport()
      .then(setImpact)
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let alive = true;

    getTelemetry()
      .then((snap) => {
        if (alive) setHealth(snap);
      })
      .catch(() => undefined);

    onTelemetryTick((snap) => {
      if (alive) setHealth(snap);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // Vite-only fallback: poll if events unavailable.
        const id = setInterval(() => {
          getTelemetry()
            .then((snap) => {
              if (alive) setHealth(snap);
            })
            .catch(() => undefined);
        }, 5000);
        unlisten = () => clearInterval(id);
      });

    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onPipelineProgress((progress) => {
      pushLog(toActivityLevel(progress.level), progress.message);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, [pushLog]);

  useEffect(() => {
    checkOllama()
      .then(setOllamaOk)
      .catch(() => setOllamaOk(false));
    reloadKnowledgeBase()
      .then(setKbCount)
      .catch(() => setKbCount(0));
    refreshRoi();
    refreshImpact();
  }, [refreshRoi, refreshImpact]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onConsoleFeed((line) => {
      setConsoleLines((prev) => [...prev, line].slice(-200));
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onExecutionComplete((report) => {
      setValidation(report.verification);
      pushLog(
        report.verification.status === "verified" ? "ok" : "hitl",
        `Validation: ${report.verification.message}`,
      );
      if (report.verification.follow_up) {
        pushLog("hitl", report.verification.follow_up);
      }
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, [pushLog]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "k") return;
      e.preventDefault();
      if (e.shiftKey) {
        setPaletteOpen(true);
      } else {
        setNav((n) => (n === "console" ? n : "dashboard"));
        window.setTimeout(() => commandInputRef.current?.focus(), 50);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (nav === "audit" || nav === "impact") {
      getAuditHistory(100).then(setAudit).catch(() => undefined);
      refreshImpact();
    }
  }, [nav, refreshImpact]);

  const runLivePush = async (actionId?: string, command?: string) => {
    const id = actionId ?? result?.id;
    const cmd = command ?? result?.proposed_command;
    if (!id || !cmd) return;
    setExecBusy(true);
    setConsoleLines([]);
    setValidation(null);
    setNav("console");
    try {
      const report = await executeApprovedAction({
        actionId: id,
        command: cmd,
      });
      setValidation(report.verification);
      pushLog("ok", `Live push complete · ${report.device.hostname} (${report.device.vendor})`);
      getTelemetry().then(setHealth).catch(() => undefined);
    } catch (e) {
      pushLog("error", String(e));
    } finally {
      setExecBusy(false);
    }
  };

  const loadVendorTranslations = async () => {
    const cmd = result?.proposed_command || intent;
    if (!cmd.trim()) return;
    try {
      const payloads = await translateVendorPayloads(cmd);
      setVendorPayloads(payloads);
      setNav("drivers");
      pushLog("info", `Universal Translator → ${payloads.length} vendor payload(s)`);
    } catch (e) {
      pushLog("error", String(e));
    }
  };

  const handlePipelineResult = useCallback(
    async (res: PipelineResult) => {
      pushLog("judge", res.judge_summary);
      pushLog("info", `Pipeline completed in ${res.duration_ms} ms`);
      setResult(res);
      refreshRoi();
      refreshImpact();
      getAuditHistory(20).then(setAudit).catch(() => undefined);

      if (!res.lint.allowed || res.status === "blocked_by_safety") {
        pushLog("error", `BLOCKED — ${res.proposed_command}`);
        setNav("pipeline");
      } else if (
        res.lint.requires_hitl ||
        res.status === "pending_hitl" ||
        res.risk === "medium" ||
        res.risk === "high" ||
        res.risk === "critical"
      ) {
        pushLog("hitl", `HITL required (risk=${res.risk}) — awaiting human approval`);
        setHitl(res);
        setNav("pipeline");
      } else if (res.lint.auto_approvable || res.status === "auto_approved") {
        pushLog("ok", `Auto-approved (graduated autonomy): ${res.proposed_command}`);
        setAutonomyGlow(true);
        window.setTimeout(() => setAutonomyGlow(false), 1600);
        getTelemetry()
          .then(setHealth)
          .catch(() => undefined);
        setNav("console");
        await runLivePush(res.id, res.proposed_command);
      } else {
        pushLog("hitl", "Action requires human approval");
        setHitl(res);
        setNav("pipeline");
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [pushLog, refreshRoi, refreshImpact],
  );

  const runPipeline = async (submitted?: string) => {
    const text = (submitted ?? intent).trim();
    if (!text) return;
    setRunning(true);
    setResult(null);
    setIntent(""); // clear immediately after send
    pushLog("info", `Intent received: "${text}"`);
    try {
      const res = await analyzeIntent(text);
      await handlePipelineResult(res);
    } catch (e) {
      pushLog("error", String(e));
      setIntent(text); // restore on failure
    } finally {
      setRunning(false);
      window.setTimeout(() => commandInputRef.current?.focus(), 80);
    }
  };

  const injectTmfOrder = async () => {
    setRunning(true);
    setNav("console");
    setConsoleLines([]);
    pushLog("info", "TMF641 northbound ingress — injecting ORDER-123…");
    try {
      const nb = await receiveExternalIntent(null);
      pushLog(
        "ok",
        `TMF641 ${nb.ingress.order.id} · TMF637 matched ${nb.ingress.hydration.matched_devices.length} NE(s)`,
      );
      setIntent("");
      await handlePipelineResult(nb.pipeline);
    } catch (e) {
      pushLog("error", String(e));
    } finally {
      setRunning(false);
    }
  };

  const onApprove = async () => {
    if (!hitl) return;
    try {
      const outcome = await approveAction(hitl.id, { protocol: "netconf", dryRun: false });
      pushLog("ok", outcome.message);
      if (outcome.driver) {
        pushLog("info", `Northbound ${outcome.driver.status}: ${outcome.driver.message}`);
      }
      const approved = hitl;
      setHitl(null);
      getAuditHistory(20).then(setAudit).catch(() => undefined);
      refreshImpact();
      // Stream Live Execution Log after HITL approval
      void runLivePush(approved.id, approved.proposed_command);
    } catch (e) {
      pushLog("error", String(e));
    }
  };

  const onReject = async () => {
    if (!hitl) return;
    try {
      const outcome = await rejectAction(hitl.id);
      pushLog("error", outcome.message);
      setHitl(null);
      getAuditHistory(20).then(setAudit).catch(() => undefined);
      refreshImpact();
    } catch (e) {
      pushLog("error", String(e));
    }
  };

  const runLint = async () => {
    try {
      const r = await lintCommand(lintDemo);
      setLintResult(r);
      pushLog("safety", r.reason);
      refreshRoi();
    } catch (e) {
      pushLog("error", String(e));
    }
  };

  const runKbSearch = async () => {
    setKbBusy(true);
    try {
      const hits = await searchKnowledge(kbQuery, 8);
      setKbHits(hits);
      pushLog("info", `KB search “${kbQuery}” → ${hits.length} chunk(s)`);
    } catch (e) {
      pushLog("error", String(e));
    } finally {
      setKbBusy(false);
    }
  };

  const runVendorPreview = async () => {
    const text = intent.trim();
    if (!text) {
      setVendorError("Enter a network intent first.");
      setNav("vendors");
      return;
    }
    setVendorBusy(true);
    setVendorError(null);
    setNav("vendors");
    try {
      const maps = await translateIntent(text);
      setVendorMaps(maps);
      try {
        const execs = await simulateVendorExec(text);
        setVendorExec(execs);
      } catch (execErr) {
        setVendorExec([]);
        pushLog("error", `SSH sim: ${String(execErr)}`);
      }
      pushLog("info", `Translated intent to ${maps.length} vendor CLI dialects (SSH sim)`);
    } catch (e) {
      setVendorMaps([]);
      setVendorExec([]);
      setVendorError(String(e));
      pushLog("error", String(e));
    } finally {
      setVendorBusy(false);
    }
  };

  return (
    <div className="flex h-full">
      <Sidebar active={nav} onNavigate={setNav} ollamaOk={ollamaOk} />

      <main className="relative flex min-w-0 flex-1 flex-col">
        <header
          className="flex items-center justify-between border-b px-6 py-4"
          style={{ borderColor: "var(--border)" }}
        >
          <div>
            <h1 className="text-lg font-semibold tracking-tight">
              {nav === "dashboard" && "Operations Dashboard"}
              {nav === "pipeline" && "Multi-Agent Intent Pipeline"}
              {nav === "topology" && "Network Topology Engine"}
              {nav === "vendors" && "Multi-Vendor SSH Adapters"}
              {nav === "drivers" && "Northbound Drivers (NETCONF / gNMI)"}
              {nav === "console" && "Live Console — Closed-Loop Push"}
              {nav === "evidence" && "Policy Evidence — Show Your Source"}
              {nav === "audit" && "Persistent Audit Trail"}
              {nav === "impact" && "Autonomy Impact Report"}
              {nav === "safety" && "Deterministic Safety Linter"}
              {nav === "knowledge" && "Local Knowledge Base (RAG)"}
              {nav === "activity" && "Agent Activity Stream"}
            </h1>
            <p className="text-xs" style={{ color: "var(--text-muted)" }}>
              Localhost-only · Ollama :11434 · Rust guardrails · SQLite audit trail
            </p>
          </div>
          <div
            className="hidden items-center gap-2 rounded-full border px-3 py-1 text-xs sm:flex"
            style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
          >
            <Sparkles className="h-3.5 w-3.5" style={{ color: "var(--accent)" }} />
            Phi-3 / Mistral via Ollama
            <span style={{ color: "var(--text-muted)" }}>· Ctrl+K CLI · Ctrl+Shift+K audit</span>
          </div>
        </header>

        <div className="flex-1 overflow-y-auto p-6 pb-36">
          <AnimatePresence mode="wait">
            <motion.div
              key={nav}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.22 }}
              className="mx-auto max-w-7xl space-y-6"
            >
              {(nav === "dashboard" || nav === "pipeline") && (
                <>
                  <NetworkHealth health={health} autonomyGlow={autonomyGlow} />

                  <div className="flex flex-col gap-4 lg:flex-row">
                    <div className="min-w-0 flex-1 space-y-4">
                      <div
                        className="relative z-10 rounded-xl border p-4"
                        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                      >
                        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                          <label
                            className="block text-xs font-semibold uppercase tracking-wider"
                            style={{ color: "var(--text-muted)" }}
                          >
                            Network Intent CLI
                          </label>
                          <button
                            type="button"
                            onClick={injectTmfOrder}
                            disabled={running}
                            className="rounded-md border px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider disabled:opacity-50"
                            style={{ borderColor: "var(--border)", color: "var(--accent)" }}
                          >
                            Inject TMF641 Order
                          </button>
                        </div>
                        <CommandInput
                          ref={commandInputRef}
                          value={intent}
                          onChange={setIntent}
                          onSubmit={runPipeline}
                          loading={running}
                          mode="textarea"
                          rows={3}
                          autoFocus
                          placeholder="e.g. Ensure low latency for Dubai Mall…"
                        />
                        <div className="mt-3 flex flex-wrap justify-end gap-2">
                          <button
                            onClick={runVendorPreview}
                            className="rounded-lg border px-3 py-2 text-sm"
                            style={{ borderColor: "var(--border)" }}
                          >
                            Preview Vendor CLI
                          </button>
                          <button
                            onClick={loadVendorTranslations}
                            className="rounded-lg border px-3 py-2 text-sm"
                            style={{ borderColor: "var(--border)" }}
                          >
                            Universal Translator
                          </button>
                          <button
                            onClick={() => void runLivePush()}
                            disabled={execBusy || !result}
                            className="rounded-lg border px-3 py-2 text-sm disabled:opacity-50"
                            style={{ borderColor: "var(--border)" }}
                          >
                            {execBusy ? "Pushing…" : "Run Live Push"}
                          </button>
                          <button
                            onClick={() => void runPipeline()}
                            disabled={running || !intent.trim()}
                            className="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
                            style={{ background: "#0d9488" }}
                          >
                            {running ? (
                              <Loader2 className="h-4 w-4 animate-spin" />
                            ) : (
                              <Play className="h-4 w-4" />
                            )}
                            Run Multi-Agent Pipeline
                          </button>
                        </div>
                        {validation && (
                          <div className="mt-3">
                            <ValidationBadge status={validation.status} />
                          </div>
                        )}
                      </div>

                      {(nav === "dashboard" || consoleLines.length > 0) && (
                        <div className="relative z-10">
                          <div className="mb-2 flex items-center justify-between">
                            <h3 className="text-sm font-semibold">Live Execution Log</h3>
                            <ValidationBadge status={validation?.status ?? null} />
                          </div>
                          <LiveConsole
                            lines={consoleLines}
                            loading={running || execBusy}
                            showInput={false}
                          />
                        </div>
                      )}

                      {nav === "dashboard" && <TopologyMap health={health} />}

                      {result?.predicted_impact && (
                        <div
                          className="rounded-xl border p-4 text-sm"
                          style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                        >
                          <h4 className="mb-1 text-sm font-semibold">Digital Twin — Predicted Impact</h4>
                          <p className="mb-2 text-xs" style={{ color: "var(--text-muted)" }}>
                            {result.predicted_impact.summary}
                          </p>
                          <div className="grid grid-cols-2 gap-2 font-mono text-xs md:grid-cols-4">
                            <span>
                              CPU {result.predicted_impact.cpu_pct_before.toFixed(0)}→
                              {result.predicted_impact.cpu_pct_after.toFixed(0)}%
                            </span>
                            <span>
                              Lat {result.predicted_impact.latency_ms_before.toFixed(1)}→
                              {result.predicted_impact.latency_ms_after.toFixed(1)}ms
                            </span>
                            <span>
                              TP {result.predicted_impact.throughput_gbps_before.toFixed(1)}→
                              {result.predicted_impact.throughput_gbps_after.toFixed(1)}G
                            </span>
                            <span>Blast: {result.predicted_impact.blast_radius}</span>
                          </div>
                        </div>
                      )}

                      <div className="grid gap-4 lg:grid-cols-2">
                        <ActivityLog entries={activity} />
                        <EvidencePanel result={result} />
                      </div>

                      <div
                          className="rounded-xl border p-4"
                          style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                        >
                          <h3 className="mb-2 text-sm font-semibold">Audit Trail (SQLite)</h3>
                          <div className="max-h-[280px] space-y-2 overflow-y-auto font-mono text-[10px]">
                            {audit.length === 0 && (
                              <p style={{ color: "var(--text-muted)" }}>No audit rows yet.</p>
                            )}
                            {audit.map((a) => (
                              <div
                                key={a.id}
                                className="border-b py-1"
                                style={{ borderColor: "var(--border)" }}
                              >
                                <div>
                                  {a.id.slice(0, 8)} · {a.decision}
                                </div>
                                <div className="truncate" style={{ color: "var(--text-muted)" }}>
                                  {a.final_command || a.intent}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                    </div>

                    <ReasoningSidebar result={result} running={running} />
                  </div>
                </>
              )}

              {nav === "topology" && <TopologyMap health={health} />}

              {nav === "vendors" && (
                <div className="space-y-4">
                  <div
                    className="rounded-xl border p-4"
                    style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                  >
                    <h2 className="mb-2 text-lg font-semibold">Command Translator + SSH Simulation</h2>
                    <p className="mb-3 text-sm" style={{ color: "var(--text-muted)" }}>
                      Maps generic intents to Cisco IOS / Huawei VRP via local RAG. Safety Linter gates
                      every CLI before SSH sim.
                    </p>
                    <label
                      className="mb-2 block text-xs font-semibold uppercase tracking-wider"
                      style={{ color: "var(--text-muted)" }}
                    >
                      Intent to translate
                    </label>
                    <textarea
                      value={intent}
                      onChange={(e) => setIntent(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" && !e.shiftKey) {
                          e.preventDefault();
                          void runVendorPreview();
                        }
                      }}
                      rows={3}
                      className="relative z-10 mb-3 w-full resize-none rounded-lg border bg-transparent px-3 py-2 text-sm outline-none"
                      style={{ borderColor: "var(--border)", color: "var(--text)" }}
                      placeholder="e.g. Interface Down on edge PE"
                    />
                    <button
                      onClick={runVendorPreview}
                      disabled={vendorBusy || !intent.trim()}
                      className="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
                      style={{ background: "#0d9488" }}
                    >
                      {vendorBusy ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
                      {vendorBusy ? "Translating…" : "Translate Current Intent"}
                    </button>
                    {vendorError && (
                      <p className="mt-3 text-sm text-red-500">{vendorError}</p>
                    )}
                  </div>
                  <div className="grid gap-4 lg:grid-cols-2">
                    {vendorMaps.map((v) => (
                      <pre
                        key={v.vendor}
                        className="overflow-auto rounded-xl border p-3 font-mono text-xs"
                        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                      >
                        <div className="mb-2 font-sans text-xs font-semibold" style={{ color: "var(--accent)" }}>
                          {v.vendor} · conf {(v.confidence * 100).toFixed(0)}%
                        </div>
                        {v.cli}
                      </pre>
                    ))}
                  </div>
                  {!vendorBusy && vendorMaps.length === 0 && !vendorError && (
                    <p className="text-sm" style={{ color: "var(--text-muted)" }}>
                      No translations yet. Click Translate Current Intent.
                    </p>
                  )}
                  {vendorExec.map((e) => (
                    <pre
                      key={e.target_id}
                      className="overflow-auto rounded-xl border p-3 font-mono text-[11px]"
                      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                    >
                      <div className="mb-1 font-sans text-xs" style={{ color: "var(--text-muted)" }}>
                        {e.target_id} · simulated={String(e.simulated)}
                      </div>
                      {e.output}
                    </pre>
                  ))}
                </div>
              )}

              {nav === "safety" && (
                <div
                  className="rounded-xl border p-5"
                  style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                >
                  <div className="mb-4 flex items-center gap-2">
                    <Shield className="h-5 w-5" style={{ color: "var(--accent)" }} />
                    <h2 className="text-lg font-semibold">Rust Safety Linter</h2>
                  </div>
                  <textarea
                    value={lintDemo}
                    onChange={(e) => setLintDemo(e.target.value)}
                    rows={3}
                    className="mb-3 w-full rounded-lg border bg-transparent px-3 py-2 font-mono text-sm"
                    style={{ borderColor: "var(--border)" }}
                  />
                  <button
                    onClick={runLint}
                    className="rounded-lg px-4 py-2 text-sm font-medium text-white"
                    style={{ background: "#0d9488" }}
                  >
                    Lint Command
                  </button>
                  {lintResult && (
                    <pre
                      className="mt-4 overflow-auto rounded-lg border p-3 font-mono text-xs"
                      style={{ borderColor: "var(--border)" }}
                    >
                      {JSON.stringify(lintResult, null, 2)}
                    </pre>
                  )}
                </div>
              )}

              {nav === "knowledge" && (
                <div
                  className="space-y-4 rounded-xl border p-5"
                  style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                >
                  <div>
                    <h2 className="mb-2 text-lg font-semibold">Knowledge Base (RAG)</h2>
                    <p className="mb-4 text-sm" style={{ color: "var(--text-muted)" }}>
                      Chunks loaded: {kbCount ?? "—"} · local keyword retrieval (no cloud)
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button
                      onClick={async () => {
                        const n = await reloadKnowledgeBase();
                        setKbCount(n);
                        pushLog("info", `Reloaded knowledge base · ${n} chunks`);
                      }}
                      className="rounded-lg border px-4 py-2 text-sm"
                      style={{ borderColor: "var(--border)" }}
                    >
                      Reload Knowledge Base
                    </button>
                  </div>
                  <div className="flex gap-2">
                    <input
                      value={kbQuery}
                      onChange={(e) => setKbQuery(e.target.value)}
                      className="min-w-0 flex-1 rounded-lg border bg-transparent px-3 py-2 text-sm outline-none"
                      style={{ borderColor: "var(--border)" }}
                      placeholder="Search manuals…"
                    />
                    <button
                      onClick={runKbSearch}
                      disabled={kbBusy || !kbQuery.trim()}
                      className="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
                      style={{ background: "#0d9488" }}
                    >
                      {kbBusy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Search className="h-4 w-4" />}
                      Search
                    </button>
                  </div>
                  <div className="space-y-3">
                    {kbHits.length === 0 && (
                      <p className="text-sm" style={{ color: "var(--text-muted)" }}>
                        No results yet. Search the local knowledge_base/.
                      </p>
                    )}
                    {kbHits.map((hit, i) => (
                      <div
                        key={`${hit.source}-${i}`}
                        className="rounded-lg border p-3"
                        style={{ borderColor: "var(--border)" }}
                      >
                        <div className="mb-1 text-xs font-semibold" style={{ color: "var(--accent)" }}>
                          {hit.source}
                        </div>
                        <p className="text-xs leading-relaxed" style={{ color: "var(--text-muted)" }}>
                          {hit.content.slice(0, 420)}
                          {hit.content.length > 420 ? "…" : ""}
                        </p>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {nav === "drivers" && (
                <div className="space-y-4">
                  <NorthboundDrivers
                    command={result?.proposed_command ?? ""}
                    actionId={result?.id ?? hitl?.id}
                  />
                  {vendorPayloads.length > 0 && (
                    <div className="grid gap-3 lg:grid-cols-3">
                      {vendorPayloads.map((p) => (
                        <pre
                          key={`${p.vendor}-${p.device_id}`}
                          className="overflow-auto rounded-xl border p-3 font-mono text-[10px]"
                          style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                        >
                          <div className="mb-2 font-sans text-xs font-semibold" style={{ color: "var(--accent)" }}>
                            {p.summary} · {p.format}
                          </div>
                          {p.body}
                        </pre>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {nav === "console" && (
                <div className="space-y-4">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <ValidationBadge status={validation?.status ?? null} />
                    <div className="flex gap-2">
                      <button
                        onClick={injectTmfOrder}
                        disabled={running}
                        className="rounded-lg border px-3 py-2 text-sm disabled:opacity-50"
                        style={{ borderColor: "var(--border)" }}
                      >
                        Inject TMF641
                      </button>
                      <button
                        onClick={() => void runLivePush()}
                        disabled={execBusy || !result}
                        className="rounded-lg px-3 py-2 text-sm font-medium text-white disabled:opacity-50"
                        style={{ background: "#0d9488" }}
                      >
                        {execBusy ? "Executing…" : "Run Live Push + Verify"}
                      </button>
                    </div>
                  </div>
                  {validation?.follow_up && (
                    <p className="text-sm" style={{ color: "#f59e0b" }}>
                      Self-healing: {validation.follow_up}
                    </p>
                  )}
                  <LiveConsole
                    lines={consoleLines}
                    commandValue={intent}
                    onCommandChange={setIntent}
                    onCommandSubmit={runPipeline}
                    loading={running || execBusy}
                    inputRef={commandInputRef}
                    showInput
                  />
                </div>
              )}

              {nav === "evidence" && <EvidencePanel result={result} />}

              {nav === "audit" && <AuditTrailView entries={audit} />}

              {nav === "impact" && <ImpactReportView report={impact} />}

              {nav === "activity" && <ActivityLog entries={activity} />}
            </motion.div>
          </AnimatePresence>
        </div>

        <RoiCalculator roi={roi} />
      </main>

      {hitl && <CriticalAlert result={hitl} onApprove={onApprove} onReject={onReject} />}
      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        onSelect={(entry) => {
          setAudit((prev) => [entry, ...prev.filter((a) => a.id !== entry.id)]);
          setNav("audit");
        }}
      />
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <AppInner />
    </ThemeProvider>
  );
}
