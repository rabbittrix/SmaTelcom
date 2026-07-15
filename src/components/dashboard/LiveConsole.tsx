import { useEffect, useRef } from "react";
import { Terminal } from "lucide-react";
import { CommandInput, type CommandInputHandle } from "../cli/CommandInput";
import type { ConsoleLine } from "../../lib/types";

const levelColor: Record<string, string> = {
  info: "#94a3b8",
  apply: "#38bdf8",
  ok: "#22c55e",
  warn: "#f59e0b",
  error: "#ef4444",
};

export function LiveConsole({
  lines,
  commandValue,
  onCommandChange,
  onCommandSubmit,
  loading,
  inputRef,
  showInput = true,
}: {
  lines: ConsoleLine[];
  commandValue?: string;
  onCommandChange?: (v: string) => void;
  onCommandSubmit?: (v: string) => void | Promise<void>;
  loading?: boolean;
  inputRef?: React.RefObject<CommandInputHandle | null>;
  showInput?: boolean;
}) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [lines.length]);

  return (
    <div
      className="relative z-10 flex h-full min-h-[320px] flex-col overflow-hidden rounded-xl border"
      style={{
        background: "#0a0f14",
        borderColor: "var(--border)",
        fontFamily: "var(--font-mono), ui-monospace, monospace",
      }}
    >
      <div
        className="flex items-center gap-2 border-b px-3 py-2 text-xs"
        style={{ borderColor: "rgba(148,163,184,0.2)", color: "#94a3b8" }}
      >
        <Terminal className="h-3.5 w-3.5" style={{ color: "#2dd4bf" }} />
        Live Execution Log · SSH / NETCONF / gNMI
        {loading && (
          <span className="ml-auto animate-pulse text-[10px]" style={{ color: "#2dd4bf" }}>
            AI thinking…
          </span>
        )}
      </div>
      <div
        className="flex-1 space-y-1 overflow-y-auto p-3 text-[11px] leading-relaxed"
        onMouseDown={(e) => {
          // Keep focus path open for the CLI below — don't steal clicks oddly
          if ((e.target as HTMLElement).tagName === "DIV") {
            /* allow scroll */
          }
        }}
      >
        {lines.length === 0 && (
          <p style={{ color: "#64748b" }}>
            Type an intent below and press Enter — or approve an action to watch the northbound push.
          </p>
        )}
        {lines.map((l, i) => (
          <div key={`${l.ts}-${i}`} className="flex gap-2">
            <span style={{ color: "#475569" }}>{l.ts.slice(11, 19)}</span>
            <span style={{ color: levelColor[l.level] ?? "#94a3b8" }}>
              [{l.level === "ok" ? "SUCCESS" : l.level === "apply" ? "DEBUG" : l.level.toUpperCase()}]
            </span>
            <span style={{ color: "#e2e8f0" }}>{l.message}</span>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      {showInput && onCommandChange && onCommandSubmit && (
        <div
          className="relative z-20 border-t p-2"
          style={{ borderColor: "rgba(148,163,184,0.15)", background: "#060a0f" }}
        >
          <CommandInput
            ref={inputRef}
            value={commandValue ?? ""}
            onChange={onCommandChange}
            onSubmit={onCommandSubmit}
            loading={loading}
            mode="cli"
            autoFocus
            placeholder="smartelcom> network intent…"
            hint="Enter runs analyze_network_intent · Ctrl+K focuses this CLI"
          />
        </div>
      )}
    </div>
  );
}

export function ValidationBadge({
  status,
}: {
  status: "verified" | "drift_detected" | "pending" | "skipped" | null;
}) {
  if (!status || status === "pending" || status === "skipped") {
    return (
      <span
        className="rounded-full border px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider"
        style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
      >
        Post-Execution · Pending
      </span>
    );
  }
  if (status === "verified") {
    return (
      <span
        className="rounded-full px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider"
        style={{ background: "rgba(34,197,94,0.18)", color: "#22c55e" }}
      >
        Post-Execution Validation · Verified
      </span>
    );
  }
  return (
    <span
      className="rounded-full px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider"
      style={{ background: "rgba(245,158,11,0.18)", color: "#f59e0b" }}
    >
      Post-Execution Validation · Drift Detected
    </span>
  );
}
