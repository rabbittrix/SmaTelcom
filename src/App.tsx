import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Loader2, Play, Shield, Sparkles } from "lucide-react";
import { Sidebar, type NavId } from "./components/layout/Sidebar";
import { NetworkHealth } from "./components/dashboard/NetworkHealth";
import { ActivityLog } from "./components/dashboard/ActivityLog";
import { ReasoningSidebar } from "./components/dashboard/ReasoningSidebar";
import { RoiCalculator } from "./components/dashboard/RoiCalculator";
import { TopologyMap } from "./components/TopologyMap";
import { CriticalAlert } from "./components/hitl/CriticalAlert";
import { ThemeProvider } from "./hooks/useTheme";
import {
  analyzeIntent,
  approveAction,
  checkOllama,
  getAuditTrail,
  getRoi,
  getTelemetry,
  lintCommand,
  rejectAction,
  reloadKnowledgeBase,
  simulateVendorExec,
  translateIntent,
} from "./lib/tauri";
import type {
  ActivityEntry,
  AuditRecord,
  ExecResult,
  HealthSnapshot,
  LintResult,
  PipelineResult,
  RoiSnapshot,
  TranslatedCommand,
} from "./lib/types";

function nowTs() {
  return new Date().toLocaleTimeString();
}

function AppInner() {
  const [nav, setNav] = useState<NavId>("dashboard");
  const [health, setHealth] = useState<HealthSnapshot | null>(null);
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
  const [roi, setRoi] = useState<RoiSnapshot | null>(null);
  const [vendorMaps, setVendorMaps] = useState<TranslatedCommand[]>([]);
  const [vendorExec, setVendorExec] = useState<ExecResult[]>([]);
  const [audit, setAudit] = useState<AuditRecord[]>([]);

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

  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        const snap = await getTelemetry();
        if (alive) setHealth(snap);
      } catch {
        /* vite-only */
      }
    };
    tick();
    const id = setInterval(tick, 5000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  useEffect(() => {
    checkOllama()
      .then(setOllamaOk)
      .catch(() => setOllamaOk(false));
    reloadKnowledgeBase()
      .then(setKbCount)
      .catch(() => setKbCount(0));
    refreshRoi();
  }, [refreshRoi]);

  const runPipeline = async () => {
    setRunning(true);
    setResult(null);
    pushLog("info", `Intent received: "${intent}"`);
    pushLog("agent", "Performance Agent analyzing capacity & QoS…");
    pushLog("agent", "Security Agent assessing blast radius…");
    pushLog("agent", "Topology Agent evaluating path diversity…");
    try {
      const res = await analyzeIntent(intent);
      pushLog("judge", res.judge_summary);
      pushLog("safety", `Linter: ${res.lint.reason}`);
      pushLog("info", `Pipeline completed in ${res.duration_ms} ms`);
      setResult(res);
      refreshRoi();
      getAuditTrail(20).then(setAudit).catch(() => undefined);
      if (!res.lint.allowed) {
        pushLog("error", `BLOCKED — ${res.proposed_command}`);
      } else if (res.lint.auto_approvable || res.status === "auto_approved") {
        pushLog("ok", `Auto-approved (graduated autonomy): ${res.proposed_command}`);
      } else {
        pushLog("hitl", "Critical/complex action — awaiting human approval");
        setHitl(res);
      }
      setNav("pipeline");
    } catch (e) {
      pushLog("error", String(e));
    } finally {
      setRunning(false);
    }
  };

  const onApprove = async () => {
    if (!hitl) return;
    const msg = await approveAction(hitl.id);
    pushLog("ok", msg);
    setHitl(null);
    getAuditTrail(20).then(setAudit).catch(() => undefined);
  };

  const onReject = async () => {
    if (!hitl) return;
    const msg = await rejectAction(hitl.id);
    pushLog("error", msg);
    setHitl(null);
    getAuditTrail(20).then(setAudit).catch(() => undefined);
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

  const runVendorPreview = async () => {
    try {
      const [maps, execs] = await Promise.all([
        translateIntent(intent),
        simulateVendorExec(intent),
      ]);
      setVendorMaps(maps);
      setVendorExec(execs);
      pushLog("info", `Translated intent to ${maps.length} vendor CLI dialects (SSH sim)`);
    } catch (e) {
      pushLog("error", String(e));
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
                  <NetworkHealth health={health} />

                  <div className="flex flex-col gap-4 lg:flex-row">
                    <div className="min-w-0 flex-1 space-y-4">
                      <div
                        className="rounded-xl border p-4"
                        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                      >
                        <label
                          className="mb-2 block text-xs font-semibold uppercase tracking-wider"
                          style={{ color: "var(--text-muted)" }}
                        >
                          Network Intent
                        </label>
                        <textarea
                          value={intent}
                          onChange={(e) => setIntent(e.target.value)}
                          rows={3}
                          className="w-full resize-none rounded-lg border bg-transparent px-3 py-2 text-sm outline-none focus:ring-2"
                          style={{ borderColor: "var(--border)" }}
                        />
                        <div className="mt-3 flex justify-end gap-2">
                          <button
                            onClick={runVendorPreview}
                            className="rounded-lg border px-3 py-2 text-sm"
                            style={{ borderColor: "var(--border)" }}
                          >
                            Preview Vendor CLI
                          </button>
                          <button
                            onClick={runPipeline}
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
                      </div>

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
                                  #{a.id} {a.timestamp} · {a.execution_status}
                                </div>
                                <div className="truncate" style={{ color: "var(--text-muted)" }}>
                                  {a.intent}
                                </div>
                              </div>
                            ))}
                          </div>
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
                      Maps generic intents to Cisco IOS / Huawei VRP via local RAG, then runs simulated SSH sessions (`ssh2` real path available when simulate=false).
                    </p>
                    <button
                      onClick={runVendorPreview}
                      className="rounded-lg px-4 py-2 text-sm font-medium text-white"
                      style={{ background: "#0d9488" }}
                    >
                      Translate Current Intent
                    </button>
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
                  className="rounded-xl border p-5"
                  style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                >
                  <h2 className="mb-2 text-lg font-semibold">Knowledge Base (RAG)</h2>
                  <p className="mb-4 text-sm" style={{ color: "var(--text-muted)" }}>
                    Chunks loaded: {kbCount ?? "—"} (includes Cisco/Huawei CLI maps)
                  </p>
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
              )}

              {nav === "activity" && <ActivityLog entries={activity} />}
            </motion.div>
          </AnimatePresence>
        </div>

        <RoiCalculator roi={roi} />
      </main>

      {hitl && <CriticalAlert result={hitl} onApprove={onApprove} onReject={onReject} />}
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
