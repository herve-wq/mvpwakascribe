import { useEffect, useRef } from "react";
import { useAppStore } from "../../stores/appStore";

export function WaveformDisplay() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const { audioLevel, recordingState } = useAppStore();
  const barsRef = useRef<number[]>(Array(64).fill(0));

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animationId: number;

    const draw = () => {
      const { width, height } = canvas;
      const barCount = barsRef.current.length;
      const barWidth = width / barCount;
      const maxBarHeight = height * 0.8;

      // Clear canvas
      ctx.clearRect(0, 0, width, height);

      // Update bars based on audio level
      if (recordingState === "recording") {
        // Shift bars left
        for (let i = 0; i < barCount - 1; i++) {
          barsRef.current[i] = barsRef.current[i + 1];
        }
        // Add new bar with some randomness for visual effect
        barsRef.current[barCount - 1] =
          audioLevel * (0.8 + Math.random() * 0.4);
      } else if (recordingState === "idle") {
        // Decay bars when not recording
        for (let i = 0; i < barCount; i++) {
          barsRef.current[i] *= 0.95;
        }
      }

      // Get CSS variable for accent color
      const accentColor =
        getComputedStyle(document.documentElement).getPropertyValue(
          "--color-accent"
        ) || "#3b82f6";

      // Draw bars
      ctx.fillStyle = accentColor;
      for (let i = 0; i < barCount; i++) {
        const barHeight = Math.max(2, barsRef.current[i] * maxBarHeight);
        const x = i * barWidth;
        const y = (height - barHeight) / 2;

        ctx.beginPath();
        ctx.roundRect(x + 1, y, barWidth - 2, barHeight, 2);
        ctx.fill();
      }

      animationId = requestAnimationFrame(draw);
    };

    draw();

    return () => {
      cancelAnimationFrame(animationId);
    };
  }, [audioLevel, recordingState]);

  // Handle canvas resize
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        canvas.width = width * window.devicePixelRatio;
        canvas.height = height * window.devicePixelRatio;
        const ctx = canvas.getContext("2d");
        if (ctx) {
          ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
        }
      }
    });

    resizeObserver.observe(canvas);
    return () => resizeObserver.disconnect();
  }, []);

  return (
    <div className="w-full h-24 bg-[var(--color-bg-tertiary)] rounded-lg overflow-hidden">
      <canvas
        ref={canvasRef}
        className="w-full h-full"
        style={{ width: "100%", height: "100%" }}
      />
    </div>
  );
}
