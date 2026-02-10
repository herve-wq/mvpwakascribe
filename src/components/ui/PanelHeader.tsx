import type { ReactNode } from "react";

interface PanelHeaderProps {
  title: string;
  onClose?: () => void;
  subtitle?: string;
  actions?: ReactNode;
}

export function PanelHeader({ title, onClose, subtitle, actions }: PanelHeaderProps) {
  return (
    <div className="p-4 border-b border-[var(--color-border)] flex items-center justify-between">
      <div>
        <h2 className="font-semibold text-[var(--color-text-primary)]">{title}</h2>
        {subtitle && (
          <p className="text-xs text-[var(--color-text-muted)]">{subtitle}</p>
        )}
      </div>
      <div className="flex items-center gap-2">
        {actions}
        {onClose && (
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-[var(--color-bg-tertiary)]"
          >
            <svg
              className="w-5 h-5 text-[var(--color-text-muted)]"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        )}
      </div>
    </div>
  );
}
