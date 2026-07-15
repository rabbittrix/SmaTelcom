import { AnimatePresence, motion } from "framer-motion";
import { Bot, Shield, User } from "lucide-react";
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

function LevelBadge({ level }: { level: ActivityEntry["level"] }) {
  if (level === "agent" || level === "judge") {
    return (
      <span className="inline-flex items-center gap-1" style={{ color: levelColor[level] }}>
        <span aria-hidden>🤖</span>
        <Bot className="h-3 w-3" />
        <span>[{level === "judge" ? "JUDGE" : "AI"}]</span>
      </span>
    );
  }
  if (level === "safety") {
    return (
      <span className="inline-flex items-center gap-1" style={{ color: levelColor.safety }}>
        <span aria-hidden>🛡️</span>
        <Shield className="h-3 w-3" />
        <span>[GUARDRAILS]</span>
      </span>
    );
  }
  if (level === "hitl") {
    return (
      <span className="inline-flex items-center gap-1" style={{ color: levelColor.hitl }}>
        <span aria-hidden>👤</span>
        <User className="h-3 w-3" />
        <span>[HITL]</span>
      </span>
    );
  }
  return <span style={{ color: levelColor[level] }}>[{level.toUpperCase()}]</span>;
}

export function ActivityLog({ entries }: { entries: ActivityEntry[] }) {
  return (
    <div
      className="flex h-full min-h-[280px] flex-col rounded-xl border"
      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
    >
      <div className="border-b px-4 py-3" style={{ borderColor: "var(--border)" }}>
        <h3 className="text-sm font-semibold">Activity Log</h3>
        <p className="text-xs" style={{ color: "var(--text-muted)" }}>
          🤖 Agent reasoning · 🛡️ Deterministic Safety · 👤 HITL
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
              style={{
                background:
                  e.level === "safety"
                    ? "rgba(245,158,11,0.08)"
                    : e.level === "agent" || e.level === "judge"
                      ? "rgba(56,189,248,0.06)"
                      : e.level === "hitl"
                        ? "rgba(249,115,22,0.08)"
                        : "color-mix(in oklab, var(--bg) 70%, transparent)",
              }}
            >
              <span style={{ color: "var(--text-muted)" }}>{e.ts} </span>
              <LevelBadge level={e.level} />{" "}
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
