"use client";

interface ApyGaugeProps {
  value: number;
  label?: string;
}

/** Semi-circular APY gauge matching the reference staking panel. */
export function ApyGauge({ value, label = "APY" }: ApyGaugeProps) {
  const clamped = Math.min(100, Math.max(0, value));
  const radius = 52;
  const cx = 70;
  const cy = 70;
  const startAngle = 180;
  const endAngle = 0;
  const sweep = ((startAngle - endAngle) * clamped) / 100;

  const polar = (angleDeg: number) => {
    const rad = (angleDeg * Math.PI) / 180;
    return {
      x: cx + radius * Math.cos(rad),
      y: cy - radius * Math.sin(rad),
    };
  };

  const bgEnd = polar(endAngle);
  const bgStart = polar(startAngle);
  const fgEnd = polar(startAngle - sweep);

  const trackPath = `M ${bgStart.x} ${bgStart.y} A ${radius} ${radius} 0 0 1 ${bgEnd.x} ${bgEnd.y}`;
  const fillPath =
    sweep > 0
      ? `M ${bgStart.x} ${bgStart.y} A ${radius} ${radius} 0 ${sweep > 90 ? 1 : 0} 1 ${fgEnd.x} ${fgEnd.y}`
      : "";

  return (
    <div className="relative flex h-[100px] w-[140px] flex-col items-center">
      <svg viewBox="0 0 140 90" className="h-[90px] w-[140px] overflow-visible">
        <defs>
          <linearGradient id="apyGrad" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#22d3ee" />
            <stop offset="100%" stopColor="#06b6d4" />
          </linearGradient>
        </defs>
        <path
          d={trackPath}
          fill="none"
          stroke="#27272a"
          strokeWidth="10"
          strokeLinecap="round"
        />
        {fillPath ? (
          <path
            d={fillPath}
            fill="none"
            stroke="url(#apyGrad)"
            strokeWidth="10"
            strokeLinecap="round"
            className="drop-shadow-[0_0_6px_rgba(34,211,238,0.6)]"
          />
        ) : null}
      </svg>
      <div className="absolute bottom-0 text-center">
        <p className="text-2xl font-bold tabular-nums text-white">{value}%</p>
        <p className="text-[10px] font-medium uppercase tracking-wider text-zinc-500">
          {label}
        </p>
      </div>
    </div>
  );
}
