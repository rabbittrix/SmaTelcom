import { motion } from "framer-motion";
import { AlertTriangle, Cpu, Signal, Sparkles, Waves, Zap } from "lucide-react";
import { useMemo } from "react";
import type { HealthSnapshot } from "../../lib/types";

function Metric({
  label,
  value,
  unit,
  icon: Icon,
}: {
  label: string;
  value: string;
  unit?: string;
  icon: typeof Zap;
}) {
  return (
    <div
      className="rounded-xl border p-4"
      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
    >
      <div className="mb-3 flex items-center justify-between">
        <span className="text-xs font-medium uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
          {label}
        </span>
        <Icon className="h-4 w-4" style={{ color: "var(--accent)" }} />
      </div>
      <div className="font-mono text-2xl font-semibold tracking-tight">
        {value}
        {unit && (
          <span className="ml-1 text-sm font-normal" style={{ color: "var(--text-muted)" }}>
            {unit}
          </span>
        )}
      </div>
    </div>
  );
}

function Sparkline({
  values,
  color,
  label,
}: {
  values: number[];
  color: string;
  label: string;
}) {
  const path = useMemo(() => {
    if (values.length < 2) return "";
    const min = Math.min(...values);
    const max = Math.max(...values);
    const span = max - min || 1;
    const w = 240;
    const h = 48;
    return values
      .map((v, i) => {
        const x = (i / (values.length - 1)) * w;
        const y = h - ((v - min) / span) * (h - 4) - 2;
        return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }, [values]);

  return (
    <div
      className="rounded-xl border p-3"
      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
    >
      <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
        {label}
      </div>
      <svg viewBox="0 0 240 48" className="h-12 w-full" preserveAspectRatio="none">
        <path d={path} fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" />
      </svg>
    </div>
  );
}

function seriesFromEvents(
  health: HealthSnapshot,
  metric: string,
  fallback: number,
): number[] {
  const fromEvents = health.recent_events
    .filter((e) => e.metric === metric)
    .map((e) => e.value)
    .reverse();
  if (fromEvents.length >= 2) return fromEvents.slice(-24);
  return Array.from({ length: 12 }, (_, i) =>
    Number((fallback * (0.92 + (i % 5) * 0.02)).toFixed(2)),
  );
}

export function NetworkHealth({
  health,
  autonomyGlow = false,
}: {
  health: HealthSnapshot | null;
  autonomyGlow?: boolean;
}) {
  if (!health) {
    return (
      <div className="rounded-xl border p-8 text-sm" style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}>
        Waiting for telemetry simulator…
      </div>
    );
  }

  const scoreColor =
    health.overall_score >= 85 ? "#22c55e" : health.overall_score >= 65 ? "#f59e0b" : "#ef4444";

  const latencySeries = seriesFromEvents(health, "latency", health.latency_ms);
  const lossSeries = seriesFromEvents(health, "packet_loss", health.packet_loss_pct);
  const tpSeries = seriesFromEvents(health, "throughput", health.throughput_gbps);
  const savings = health.autonomy_savings ?? 0;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">Network Health</h2>
          <p className="text-sm" style={{ color: "var(--text-muted)" }}>
            Live Rust telemetry · push every 5s (`telemetry-tick`)
          </p>
        </div>
        <div className="flex items-stretch gap-3">
          <motion.div
            key={`savings-${savings}-${autonomyGlow}`}
            initial={autonomyGlow ? { scale: 0.9, boxShadow: "0 0 0 0 rgba(45,212,191,0.7)" } : false}
            animate={
              autonomyGlow
                ? {
                    scale: [1, 1.06, 1],
                    boxShadow: [
                      "0 0 0 0 rgba(45,212,191,0.55)",
                      "0 0 28px 8px rgba(45,212,191,0.45)",
                      "0 0 0 0 rgba(45,212,191,0)",
                    ],
                  }
                : { scale: 1, boxShadow: "0 0 0 0 rgba(0,0,0,0)" }
            }
            transition={{ duration: 1.1, ease: "easeOut" }}
            className="rounded-xl border px-5 py-3 text-right"
            style={{
              borderColor: autonomyGlow ? "rgba(45,212,191,0.65)" : "var(--border)",
              background: autonomyGlow
                ? "linear-gradient(135deg, rgba(13,148,136,0.25), rgba(45,212,191,0.08))"
                : "var(--bg-elevated)",
            }}
          >
            <div
              className="flex items-center justify-end gap-1.5 text-[10px] uppercase tracking-widest"
              style={{ color: "var(--accent)" }}
            >
              <Sparkles className="h-3 w-3" />
              Autonomy Savings
            </div>
            <div className="font-mono text-3xl font-bold" style={{ color: "#2dd4bf" }}>
              {savings}
            </div>
            <div className="text-[10px]" style={{ color: "var(--text-muted)" }}>
              auto-approved this session
            </div>
          </motion.div>
          <motion.div
            key={health.overall_score}
            initial={{ scale: 0.92, opacity: 0.6 }}
            animate={{ scale: 1, opacity: 1 }}
            className="rounded-xl border px-5 py-3 text-right"
            style={{ borderColor: "var(--border)", background: "var(--bg-elevated)" }}
          >
            <div className="text-[10px] uppercase tracking-widest" style={{ color: "var(--text-muted)" }}>
              Health Score
            </div>
            <div className="font-mono text-3xl font-bold" style={{ color: scoreColor }}>
              {health.overall_score}
            </div>
          </motion.div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3 xl:grid-cols-4">
        <Metric label="Latency" value={health.latency_ms.toFixed(1)} unit="ms" icon={Zap} />
        <Metric label="Packet Loss" value={health.packet_loss_pct.toFixed(2)} unit="%" icon={Waves} />
        <Metric label="Throughput" value={health.throughput_gbps.toFixed(1)} unit="Gbps" icon={Signal} />
        <Metric
          label="Sites Online"
          value={`${health.sites_online}/${health.sites_total}`}
          icon={Cpu}
        />
      </div>

      <div className="grid gap-3 md:grid-cols-3">
        <Sparkline values={latencySeries} color="#2dd4bf" label="Latency trend" />
        <Sparkline values={lossSeries} color="#f59e0b" label="Packet loss trend" />
        <Sparkline values={tpSeries} color="#38bdf8" label="Throughput trend" />
      </div>

      {health.active_alarms > 0 && (
        <div
          className="flex items-center gap-2 rounded-lg border px-3 py-2 text-sm"
          style={{
            borderColor: "rgba(245,158,11,0.35)",
            background: "rgba(245,158,11,0.08)",
            color: "#f59e0b",
          }}
        >
          <AlertTriangle className="h-4 w-4" />
          {health.active_alarms} active alarm{health.active_alarms === 1 ? "" : "s"}
          {health.last_event ? ` · ${health.last_event.message}` : ""}
        </div>
      )}

      {health.recent_events.length > 0 && (
        <div
          className="rounded-xl border p-3"
          style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
        >
          <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
            Recent events
          </div>
          <div className="max-h-28 space-y-1 overflow-y-auto font-mono text-[10px]">
            {health.recent_events.slice(0, 8).map((e) => (
              <div key={e.id} className="flex justify-between gap-2">
                <span style={{ color: "var(--text-muted)" }}>{e.timestamp.slice(11, 19)}</span>
                <span className="truncate">{e.message}</span>
                <span
                  style={{
                    color:
                      e.severity === "critical"
                        ? "#ef4444"
                        : e.severity === "warning"
                          ? "#f59e0b"
                          : "var(--text-muted)",
                  }}
                >
                  {e.severity}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
