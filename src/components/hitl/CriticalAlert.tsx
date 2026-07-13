import { motion } from "framer-motion";
import { Check, ShieldAlert, X } from "lucide-react";
import type { PipelineResult, PredictedImpact } from "../../lib/types";

export function CriticalAlert({
  result,
  onApprove,
  onReject,
}: {
  result: PipelineResult;
  onApprove: () => void;
  onReject: () => void;
}) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    >
      <motion.div
        initial={{ scale: 0.94, y: 12 }}
        animate={{ scale: 1, y: 0 }}
        className="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-2xl border shadow-2xl"
        style={{ background: "var(--bg-elevated)", borderColor: "rgba(239,68,68,0.45)" }}
      >
        <div
          className="flex items-center gap-3 border-b px-5 py-4"
          style={{ borderColor: "var(--border)", background: "rgba(239,68,68,0.08)" }}
        >
          <ShieldAlert className="h-6 w-6 text-red-500" />
          <div>
            <h3 className="text-lg font-semibold">Human-in-the-Loop Required</h3>
            <p className="text-xs" style={{ color: "var(--text-muted)" }}>
              Digital Twin Lite · review Predicted Impact before Approve
            </p>
          </div>
        </div>

        <div className="space-y-4 px-5 py-5 text-sm">
          <Field label="Proposed Command" mono>
            {result.proposed_command}
          </Field>
          <Field label="Decision Logic">{result.decision_logic}</Field>
          <Field label="Judge Summary">{result.judge_summary}</Field>

          <PredictedImpactPanel impact={result.predicted_impact} />

          {result.vendor_commands?.length > 0 && (
            <div>
              <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
                Vendor CLI Translations
              </div>
              <div className="grid gap-2 sm:grid-cols-2">
                {result.vendor_commands.map((v) => (
                  <pre
                    key={v.vendor}
                    className="overflow-auto rounded-lg border p-2 font-mono text-[10px]"
                    style={{ borderColor: "var(--border)" }}
                  >
                    <div className="mb-1 font-sans text-[10px] uppercase" style={{ color: "var(--accent)" }}>
                      {v.vendor} · conf {(v.confidence * 100).toFixed(0)}%
                    </div>
                    {v.cli}
                  </pre>
                ))}
              </div>
            </div>
          )}

          <div className="flex gap-3">
            <Field label="Risk Assessment">
              <span
                className="rounded px-2 py-0.5 font-mono text-xs uppercase"
                style={{
                  background:
                    result.risk === "critical" || result.risk === "high"
                      ? "rgba(239,68,68,0.15)"
                      : "rgba(245,158,11,0.15)",
                  color:
                    result.risk === "critical" || result.risk === "high" ? "#ef4444" : "#f59e0b",
                }}
              >
                {result.risk}
              </span>
            </Field>
            <Field label="Safety Linter">{result.lint.reason}</Field>
          </div>
        </div>

        <div className="flex justify-end gap-3 border-t px-5 py-4" style={{ borderColor: "var(--border)" }}>
          <button
            onClick={onReject}
            className="inline-flex items-center gap-2 rounded-lg border px-4 py-2 text-sm font-medium"
            style={{ borderColor: "var(--border)" }}
          >
            <X className="h-4 w-4" /> Reject
          </button>
          <button
            onClick={onApprove}
            className="inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium text-white"
            style={{ background: "#0d9488" }}
          >
            <Check className="h-4 w-4" /> Approve
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}

function PredictedImpactPanel({ impact }: { impact: PredictedImpact }) {
  const rows = [
    ["CPU %", impact.cpu_pct_before, impact.cpu_pct_after],
    ["Latency ms", impact.latency_ms_before, impact.latency_ms_after],
    ["Throughput Gbps", impact.throughput_gbps_before, impact.throughput_gbps_after],
    ["Loss %", impact.packet_loss_pct_before, impact.packet_loss_pct_after],
  ] as const;

  return (
    <div
      className="rounded-xl border p-3"
      style={{ borderColor: "rgba(45,212,191,0.35)", background: "rgba(13,148,136,0.06)" }}
    >
      <div className="mb-2 text-xs font-semibold uppercase tracking-wider" style={{ color: "var(--accent)" }}>
        Predicted Impact (Digital Twin Lite)
      </div>
      <p className="mb-3 text-xs" style={{ color: "var(--text-muted)" }}>
        {impact.summary} · Blast radius: {impact.blast_radius}
      </p>
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {rows.map(([label, before, after]) => (
          <div key={label} className="rounded-lg border p-2" style={{ borderColor: "var(--border)" }}>
            <div className="text-[10px] uppercase" style={{ color: "var(--text-muted)" }}>
              {label}
            </div>
            <div className="font-mono text-xs">
              {Number(before).toFixed(1)} → <span style={{ color: "var(--accent)" }}>{Number(after).toFixed(1)}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function Field({
  label,
  children,
  mono,
}: {
  label: string;
  children: React.ReactNode;
  mono?: boolean;
}) {
  return (
    <div className="min-w-0 flex-1">
      <div className="mb-1 text-[10px] font-semibold uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
        {label}
      </div>
      <div className={mono ? "font-mono text-xs leading-relaxed" : "leading-relaxed"}>{children}</div>
    </div>
  );
}
