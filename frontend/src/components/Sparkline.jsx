import React from 'react';

export default function Sparkline({ data, color = '#60a5fa' }) {
  if (!data || data.length < 2) {
    return (
      <div className="w-full h-full flex items-center justify-center">
        <span className="text-[10px] text-slate-700 italic">awaiting data…</span>
      </div>
    );
  }

  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;
  const padding = 4;

  // Use viewBox for responsive scaling — SVG fills container
  const vw = 300;
  const vh = 80;

  const points = data.map((val, i) => {
    const x = padding + (i / (data.length - 1)) * (vw - padding * 2);
    const y = vh - padding - ((val - min) / range) * (vh - padding * 2);
    return `${x},${y}`;
  }).join(' ');

  // Area fill
  const firstX = padding;
  const lastX = padding + ((data.length - 1) / (data.length - 1)) * (vw - padding * 2);
  const areaPoints = `${firstX},${vh} ${points} ${lastX},${vh}`;

  // Unique gradient ID based on color
  const gradId = `grad-${color.replace('#', '')}`;

  return (
    <svg
      viewBox={`0 0 ${vw} ${vh}`}
      preserveAspectRatio="none"
      className="w-full h-full"
    >
      <defs>
        <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.25" />
          <stop offset="100%" stopColor={color} stopOpacity="0.02" />
        </linearGradient>
      </defs>
      <polygon
        points={areaPoints}
        fill={`url(#${gradId})`}
      />
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth="2.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
      />
      {/* End dot */}
      {data.length > 0 && (() => {
        const lastVal = data[data.length - 1];
        const cx = padding + ((data.length - 1) / (data.length - 1)) * (vw - padding * 2);
        const cy = vh - padding - ((lastVal - min) / range) * (vh - padding * 2);
        return <circle cx={cx} cy={cy} r="3" fill={color} opacity="0.8" />;
      })()}
    </svg>
  );
}
