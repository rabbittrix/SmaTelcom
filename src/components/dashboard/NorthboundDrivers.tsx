import { useState } from "react";
import { Loader2, Radio } from "lucide-react";
import { northboundCommit, northboundDryRun } from "../../lib/tauri";
import type { DriverResult } from "../../lib/types";

export function NorthboundDrivers({
  command,
  actionId,
}: {
  command: string;
  actionId?: string | null;
}) {
  const [protocol, setProtocol] = useState<"netconf" | "gnmi">("netconf");
  const [target, setTarget] = useState("lab-ne-01");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<DriverResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = async (dryRun: boolean) => {
    if (!command.trim()) {
      setError("No proposed command — run the pipeline first.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const r = dryRun
        ? await northboundDryRun({
            command,
            protocol,
            target,
            actionId: actionId ?? undefined,
          })
        : await northboundCommit({
            command,
            protocol,
            target,
            dryRun: false,
            actionId: actionId ?? undefined,
          });
      setResult(r);
    } catch (e) {
      setResult(null);
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="rounded-xl border p-4"
      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
    >
      <div className="mb-3 flex items-center gap-2">
        <Radio className="h-4 w-4" style={{ color: "var(--accent)" }} />
        <h3 className="text-sm font-semibold">Northbound Drivers (NETCONF / gNMI)</h3>
      </div>
      <p className="mb-3 text-xs" style={{ color: "var(--text-muted)" }}>
        Simulated lab RPCs only — Dry Run shows XML/JSON before commit.
      </p>

      <div className="mb-3 grid gap-2 sm:grid-cols-2">
        <label className="text-xs" style={{ color: "var(--text-muted)" }}>
          Protocol
          <select
            value={protocol}
            onChange={(e) => setProtocol(e.target.value as "netconf" | "gnmi")}
            className="mt-1 w-full rounded-lg border bg-transparent px-2 py-2 text-sm"
            style={{ borderColor: "var(--border)" }}
          >
            <option value="netconf">NETCONF (XML)</option>
            <option value="gnmi">gNMI (JSON)</option>
          </select>
        </label>
        <label className="text-xs" style={{ color: "var(--text-muted)" }}>
          Target NE
          <input
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            className="mt-1 w-full rounded-lg border bg-transparent px-2 py-2 text-sm"
            style={{ borderColor: "var(--border)" }}
          />
        </label>
      </div>

      <div
        className="mb-3 rounded-lg border p-2 font-mono text-[11px]"
        style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
      >
        {command || "— no command —"}
      </div>

      <div className="mb-3 flex flex-wrap gap-2">
        <button
          onClick={() => run(true)}
          disabled={busy}
          className="inline-flex items-center gap-2 rounded-lg border px-3 py-2 text-sm disabled:opacity-50"
          style={{ borderColor: "var(--border)" }}
        >
          {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
          Dry Run
        </button>
        <button
          onClick={() => run(false)}
          disabled={busy}
          className="inline-flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium text-white disabled:opacity-50"
          style={{ background: "#0d9488" }}
        >
          Commit (Simulated)
        </button>
      </div>

      {error && <p className="mb-2 text-sm text-red-500">{error}</p>}

      {result && (
        <div className="space-y-2">
          <div className="text-xs font-semibold" style={{ color: "var(--accent)" }}>
            {result.status}
            {result.commit_id ? ` · ${result.commit_id}` : ""}
          </div>
          <p className="text-xs" style={{ color: "var(--text-muted)" }}>
            {result.message}
          </p>
          <pre
            className="max-h-64 overflow-auto rounded-lg border p-3 font-mono text-[10px] leading-relaxed"
            style={{ borderColor: "var(--border)" }}
          >
            {result.payload.body}
          </pre>
        </div>
      )}
    </div>
  );
}
