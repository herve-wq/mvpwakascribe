import type { Segment } from "../../lib/types";
import { ConfidenceIndicator } from "../Recorder/ConfidenceIndicator";

interface SegmentListProps {
  segments: Segment[];
  onSegmentClick?: (segment: Segment) => void;
}

function formatTimestamp(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

export function SegmentList({ segments, onSegmentClick }: SegmentListProps) {
  return (
    <div className="space-y-3">
      {segments.map((segment) => (
        <div
          key={segment.id}
          onClick={() => onSegmentClick?.(segment)}
          className={`p-3 rounded-lg bg-[var(--color-bg-tertiary)] ${
            onSegmentClick ? "cursor-pointer hover:bg-[var(--color-border)]" : ""
          }`}
        >
          <div className="flex items-start gap-3">
            <span className="text-xs text-[var(--color-text-muted)] font-mono shrink-0">
              [{formatTimestamp(segment.startMs)}]
            </span>
            <p className="flex-1 text-[var(--color-text-primary)]">{segment.text}</p>
          </div>
          <div className="mt-2 pl-14">
            <ConfidenceIndicator confidence={segment.confidence} />
          </div>
        </div>
      ))}
    </div>
  );
}
