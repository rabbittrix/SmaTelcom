import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Loader2, Play, Shield, Sparkles } from "lucide-react";
import { Sidebar, type NavId } from "./components/layout/Sidebar";
import { NetworkHealth } from "./components/dashboard/NetworkHealth";
import { ActivityLog } from "./components/dashboard/ActivityLog";
import { CriticalAlert } from "./components/hitl/CriticalAlert";
import { ThemeProvider } from "./hooks/useTheme";
import {
  analyzeIntent,
  approveAction,
  checkOllama,
  getTelemetry,
  lintCommand,
  rejectAction,
  reloadKnowledgeBase,
} from "./lib/tauri";
import type { ActivityEntry, HealthSnapshot, LintResult, PipelineResult } from "./lib/types";

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

  const pushLog = useCallback((level: ActivityEntry["level"], message: string) => {
    setActivity((prev) => [
      { id: crypto.randomUUID(), ts: nowTs(), level, message },
      ...prev,
    ].slice(0, 80));
  }, []);

  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        const snap = await getTelemetry();
        if (alive) setHealth(snap);
      } catch {
        /* running under vite without tauri */
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
  }, []);

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
      setResult(res);
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
  };

  const onReject = async () => {
    if (!hitl) return;
    const msg = await rejectAction(hitl.id);
    pushLog("error", msg);
    setHitl(null);
  };

  const runLint = async () => {
    try {
      const r = await lintCommand(lintDemo);
      setLintResult(r);
      pushLog("safety", r.reason);
    } catch (e) {
      pushLog("error", String(e));
    }
  };

  return (
    <div className="flex h-full">
      <Sidebar active={nav} onNavigate={setNav} ollamaOk={ollamaOk} />

      <main className="flex min-w-0 flex-1 flex-col">
        <header
          className="flex items-center justify-between border-b px-6 py-4"
          style={{ borderColor: "var(--border)" }}
        >
          <div>
            <h1 className="text-lg font-semibold tracking-tight">
              {nav === "dashboard" && "Operations Dashboard"}
              {nav === "pipeline" && "Multi-Agent Intent Pipeline"}
              {nav === "safety" && "Deterministic Safety Linter"}
              {nav === "knowledge" && "Local Knowledge Base (RAG)"}
              {nav === "activity" && "Agent Activity Stream"}
            </h1>
            <p className="text-xs" style={{ color: "var(--text-muted)" }}>
              Localhost-only · Ollama :11434 · Rust guardrails before HITL
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

        <div className="flex-1 overflow-y-auto p-6">
          <AnimatePresence mode="wait">
            <motion.div
              key={nav}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.22 }}
              className="mx-auto max-w-6xl space-y-6"
            >
              {(nav === "dashboard" || nav === "pipeline") && (
                <>
                  <NetworkHealth health={health} />

                  <div
                    className="rounded-xl border p-4"
                    style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                  >
                    <label className="mb-2 block text-xs font-semibold uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
                      Network Intent
                    </label>
                    <textarea
                      value={intent}
                      onChange={(e) => setIntent(e.target.value)}
                      rows={3}
                      className="w-full resize-none rounded-lg border bg-transparent px-3 py-2 text-sm outline-none focus:ring-2"
                      style={{
                        borderColor: "var(--border)",
                        // @ts-expect-error css var
                        "--tw-ring-color": "var(--accent)",
                      }}
                    />
                    <div className="mt-3 flex justify-end">
                      <button
                        onClick={runPipeline}
                        disabled={running || !intent.trim()}
                        className="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
                        style={{ background: "#0d9488" }}
                      >
                        {running ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
                        Run Multi-Agent Pipeline
                      </button>
                    </div>
                  </div>

                  {result && (
                    <div className="grid gap-4 lg:grid-cols-2">
                      {result.opinions.map((o) => (
                        <div
                          key={o.agent}
                          className="rounded-xl border p-4"
                          style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                        >
                          <div className="mb-2 flex items-center justify-between">
                            <h4 className="text-sm font-semibold">{o.agent}</h4>
                            <span className="font-mono text-xs" style={{ color: "var(--accent)" }}>
                              conf {(o.confidence * 100).toFixed(0)}%
                            </span>
                          </div>
                          <p className="mb-2 text-sm" style={{ color: "var(--text-muted)" }}>
                            {o.analysis}
                          </p>
                          <p className="font-mono text-xs">{o.recommendation}</p>
                        </div>
                      ))}
                      <div
                        className="rounded-xl border p-4 lg:col-span-2"
                        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                      >
                        <h4 className="mb-2 text-sm font-semibold">Judge Agent</h4>
                        <p className="mb-2 text-sm">{result.judge_summary}</p>
                        <p className="font-mono text-xs" style={{ color: "var(--accent)" }}>
                          {result.proposed_command}
                        </p>
                        <p className="mt-2 text-xs" style={{ color: "var(--text-muted)" }}>
                          Status: {result.status} · Risk: {result.risk}
                          {result.knowledge_used ? " · RAG grounded" : ""}
                        </p>
                      </div>
                    </div>
                  )}

                  <div className="grid gap-4 lg:grid-cols-2">
                    <ActivityLog entries={activity} />
                    <div
                      className="rounded-xl border p-4"
                      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                    >
                      <h3 className="mb-2 text-sm font-semibold">Recent Telemetry Events</h3>
                      <div className="max-h-[280px] space-y-2 overflow-y-auto font-mono text-xs">
                        {(health?.recent_events ?? []).slice(0, 12).map((ev) => (
                          <div key={ev.id} className="flex justify-between gap-2 border-b py-1" style={{ borderColor: "var(--border)" }}>
                            <span className="truncate">{ev.message}</span>
                            <span style={{ color: "var(--text-muted)" }}>{ev.severity}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                </>
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
                  <p className="mb-4 text-sm" style={{ color: "var(--text-muted)" }}>
                    Deterministic blacklist validation — no LLM. Runs before HITL.
                  </p>
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
                    <pre className="mt-4 overflow-auto rounded-lg border p-3 font-mono text-xs" style={{ borderColor: "var(--border)" }}>
                      {JSON.stringify(lintResult, null, 2)}
                    </pre>
                  )}
                  <div className="mt-4 text-xs" style={{ color: "var(--text-muted)" }}>
                    Try: <code>shutdown core_router cr-01</code> or <code>delete running-config</code>
                  </div>
                </div>
              )}

              {nav === "knowledge" && (
                <div
                  className="rounded-xl border p-5"
                  style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
                >
                  <h2 className="mb-2 text-lg font-semibold">Knowledge Base (RAG)</h2>
                  <p className="mb-4 text-sm" style={{ color: "var(--text-muted)" }}>
                    Place <code>.txt</code> / <code>.pdf</code> manuals in <code>knowledge_base/</code>.
                    Chunks loaded: {kbCount ?? "—"}
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
