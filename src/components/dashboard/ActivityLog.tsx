import { AnimatePresence, motion } from "framer-motion";
import type { ActivityEntry } from "../../lib/types";

const levelColor: Record<ActivityEntry["level"], string> = {
  info: "var(--text-muted)",
  agent: "#38bdf8",
  judge: "#a78bfa",
  safety: "#f59e0b",
  hitl: "#f97316",
  ok: "#22c55e",
  error: "#ef4444",
};

export function ActivityLog({ entries }: { entries: ActivityEntry[] }) {
  return (
    <div
      className="flex h-full min-h-[280px] flex-col rounded-xl border"
      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
    >
      <div className="border-b px-4 py-3" style={{ borderColor: "var(--border)" }}>
        <h3 className="text-sm font-semibold">Activity Log</h3>
        <p className="text-xs" style={{ color: "var(--text-muted)" }}>
          Agents think in real time
        </p>
      </div>
      <div className="flex-1 space-y-2 overflow-y-auto p-3 font-mono text-xs">
        <AnimatePresence initial={false}>
          {entries.map((e) => (
            <motion.div
              key={e.id}
              initial={{ opacity: 0, y: -6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              className="rounded-md px-2 py-1.5"
              style={{ background: "color-mix(in oklab, var(--bg) 70%, transparent)" }}
            >
              <span style={{ color: "var(--text-muted)" }}>{e.ts} </span>
              <span style={{ color: levelColor[e.level] }}>[{e.level.toUpperCase()}] </span>
              <span>{e.message}</span>
            </motion.div>
          ))}
        </AnimatePresence>
        {entries.length === 0 && (
          <p style={{ color: "var(--text-muted)" }}>No agent activity yet. Submit a network intent.</p>
        )}
      </div>
    </div>
  );
}
