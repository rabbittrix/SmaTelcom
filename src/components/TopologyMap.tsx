import { useCallback, useRef, useState } from "react";
import { motion } from "framer-motion";
import { Minus, Plus, RotateCcw } from "lucide-react";
import type { HealthSnapshot, TopologyNode } from "../lib/types";

const VIEW_W = 680;
const VIEW_H = 500;
const MIN_ZOOM = 0.5;
const MAX_ZOOM = 3;
const ZOOM_STEP = 0.2;

function statusColor(status: string) {
  if (status === "critical") return "#ef4444";
  if (status === "warning" || status === "degraded") return "#f59e0b";
  return "#22c55e";
}

function clampZoom(z: number) {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));
}

export function TopologyMap({ health }: { health: HealthSnapshot | null }) {
  const nodes = health?.nodes ?? [];
  const links = health?.links ?? [];
  const byId = new Map(nodes.map((n) => [n.id, n]));

  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const dragRef = useRef<{
    active: boolean;
    startX: number;
    startY: number;
    originX: number;
    originY: number;
  } | null>(null);

  const zoomBy = useCallback((delta: number, center?: { x: number; y: number }) => {
    setZoom((prev) => {
      const next = clampZoom(prev + delta);
      if (center && next !== prev) {
        // Keep point under cursor stable when zooming
        const ratio = next / prev;
        setPan((p) => ({
          x: center.x - (center.x - p.x) * ratio,
          y: center.y - (center.y - p.y) * ratio,
        }));
      }
      return next;
    });
  }, []);

  const resetView = () => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  };

  const onWheel = (e: React.WheelEvent<SVGSVGElement>) => {
    e.preventDefault();
    const svg = e.currentTarget;
    const rect = svg.getBoundingClientRect();
    const sx = ((e.clientX - rect.left) / rect.width) * VIEW_W;
    const sy = ((e.clientY - rect.top) / rect.height) * VIEW_H;
    zoomBy(e.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP, { x: sx, y: sy });
  };

  const onPointerDown = (e: React.PointerEvent<SVGSVGElement>) => {
    if (e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    dragRef.current = {
      active: true,
      startX: e.clientX,
      startY: e.clientY,
      originX: pan.x,
      originY: pan.y,
    };
  };

  const onPointerMove = (e: React.PointerEvent<SVGSVGElement>) => {
    const d = dragRef.current;
    if (!d?.active) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const dx = ((e.clientX - d.startX) / rect.width) * VIEW_W;
    const dy = ((e.clientY - d.startY) / rect.height) * VIEW_H;
    setPan({ x: d.originX + dx, y: d.originY + dy });
  };

  const onPointerUp = (e: React.PointerEvent<SVGSVGElement>) => {
    if (dragRef.current) dragRef.current.active = false;
    e.currentTarget.releasePointerCapture(e.pointerId);
  };

  return (
    <div
      className="overflow-hidden rounded-xl border"
      style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
    >
      <div
        className="flex items-center justify-between gap-3 border-b px-4 py-3"
        style={{ borderColor: "var(--border)" }}
      >
        <div>
          <h3 className="text-sm font-semibold">Network Topology</h3>
          <p className="text-xs" style={{ color: "var(--text-muted)" }}>
            Live node state · scroll to zoom · drag to pan
          </p>
        </div>

        <div className="flex items-center gap-3">
          <div
            className="hidden gap-3 text-[10px] uppercase tracking-wider sm:flex"
            style={{ color: "var(--text-muted)" }}
          >
            <span className="flex items-center gap-1">
              <i className="inline-block h-2 w-2 rounded-full bg-green-500" /> ok
            </span>
            <span className="flex items-center gap-1">
              <i className="inline-block h-2 w-2 rounded-full bg-amber-500" /> warn
            </span>
            <span className="flex items-center gap-1">
              <i className="inline-block h-2 w-2 rounded-full bg-red-500" /> critical
            </span>
          </div>

          <div
            className="flex items-center gap-1 rounded-lg border p-0.5"
            style={{ borderColor: "var(--border)" }}
          >
            <ZoomBtn
              label="Zoom out"
              onClick={() => zoomBy(-ZOOM_STEP, { x: VIEW_W / 2, y: VIEW_H / 2 })}
              disabled={zoom <= MIN_ZOOM}
            >
              <Minus className="h-3.5 w-3.5" />
            </ZoomBtn>
            <span className="min-w-[3.25rem] text-center font-mono text-[11px] tabular-nums">
              {Math.round(zoom * 100)}%
            </span>
            <ZoomBtn
              label="Zoom in"
              onClick={() => zoomBy(ZOOM_STEP, { x: VIEW_W / 2, y: VIEW_H / 2 })}
              disabled={zoom >= MAX_ZOOM}
            >
              <Plus className="h-3.5 w-3.5" />
            </ZoomBtn>
            <ZoomBtn label="Reset view" onClick={resetView}>
              <RotateCcw className="h-3.5 w-3.5" />
            </ZoomBtn>
          </div>
        </div>
      </div>

      <svg
        viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
        className="h-[340px] w-full touch-none select-none"
        style={{ cursor: "grab" }}
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerUp}
      >
        <defs>
          <pattern id="topo-grid" width="24" height="24" patternUnits="userSpaceOnUse">
            <path
              d="M 24 0 L 0 0 0 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="0.4"
              opacity="0.15"
            />
          </pattern>
        </defs>
        <rect
          width={VIEW_W}
          height={VIEW_H}
          fill="url(#topo-grid)"
          style={{ color: "var(--text-muted)" }}
        />

        <g transform={`translate(${pan.x} ${pan.y}) scale(${zoom})`}>
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
                strokeWidth={(l.status === "up" ? 1.5 : 2.5) / zoom}
                strokeOpacity={0.65}
                strokeDasharray={l.status === "degraded" ? "6 4" : undefined}
              />
            );
          })}

          {nodes.map((n) => (
            <TopoNode key={n.id} node={n} zoom={zoom} />
          ))}
        </g>
      </svg>
    </div>
  );
}

function ZoomBtn({
  children,
  onClick,
  label,
  disabled,
}: {
  children: React.ReactNode;
  onClick: () => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className="inline-flex h-7 w-7 items-center justify-center rounded-md disabled:opacity-35"
      style={{ color: "var(--text-muted)" }}
    >
      {children}
    </button>
  );
}

function TopoNode({ node, zoom }: { node: TopologyNode; zoom: number }) {
  const color = statusColor(node.status);
  const inv = 1 / zoom;
  return (
    <motion.g initial={false} style={{ cursor: "pointer" }}>
      <title>
        {node.label} · {node.site} · {node.vendor} · CPU {node.cpu_pct.toFixed(0)}% · {node.status}
      </title>
      <circle
        cx={node.x}
        cy={node.y}
        r={18 * inv}
        fill="var(--bg)"
        stroke={color}
        strokeWidth={2.5 * inv}
      />
      <circle cx={node.x} cy={node.y} r={6 * inv} fill={color} opacity={0.9} />
      <text
        x={node.x}
        y={node.y + 32 * inv}
        textAnchor="middle"
        fontSize={10 * inv}
        fill="var(--text)"
        fontFamily="IBM Plex Mono, monospace"
      >
        {node.label}
      </text>
      <text
        x={node.x}
        y={node.y + 44 * inv}
        textAnchor="middle"
        fontSize={8 * inv}
        fill="var(--text-muted)"
        fontFamily="IBM Plex Sans, sans-serif"
      >
        {node.role} · {node.cpu_pct.toFixed(0)}%
      </text>
    </motion.g>
  );
}
