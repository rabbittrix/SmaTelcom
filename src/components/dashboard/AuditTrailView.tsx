import { useState } from "react";
import { motion } from "framer-motion";
import { ClipboardList, Code2 } from "lucide-react";
import type { AuditLogEntry } from "../../lib/types";

function decisionStyle(decision: string): { bg: string; color: string } {
  const d = decision.toLowerCase();
  if (d.includes("auto")) return { bg: "rgba(34,197,94,0.15)", color: "#22c55e" };
  if (d.includes("blocked") || d.includes("critical"))
    return { bg: "rgba(239,68,68,0.15)", color: "#ef4444" };
  if (d.includes("reject")) return { bg: "rgba(248,113,113,0.12)", color: "#f87171" };
  if (d.includes("hitl")) return { bg: "rgba(249,115,22,0.15)", color: "#f97316" };
  return { bg: "rgba(148,163,184,0.12)", color: "var(--text-muted)" };
}

function riskStyle(risk: string): string {
  const r = risk.toLowerCase();
  if (r.includes("critical") || r.includes("high")) return "#ef4444";
  if (r.includes("medium")) return "#f59e0b";
  return "#22c55e";
}

export function AuditTrailView({ entries }: { entries: AuditLogEntry[] }) {
  const [openPayload, setOpenPayload] = useState<AuditLogEntry | null>(null);

  return (
    <div className="space-y-4">
      <div>
        <h2 className="flex items-center gap-2 text-xl font-semibold tracking-tight">
          <ClipboardList className="h-5 w-5" style={{ color: "var(--accent)" }} />
          Audit Trail
        </h2>
        <p className="text-sm" style={{ color: "var(--text-muted)" }}>
          Persistent SQLite `audit_logs` — compliance-grade decision history
        </p>
      </div>

      <div
        className="overflow-x-auto rounded-xl border"
        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
      >
        <table className="w-full min-w-[900px] text-left text-xs">
          <thead>
            <tr className="border-b text-[10px] uppercase tracking-wider" style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}>
              <th className="px-3 py-3">Time</th>
              <th className="px-3 py-3">Intent</th>
              <th className="px-3 py-3">Command</th>
              <th className="px-3 py-3">Risk</th>
              <th className="px-3 py-3">Decision</th>
              <th className="px-3 py-3">Payload</th>
            </tr>
          </thead>
          <tbody>
            {entries.length === 0 && (
              <tr>
                <td colSpan={6} className="px-3 py-8 text-center" style={{ color: "var(--text-muted)" }}>
                  No audit rows yet. Run the multi-agent pipeline.
                </td>
              </tr>
            )}
            {entries.map((e) => {
              const ds = decisionStyle(e.decision);
              const isCritical =
                e.risk_level.toLowerCase().includes("critical") ||
                e.risk_level.toLowerCase().includes("high") ||
                e.decision.toLowerCase().includes("blocked");
              return (
                <motion.tr
                  key={e.id}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className="border-b"
                  style={{
                    borderColor: "var(--border)",
                    background: isCritical ? "rgba(239,68,68,0.04)" : "transparent",
                  }}
                >
                  <td className="whitespace-nowrap px-3 py-2 font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>
                    {e.timestamp.slice(0, 19).replace("T", " ")}
                  </td>
                  <td className="max-w-[180px] truncate px-3 py-2" title={e.intent}>
                    {e.intent}
                  </td>
                  <td className="max-w-[200px] truncate px-3 py-2 font-mono text-[10px]" title={e.final_command}>
                    {e.final_command}
                  </td>
                  <td className="px-3 py-2">
                    <span className="font-mono text-[10px] uppercase" style={{ color: riskStyle(e.risk_level) }}>
                      {e.risk_level}
                    </span>
                  </td>
                  <td className="px-3 py-2">
                    <span
                      className="rounded px-2 py-0.5 font-mono text-[10px]"
                      style={{ background: ds.bg, color: ds.color }}
                    >
                      {e.decision}
                    </span>
                  </td>
                  <td className="px-3 py-2">
                    {e.payload_preview ? (
                      <button
                        onClick={() => setOpenPayload(e)}
                        className="inline-flex items-center gap-1 text-[10px] font-medium"
                        style={{ color: "var(--accent)" }}
                      >
                        <Code2 className="h-3 w-3" /> View
                      </button>
                    ) : (
                      <span style={{ color: "var(--text-muted)" }}>—</span>
                    )}
                  </td>
                </motion.tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {openPayload && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
          onClick={() => setOpenPayload(null)}
        >
          <div
            className="max-h-[80vh] w-full max-w-2xl overflow-hidden rounded-2xl border"
            style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
            onClick={(ev) => ev.stopPropagation()}
          >
            <div className="border-b px-4 py-3 text-sm font-semibold" style={{ borderColor: "var(--border)" }}>
              Payload Preview · {openPayload.id.slice(0, 8)}
            </div>
            <pre className="max-h-[60vh] overflow-auto p-4 font-mono text-[10px] leading-relaxed">
              {openPayload.payload_preview}
            </pre>
            {openPayload.conflict_resolution && (
              <div className="border-t px-4 py-3 text-xs" style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}>
                Conflict: {openPayload.conflict_resolution}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
