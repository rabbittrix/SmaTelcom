import { motion } from "framer-motion";
import {
  Activity,
  BarChart3,
  BookOpen,
  ClipboardList,
  FileSearch,
  Gauge,
  GitBranch,
  Moon,
  Network,
  Shield,
  Sun,
  Radio,
  Cable,
  Terminal,
} from "lucide-react";
import { useTheme } from "../../hooks/useTheme";

const nav = [
  { id: "dashboard", label: "Dashboard", icon: Gauge },
  { id: "pipeline", label: "Intent Pipeline", icon: Network },
  { id: "topology", label: "Topology Map", icon: GitBranch },
  { id: "vendors", label: "Vendor Adapters", icon: Cable },
  { id: "drivers", label: "Northbound", icon: Radio },
  { id: "console", label: "Live Console", icon: Terminal },
  { id: "evidence", label: "Evidence", icon: FileSearch },
  { id: "audit", label: "Audit Trail", icon: ClipboardList },
  { id: "impact", label: "Impact Report", icon: BarChart3 },
  { id: "safety", label: "Safety Linter", icon: Shield },
  { id: "knowledge", label: "Knowledge Base", icon: BookOpen },
  { id: "activity", label: "Activity Log", icon: Activity },
] as const;

export type NavId = (typeof nav)[number]["id"];

export function Sidebar({
  active,
  onNavigate,
  ollamaOk,
}: {
  active: NavId;
  onNavigate: (id: NavId) => void;
  ollamaOk: boolean | null;
}) {
  const { theme, toggle } = useTheme();

  return (
    <aside
      className="flex h-full w-60 shrink-0 flex-col border-r"
      style={{ background: "var(--bg-sidebar)", borderColor: "var(--border)" }}
    >
      <div className="flex items-center gap-3 px-5 py-6">
        <img src="/logo.png" alt="SmarTelcom" className="h-10 w-10 rounded-lg" />
        <div>
          <div className="text-lg font-semibold tracking-tight">SmarTelcom</div>
          <div className="text-xs" style={{ color: "var(--text-muted)" }}>
            AN Level 4 Orchestrator
          </div>
        </div>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-3">
        {nav.map(({ id, label, icon: Icon }) => {
          const selected = active === id;
          return (
            <button
              key={id}
              onClick={() => onNavigate(id)}
              className="relative flex items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm font-medium transition-colors"
              style={{
                color: selected ? "var(--accent)" : "var(--text-muted)",
                background: selected ? "var(--accent-soft)" : "transparent",
              }}
            >
              {selected && (
                <motion.span
                  layoutId="nav-pill"
                  className="pointer-events-none absolute inset-0 rounded-lg"
                  style={{ background: "var(--accent-soft)" }}
                  transition={{ type: "spring", stiffness: 380, damping: 30 }}
                />
              )}
              <Icon className="relative z-10 h-4 w-4 shrink-0" />
              <span className="relative z-10 truncate">{label}</span>
            </button>
          );
        })}
      </nav>

      <div className="space-y-3 border-t px-4 py-4" style={{ borderColor: "var(--border)" }}>
        <div className="flex items-center justify-between text-xs">
          <span className="flex items-center gap-2" style={{ color: "var(--text-muted)" }}>
            <Radio className="h-3.5 w-3.5" />
            Ollama
          </span>
          <span
            className="rounded px-2 py-0.5 font-mono"
            style={{
              background: ollamaOk ? "rgba(34,197,94,0.15)" : "rgba(239,68,68,0.15)",
              color: ollamaOk ? "#22c55e" : "#ef4444",
            }}
          >
            {ollamaOk === null ? "…" : ollamaOk ? "online" : "offline"}
          </span>
        </div>
        <button
          onClick={toggle}
          className="flex w-full items-center justify-between rounded-lg border px-3 py-2 text-sm"
          style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}
        >
          <span>{theme === "dark" ? "Dark" : "Light"} theme</span>
          {theme === "dark" ? <Moon className="h-4 w-4" /> : <Sun className="h-4 w-4" />}
        </button>
        <p className="text-[10px] leading-relaxed" style={{ color: "var(--text-muted)" }}>
          Roberto de Souza · rabbittrix@hotmail.com
        </p>
      </div>
    </aside>
  );
}
