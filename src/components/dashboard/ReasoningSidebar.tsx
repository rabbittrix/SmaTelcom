import { AnimatePresence, motion } from "framer-motion";
import { Bot, Gavel, Shield } from "lucide-react";
import type { AgentOpinion, PipelineResult } from "../../lib/types";

const iconFor = (name: string) => {
  if (name.toLowerCase().includes("security")) return Shield;
  if (name.toLowerCase().includes("judge")) return Gavel;
  return Bot;
};

export function ReasoningSidebar({
  result,
  running,
}: {
  result: PipelineResult | null;
  running: boolean;
}) {
  const turns: { who: string; body: string; meta?: string }[] = [];

  if (running && !result) {
    turns.push(
      { who: "Performance Agent", body: "Analyzing capacity, QoS and congestion signals…" },
      { who: "Security Agent", body: "Assessing blast radius and policy integrity…" },
      { who: "Topology Agent", body: "Evaluating path diversity and site dependencies…" },
    );
  }

  if (result) {
    result.opinions.forEach((o: AgentOpinion) => {
      turns.push({
        who: o.agent,
        body: `${o.analysis}\n→ ${o.recommendation}`,
        meta: `confidence ${(o.confidence * 100).toFixed(0)}%`,
      });
    });
    if (result.conflict_resolution?.conflict_detected) {
      const c = result.conflict_resolution;
      turns.push({
        who: "Judge Agent — Conflict Resolution",
        body: `${c.winner} wins over ${c.loser}\nPriority: ${c.priority_applied}\n${c.rationale}\nPolicy: ${c.policy_citation}`,
        meta: "Security/Stability > Compliance > Performance",
      });
    }
    turns.push({
      who: "Judge Agent",
      body: `${result.judge_summary}\nLogic: ${result.decision_logic}\nCommand: ${result.proposed_command}`,
      meta: `risk ${result.risk} · ${result.status}`,
    });
  }

  return (
    <aside
      className="flex h-full min-h-[420px] w-full flex-col rounded-xl border lg:w-[320px] lg:shrink-0"
      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
    >
      <div className="border-b px-4 py-3" style={{ borderColor: "var(--border)" }}>
        <h3 className="text-sm font-semibold">Reasoning Sidebar</h3>
        <p className="text-xs" style={{ color: "var(--text-muted)" }}>
          Conversation between the 3 agents + Judge
        </p>
      </div>

      <div className="flex-1 space-y-3 overflow-y-auto p-3">
        <AnimatePresence initial={false}>
          {turns.length === 0 && (
            <p className="px-1 text-xs" style={{ color: "var(--text-muted)" }}>
              Run a network intent to watch specialist agents debate.
            </p>
          )}
          {turns.map((t, i) => {
            const Icon = iconFor(t.who);
            return (
              <motion.div
                key={`${t.who}-${i}`}
                initial={{ opacity: 0, x: 8 }}
                animate={{ opacity: 1, x: 0 }}
                className="rounded-lg border p-3"
                style={{ borderColor: "var(--border)" }}
              >
                <div className="mb-1.5 flex items-center gap-2">
                  <Icon className="h-3.5 w-3.5" style={{ color: "var(--accent)" }} />
                  <span className="text-xs font-semibold">{t.who}</span>
                </div>
                <p className="whitespace-pre-wrap text-xs leading-relaxed" style={{ color: "var(--text-muted)" }}>
                  {t.body}
                </p>
                {t.meta && (
                  <p className="mt-2 font-mono text-[10px]" style={{ color: "var(--accent)" }}>
                    {t.meta}
                  </p>
                )}
              </motion.div>
            );
          })}
        </AnimatePresence>
      </div>
    </aside>
  );
}
