import { motion } from "framer-motion";
import { AlertTriangle, Cpu, Signal, Waves, Zap } from "lucide-react";
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

export function NetworkHealth({ health }: { health: HealthSnapshot | null }) {
  if (!health) {
    return (
      <div className="rounded-xl border p-8 text-sm" style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}>
        Waiting for telemetry simulator…
      </div>
    );
  }

  const scoreColor =
    health.overall_score >= 85 ? "#22c55e" : health.overall_score >= 65 ? "#f59e0b" : "#ef4444";

  return (
    <div className="space-y-4">
      <div className="flex items-end justify-between">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">Network Health</h2>
          <p className="text-sm" style={{ color: "var(--text-muted)" }}>
            Live mock telemetry · refresh every 5s from Rust simulator
          </p>
        </div>
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
    </div>
  );
}
