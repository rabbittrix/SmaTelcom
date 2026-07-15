import { motion } from "framer-motion";
import { Clock3, ShieldAlert, Sparkles, Target } from "lucide-react";
import type { ImpactReport } from "../../lib/types";

export function ImpactReportView({ report }: { report: ImpactReport | null }) {
  if (!report) {
    return (
      <div
        className="rounded-xl border p-8 text-sm"
        style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
      >
        Loading impact metrics…
      </div>
    );
  }

  const cards = [
    {
      label: "Total Intentions Processed",
      value: String(report.intents_processed),
      hint: "Judge decisions persisted to SQLite",
      icon: Target,
      color: "#38bdf8",
    },
    {
      label: "Human Hours Saved",
      value: report.human_hours_saved.toFixed(2),
      hint: `${report.minutes_per_auto_approve} min × ${report.auto_approved} auto-approves`,
      icon: Clock3,
      color: "#2dd4bf",
    },
    {
      label: "Critical Risks Averted",
      value: String(report.critical_risks_averted),
      hint: "High/Critical blocks by Rust Safety Linter",
      icon: ShieldAlert,
      color: "#f59e0b",
    },
    {
      label: "Auto-Approved",
      value: String(report.auto_approved),
      hint: "Low-risk graduated autonomy",
      icon: Sparkles,
      color: "#22c55e",
    },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold tracking-tight">Impact Report</h2>
        <p className="text-sm" style={{ color: "var(--text-muted)" }}>
          Business value of AN Level-4 autonomy — compliance-grade session + audit metrics
        </p>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {cards.map((c, i) => (
          <motion.div
            key={c.label}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.06 }}
            className="rounded-xl border p-5"
            style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
          >
            <div className="mb-3 flex items-center justify-between">
              <span className="text-[10px] font-semibold uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
                {c.label}
              </span>
              <c.icon className="h-4 w-4" style={{ color: c.color }} />
            </div>
            <div className="font-mono text-3xl font-bold" style={{ color: c.color }}>
              {c.value}
            </div>
            <p className="mt-2 text-[11px]" style={{ color: "var(--text-muted)" }}>
              {c.hint}
            </p>
          </motion.div>
        ))}
      </div>

      <div
        className="rounded-xl border p-4 text-sm"
        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
      >
        <h3 className="mb-2 text-sm font-semibold">Autonomy mix</h3>
        <div className="grid gap-2 font-mono text-xs md:grid-cols-3">
          <div>HITL path: {report.hitl_pending_or_resolved}</div>
          <div>Blocked: {report.blocked}</div>
          <div>Auto-approved: {report.auto_approved}</div>
        </div>
      </div>
    </div>
  );
}
