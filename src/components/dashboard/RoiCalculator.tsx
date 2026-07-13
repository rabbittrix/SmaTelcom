import { motion } from "framer-motion";
import { Clock3, ShieldCheck } from "lucide-react";
import type { RoiSnapshot } from "../../lib/types";

export function RoiCalculator({ roi }: { roi: RoiSnapshot | null }) {
  const hours = roi?.engineering_hours_saved ?? 0;
  const risks = roi?.risks_mitigated ?? 0;

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="fixed bottom-4 right-4 z-40 w-[260px] rounded-xl border p-4 shadow-xl backdrop-blur"
      style={{
        background: "color-mix(in oklab, var(--bg-elevated) 92%, transparent)",
        borderColor: "var(--border)",
      }}
    >
      <div className="mb-3 text-[10px] font-semibold uppercase tracking-widest" style={{ color: "var(--text-muted)" }}>
        Real-Time ROI
      </div>
      <div className="space-y-3">
        <div className="flex items-start gap-2">
          <Clock3 className="mt-0.5 h-4 w-4" style={{ color: "var(--accent)" }} />
          <div>
            <div className="text-xs" style={{ color: "var(--text-muted)" }}>
              Engineering Hours Saved
            </div>
            <div className="font-mono text-xl font-semibold">{hours.toFixed(2)}h</div>
            <div className="text-[10px]" style={{ color: "var(--text-muted)" }}>
              vs {roi?.human_baseline_minutes ?? 45} min human baseline / intent
            </div>
          </div>
        </div>
        <div className="flex items-start gap-2">
          <ShieldCheck className="mt-0.5 h-4 w-4 text-amber-500" />
          <div>
            <div className="text-xs" style={{ color: "var(--text-muted)" }}>
              Risks Mitigated
            </div>
            <div className="font-mono text-xl font-semibold">{risks}</div>
            <div className="text-[10px]" style={{ color: "var(--text-muted)" }}>
              Dangerous commands blocked by Rust linter
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}
