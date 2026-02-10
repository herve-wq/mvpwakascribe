interface RangeSliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  formatValue?: (value: number) => string;
  minLabel?: string;
  maxLabel?: string;
}

export function RangeSlider({
  label,
  value,
  min,
  max,
  step,
  onChange,
  formatValue,
  minLabel,
  maxLabel,
}: RangeSliderProps) {
  const displayValue = formatValue ? formatValue(value) : String(value);

  return (
    <div className="space-y-2">
      <div className="flex justify-between">
        <label className="text-sm text-[var(--color-text-secondary)]">
          {label}
        </label>
        <span className="text-sm text-[var(--color-text-muted)]">
          {displayValue}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="w-full accent-[var(--color-accent)]"
      />
      {(minLabel || maxLabel) && (
        <div className="flex justify-between text-xs text-[var(--color-text-muted)]">
          <span>{minLabel}</span>
          <span>{maxLabel}</span>
        </div>
      )}
    </div>
  );
}
