import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Check, Code2, ShieldAlert, X } from "lucide-react";
import { northboundDryRun } from "../../lib/tauri";
import type { DriverResult, PipelineResult, PredictedImpact } from "../../lib/types";

type Tab = "review" | "protocol";

export function CriticalAlert({
  result,
  onApprove,
  onReject,
}: {
  result: PipelineResult;
  onApprove: () => void;
  onReject: () => void;
}) {
  const [tab, setTab] = useState<Tab>("review");
  const [protocol, setProtocol] = useState<"netconf" | "gnmi">("netconf");
  const [preview, setPreview] = useState<DriverResult | null>(null);
  const [previewErr, setPreviewErr] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setPreviewErr(null);
    northboundDryRun({
      command: result.proposed_command,
      protocol,
      target: "lab-ne-01",
      actionId: result.id,
    })
      .then((r) => {
        if (alive) setPreview(r);
      })
      .catch((e) => {
        if (alive) {
          setPreview(null);
          setPreviewErr(String(e));
        }
      });
    return () => {
      alive = false;
    };
  }, [result.proposed_command, result.id, protocol]);

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
              Review decision + Protocol Preview before Approve
            </p>
          </div>
        </div>

        <div className="flex gap-2 border-b px-5 pt-3" style={{ borderColor: "var(--border)" }}>
          {(
            [
              ["review", "Decision Review"],
              ["protocol", "Protocol Preview"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              onClick={() => setTab(id)}
              className="rounded-t-lg px-3 py-2 text-xs font-semibold"
              style={{
                color: tab === id ? "var(--accent)" : "var(--text-muted)",
                borderBottom: tab === id ? "2px solid var(--accent)" : "2px solid transparent",
              }}
            >
              {id === "protocol" ? (
                <span className="inline-flex items-center gap-1">
                  <Code2 className="h-3.5 w-3.5" /> {label}
                </span>
              ) : (
                label
              )}
            </button>
          ))}
        </div>

        <div className="space-y-4 px-5 py-5 text-sm">
          {tab === "review" && (
            <>
              <Field label="Proposed Command" mono>
                {result.proposed_command}
              </Field>
              <Field label="Risk Level">
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
              <Field label="Reasoning">{result.decision_logic}</Field>
              <Field label="Judge Summary">{result.judge_summary}</Field>
              <PredictedImpactPanel impact={result.predicted_impact} />
              <div className="flex gap-3">
                <Field label="Safety Linter">{result.lint.reason}</Field>
                <Field label="HITL Required">
                  <span className="font-mono text-xs">{String(result.lint.requires_hitl)}</span>
                </Field>
              </div>
            </>
          )}

          {tab === "protocol" && (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-[10px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
                  Driver
                </span>
                <button
                  onClick={() => setProtocol("netconf")}
                  className="rounded border px-2 py-1 text-xs"
                  style={{
                    borderColor: protocol === "netconf" ? "var(--accent)" : "var(--border)",
                    color: protocol === "netconf" ? "var(--accent)" : "var(--text-muted)",
                  }}
                >
                  NETCONF (XML)
                </button>
                <button
                  onClick={() => setProtocol("gnmi")}
                  className="rounded border px-2 py-1 text-xs"
                  style={{
                    borderColor: protocol === "gnmi" ? "var(--accent)" : "var(--border)",
                    color: protocol === "gnmi" ? "var(--accent)" : "var(--text-muted)",
                  }}
                >
                  gNMI (JSON)
                </button>
              </div>
              {previewErr && <p className="text-sm text-red-500">{previewErr}</p>}
              {preview && (
                <>
                  <p className="text-xs" style={{ color: "var(--text-muted)" }}>
                    {preview.message} · {preview.payload.content_type}
                  </p>
                  <pre
                    className="max-h-72 overflow-auto rounded-lg border p-3 font-mono text-[10px] leading-relaxed"
                    style={{ borderColor: "var(--border)", background: "color-mix(in oklab, var(--bg) 80%, transparent)" }}
                  >
                    {preview.payload.body}
                  </pre>
                </>
              )}
              {!preview && !previewErr && (
                <p className="text-xs" style={{ color: "var(--text-muted)" }}>
                  Generating dry-run payload…
                </p>
              )}
            </>
          )}
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
              {Number(before).toFixed(1)} →{" "}
              <span style={{ color: "var(--accent)" }}>{Number(after).toFixed(1)}</span>
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
