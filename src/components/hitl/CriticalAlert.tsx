import { motion } from "framer-motion";
import { Check, ShieldAlert, X } from "lucide-react";
import type { PipelineResult } from "../../lib/types";

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
        className="w-full max-w-xl overflow-hidden rounded-2xl border shadow-2xl"
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
              Graduated autonomy · action flagged before execution
            </p>
          </div>
        </div>

        <div className="space-y-4 px-5 py-5 text-sm">
          <Field label="Proposed Command" mono>
            {result.proposed_command}
          </Field>
          <Field label="Decision Logic">{result.decision_logic}</Field>
          <Field label="Judge Summary">{result.judge_summary}</Field>
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
