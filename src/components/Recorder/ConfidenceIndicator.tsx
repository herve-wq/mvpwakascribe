interface ConfidenceIndicatorProps {
  confidence: number;
}

export function ConfidenceIndicator({ confidence }: ConfidenceIndicatorProps) {
  const percentage = Math.round(confidence * 100);
  const barCount = 10;
  const filledBars = Math.round(confidence * barCount);

  const getColor = () => {
    if (confidence >= 0.8) return "bg-green-500";
    if (confidence >= 0.6) return "bg-yellow-500";
    return "bg-red-500";
  };

  return (
    <div className="flex items-center gap-2 text-xs text-[var(--color-text-muted)]">
      <div className="flex gap-0.5">
        {Array.from({ length: barCount }).map((_, i) => (
          <div
            key={i}
            className={`w-1.5 h-3 rounded-sm ${
              i < filledBars ? getColor() : "bg-[var(--color-border)]"
            }`}
          />
        ))}
      </div>
      <span>{percentage}%</span>
    </div>
  );
}
