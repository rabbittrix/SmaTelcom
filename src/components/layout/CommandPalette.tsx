import { useEffect, useMemo, useState } from "react";
import { Search, X } from "lucide-react";
import { searchAuditLogs } from "../../lib/tauri";
import type { AuditLogEntry } from "../../lib/types";

export function CommandPalette({
  open,
  onClose,
  onSelect,
}: {
  open: boolean;
  onClose: () => void;
  onSelect?: (entry: AuditLogEntry) => void;
}) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<AuditLogEntry[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  useEffect(() => {
    if (!open) return;
    let alive = true;
    setBusy(true);
    const t = window.setTimeout(() => {
      searchAuditLogs(query, 25)
        .then((rows) => {
          if (alive) setHits(rows);
        })
        .catch(() => {
          if (alive) setHits([]);
        })
        .finally(() => {
          if (alive) setBusy(false);
        });
    }, 180);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [query, open]);

  const placeholder = useMemo(
    () => (busy ? "Searching audit_logs…" : "Search intents, commands, decisions…"),
    [busy],
  );

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[60] flex items-start justify-center bg-black/55 p-4 pt-[12vh] backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="w-full max-w-xl overflow-hidden rounded-2xl border shadow-2xl"
        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b px-3 py-3" style={{ borderColor: "var(--border)" }}>
          <Search className="h-4 w-4" style={{ color: "var(--accent)" }} />
          <input
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={placeholder}
            className="min-w-0 flex-1 bg-transparent text-sm outline-none"
          />
          <kbd
            className="rounded border px-1.5 py-0.5 text-[10px]"
            style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
          >
            Esc
          </kbd>
          <button onClick={onClose} style={{ color: "var(--text-muted)" }}>
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="max-h-[50vh] overflow-y-auto p-2">
          {hits.length === 0 && (
            <p className="px-2 py-6 text-center text-xs" style={{ color: "var(--text-muted)" }}>
              No matches in audit_logs.
            </p>
          )}
          {hits.map((h) => (
            <button
              key={h.id}
              onClick={() => {
                onSelect?.(h);
                onClose();
              }}
              className="mb-1 w-full rounded-lg border px-3 py-2 text-left hover:opacity-90"
              style={{ borderColor: "var(--border)" }}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="truncate text-xs font-medium">{h.intent}</span>
                <span className="shrink-0 font-mono text-[10px]" style={{ color: "var(--accent)" }}>
                  {h.decision}
                </span>
              </div>
              <div className="mt-1 truncate font-mono text-[10px]" style={{ color: "var(--text-muted)" }}>
                {h.final_command}
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
