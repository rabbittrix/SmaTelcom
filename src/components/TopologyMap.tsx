import { motion } from "framer-motion";
import type { HealthSnapshot, TopologyNode } from "../lib/types";

function statusColor(status: string) {
  if (status === "critical") return "#ef4444";
  if (status === "warning" || status === "degraded") return "#f59e0b";
  return "#22c55e";
}

export function TopologyMap({ health }: { health: HealthSnapshot | null }) {
  const nodes = health?.nodes ?? [];
  const links = health?.links ?? [];
  const byId = new Map(nodes.map((n) => [n.id, n]));

  return (
    <div
      className="overflow-hidden rounded-xl border"
      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
    >
      <div className="flex items-center justify-between border-b px-4 py-3" style={{ borderColor: "var(--border)" }}>
        <div>
          <h3 className="text-sm font-semibold">Network Topology</h3>
          <p className="text-xs" style={{ color: "var(--text-muted)" }}>
            Live node state from Rust telemetry
          </p>
        </div>
        <div className="flex gap-3 text-[10px] uppercase tracking-wider" style={{ color: "var(--text-muted)" }}>
          <span className="flex items-center gap-1"><i className="inline-block h-2 w-2 rounded-full bg-green-500" /> ok</span>
          <span className="flex items-center gap-1"><i className="inline-block h-2 w-2 rounded-full bg-amber-500" /> warn</span>
          <span className="flex items-center gap-1"><i className="inline-block h-2 w-2 rounded-full bg-red-500" /> critical</span>
        </div>
      </div>

      <svg viewBox="0 0 680 500" className="h-[340px] w-full">
        <defs>
          <pattern id="topo-grid" width="24" height="24" patternUnits="userSpaceOnUse">
            <path d="M 24 0 L 0 0 0 24" fill="none" stroke="currentColor" strokeWidth="0.4" opacity="0.15" />
          </pattern>
        </defs>
        <rect width="680" height="500" fill="url(#topo-grid)" style={{ color: "var(--text-muted)" }} />

        {links.map((l) => {
          const a = byId.get(l.source);
          const b = byId.get(l.target);
          if (!a || !b) return null;
          return (
            <line
              key={`${l.source}-${l.target}`}
              x1={a.x}
              y1={a.y}
              x2={b.x}
              y2={b.y}
              stroke={statusColor(l.status)}
              strokeWidth={l.status === "up" ? 1.5 : 2.5}
              strokeOpacity={0.65}
              strokeDasharray={l.status === "degraded" ? "6 4" : undefined}
            />
          );
        })}

        {nodes.map((n) => (
          <TopoNode key={n.id} node={n} />
        ))}
      </svg>
    </div>
  );
}

function TopoNode({ node }: { node: TopologyNode }) {
  const color = statusColor(node.status);
  return (
    <motion.g
      initial={false}
      animate={{ x: 0 }}
      style={{ cursor: "pointer" }}
    >
      <title>
        {node.label} · {node.site} · {node.vendor} · CPU {node.cpu_pct.toFixed(0)}% · {node.status}
      </title>
      <circle
        cx={node.x}
        cy={node.y}
        r={18}
        fill="var(--bg)"
        stroke={color}
        strokeWidth={2.5}
      />
      <circle cx={node.x} cy={node.y} r={6} fill={color} opacity={0.9} />
      <text
        x={node.x}
        y={node.y + 32}
        textAnchor="middle"
        fontSize="10"
        fill="var(--text)"
        fontFamily="IBM Plex Mono, monospace"
      >
        {node.label}
      </text>
      <text
        x={node.x}
        y={node.y + 44}
        textAnchor="middle"
        fontSize="8"
        fill="var(--text-muted)"
        fontFamily="IBM Plex Sans, sans-serif"
      >
        {node.role} · {node.cpu_pct.toFixed(0)}%
      </text>
    </motion.g>
  );
}
