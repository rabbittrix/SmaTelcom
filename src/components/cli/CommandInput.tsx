import { forwardRef, useImperativeHandle, useRef } from "react";
import { Loader2, Terminal } from "lucide-react";

export type CommandInputHandle = {
  focus: () => void;
  clear: () => void;
};

type Props = {
  value: string;
  onChange: (value: string) => void;
  onSubmit: (value: string) => void | Promise<void>;
  loading?: boolean;
  placeholder?: string;
  /** Single-line CLI prompt (default) or multiline intent box */
  mode?: "cli" | "textarea";
  rows?: number;
  autoFocus?: boolean;
  disabled?: boolean;
  hint?: string;
};

/**
 * Controlled network-intent CLI. Enter submits (Shift+Enter inserts newline in textarea mode).
 * Cleared by parent after successful send; expose focus() for Ctrl+K.
 */
export const CommandInput = forwardRef<CommandInputHandle, Props>(function CommandInput(
  {
    value,
    onChange,
    onSubmit,
    loading = false,
    placeholder = "Enter network intent… (Enter to run)",
    mode = "cli",
    rows = 3,
    autoFocus = false,
    disabled = false,
    hint = "Enter ↵ run · Shift+Enter newline · Ctrl+K focus",
  },
  ref,
) {
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement>(null);

  useImperativeHandle(ref, () => ({
    focus: () => inputRef.current?.focus(),
    clear: () => onChange(""),
  }));

  const submit = () => {
    const trimmed = value.trim();
    if (!trimmed || loading || disabled) return;
    void onSubmit(trimmed);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    if (e.key !== "Enter") return;
    if (mode === "textarea" && e.shiftKey) return;
    e.preventDefault();
    e.stopPropagation();
    submit();
  };

  const sharedClass =
    "relative z-10 w-full resize-none rounded-lg border bg-transparent px-3 py-2.5 text-sm outline-none focus:ring-2 disabled:opacity-50";
  const sharedStyle = {
    borderColor: "var(--border)",
    color: "var(--text)",
    caretColor: "var(--accent)",
  } as const;

  return (
    <div className="relative z-10 space-y-2">
      <div
        className="flex items-center gap-2 rounded-xl border p-2"
        style={{
          background: "var(--bg-elevated)",
          borderColor: "var(--border)",
          boxShadow: loading ? "0 0 0 1px color-mix(in oklab, var(--accent) 40%, transparent)" : undefined,
        }}
      >
        <div
          className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg"
          style={{ background: "var(--accent-soft)", color: "var(--accent)" }}
          aria-hidden
        >
          {loading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Terminal className="h-4 w-4" />
          )}
        </div>

        <div className="min-w-0 flex-1">
          {mode === "cli" ? (
            <input
              ref={inputRef as React.RefObject<HTMLInputElement>}
              type="text"
              value={value}
              onChange={(e) => onChange(e.target.value)}
              onKeyDown={onKeyDown}
              autoFocus={autoFocus}
              disabled={disabled || loading}
              placeholder={placeholder}
              autoComplete="off"
              spellCheck={false}
              className={sharedClass}
              style={sharedStyle}
              aria-label="Network intent command"
            />
          ) : (
            <textarea
              ref={inputRef as React.RefObject<HTMLTextAreaElement>}
              value={value}
              onChange={(e) => onChange(e.target.value)}
              onKeyDown={onKeyDown}
              autoFocus={autoFocus}
              disabled={disabled || loading}
              placeholder={placeholder}
              rows={rows}
              spellCheck={false}
              className={sharedClass}
              style={sharedStyle}
              aria-label="Network intent"
            />
          )}
        </div>

        <button
          type="button"
          onClick={submit}
          disabled={disabled || loading || !value.trim()}
          className="shrink-0 rounded-lg px-3 py-2 text-xs font-semibold text-white disabled:opacity-40"
          style={{ background: "var(--accent)" }}
        >
          {loading ? "Thinking…" : "Run"}
        </button>
      </div>
      <p className="px-1 text-[10px]" style={{ color: "var(--text-muted)" }}>
        {loading ? "AI agents analyzing intent…" : hint}
      </p>
    </div>
  );
});
