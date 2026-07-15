import { motion, AnimatePresence } from "framer-motion";
import { BookOpen, FileText, X } from "lucide-react";
import { useState } from "react";
import type { DocumentChunk, PipelineResult } from "../../lib/types";

export function EvidencePanel({
  result,
}: {
  result: PipelineResult | null;
}) {
  const [open, setOpen] = useState<DocumentChunk | null>(null);
  const sources = result?.evidence_sources ?? [];
  const citation = result?.conflict_resolution?.policy_citation;

  if (!result) {
    return (
      <div
        className="rounded-xl border p-4 text-sm"
        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)", color: "var(--text-muted)" }}
      >
        Run a pipeline to see RAG policy evidence (“Show Your Source”).
      </div>
    );
  }

  return (
    <>
      <div
        className="rounded-xl border p-4"
        style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
      >
        <div className="mb-3 flex items-center gap-2">
          <BookOpen className="h-4 w-4" style={{ color: "var(--accent)" }} />
          <h3 className="text-sm font-semibold">Source of Truth (RAG)</h3>
        </div>
        {citation && (
          <p className="mb-3 text-xs leading-relaxed" style={{ color: "var(--text-muted)" }}>
            Judge citation: “{citation}”
          </p>
        )}
        {sources.length === 0 && (
          <p className="text-xs" style={{ color: "var(--text-muted)" }}>
            No knowledge_base chunks matched this intent.
          </p>
        )}
        <div className="space-y-2">
          {sources.map((s, i) => (
            <div
              key={`${s.source}-${i}`}
              className="flex items-start justify-between gap-3 rounded-lg border p-3"
              style={{ borderColor: "var(--border)" }}
            >
              <div className="min-w-0 flex-1">
                <div className="mb-1 flex items-center gap-1.5 text-xs font-semibold" style={{ color: "var(--accent)" }}>
                  <FileText className="h-3.5 w-3.5" />
                  {s.source}
                </div>
                <p className="truncate text-[11px]" style={{ color: "var(--text-muted)" }}>
                  {s.content.slice(0, 140)}…
                </p>
              </div>
              <button
                onClick={() => setOpen(s)}
                className="shrink-0 rounded-lg border px-2.5 py-1.5 text-[11px] font-medium"
                style={{ borderColor: "var(--border)", color: "var(--accent)" }}
              >
                View Source
              </button>
            </div>
          ))}
        </div>
      </div>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
            onClick={() => setOpen(null)}
          >
            <motion.div
              initial={{ scale: 0.96, y: 8 }}
              animate={{ scale: 1, y: 0 }}
              exit={{ scale: 0.96, opacity: 0 }}
              className="max-h-[80vh] w-full max-w-2xl overflow-hidden rounded-2xl border shadow-2xl"
              style={{ background: "var(--bg-elevated)", borderColor: "var(--border)" }}
              onClick={(e) => e.stopPropagation()}
            >
              <div
                className="flex items-center justify-between border-b px-5 py-3"
                style={{ borderColor: "var(--border)" }}
              >
                <div>
                  <h4 className="text-sm font-semibold">Policy Source</h4>
                  <p className="font-mono text-[11px]" style={{ color: "var(--accent)" }}>
                    knowledge_base/{open.source}
                  </p>
                </div>
                <button onClick={() => setOpen(null)} className="rounded-lg p-1.5" style={{ color: "var(--text-muted)" }}>
                  <X className="h-4 w-4" />
                </button>
              </div>
              <pre
                className="max-h-[60vh] overflow-auto whitespace-pre-wrap p-5 font-mono text-xs leading-relaxed"
                style={{ color: "var(--text-muted)" }}
              >
                {open.content}
              </pre>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
