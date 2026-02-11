import React, { useState } from 'react';
import Sparkline from './Sparkline';

const PERIODS = ['24h', '7d', '30d', '90d'];

function formatNumber(n) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
  if (n >= 10_000) return (n / 1_000).toFixed(0) + 'K';
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
  if (Number.isInteger(n)) return n.toLocaleString();
  return n.toFixed(1);
}

function TrendBadge({ trend, period }) {
  if (!trend || trend.change === null || trend.change === undefined) {
    return <span className="text-[11px] text-slate-600 italic">collecting…</span>;
  }

  const isUp = trend.change > 0;
  const isDown = trend.change < 0;

  const color = isUp ? 'text-emerald-400' : isDown ? 'text-red-400' : 'text-slate-500';
  const arrow = isUp ? '↑' : isDown ? '↓' : '→';
  const pct = trend.pct !== null && trend.pct !== undefined
    ? `${Math.abs(trend.pct).toFixed(1)}%`
    : '';

  return (
    <span className={`text-[11px] font-medium ${color}`}>
      {arrow} {formatNumber(Math.abs(trend.change))}
      {pct && <span className="text-slate-500 ml-1">({pct})</span>}
      <span className="text-slate-600 ml-1">{period}</span>
    </span>
  );
}

export default function StatCard({ stat }) {
  const [period, setPeriod] = useState('24h');
  const trend = stat.trends?.[period];
  const sparkColor = trend?.change > 0 ? '#34d399' : trend?.change < 0 ? '#f87171' : '#60a5fa';

  // Status indicator dot color
  const dotColor = stat.current === 0 && stat.key.includes('health')
    ? 'bg-red-500'
    : trend?.change > 0 ? 'bg-emerald-500' : trend?.change < 0 ? 'bg-red-500' : 'bg-slate-600';

  return (
    <div className="bg-slate-900/80 border border-slate-800/60 rounded-xl p-4 flex flex-col min-h-0 hover:border-slate-700/60 transition-colors">
      {/* Header row */}
      <div className="flex items-center justify-between flex-shrink-0 mb-1">
        <div className="flex items-center gap-2">
          <div className={`w-1.5 h-1.5 rounded-full ${dotColor}`} />
          <h3 className="text-xs font-medium text-slate-400 uppercase tracking-wider">
            {stat.label}
          </h3>
        </div>
        {/* Period selector — inline */}
        <div className="flex gap-0.5">
          {PERIODS.map(p => (
            <button
              key={p}
              onClick={() => setPeriod(p)}
              className={`text-[10px] px-1.5 py-0.5 rounded transition-colors ${
                p === period
                  ? 'bg-slate-700 text-white'
                  : 'text-slate-600 hover:text-slate-400'
              }`}
            >
              {p}
            </button>
          ))}
        </div>
      </div>

      {/* Big number + trend */}
      <div className="flex items-baseline gap-3 flex-shrink-0">
        <span className="text-3xl font-bold text-white tabular-nums leading-tight">
          {formatNumber(stat.current)}
        </span>
        <TrendBadge trend={trend} period={period} />
      </div>

      {/* Sparkline — fills remaining space */}
      <div className="flex-1 mt-2 min-h-0">
        <Sparkline data={stat.sparkline_24h} color={sparkColor} />
      </div>

      {/* Last updated */}
      <div className="text-[10px] text-slate-700 mt-1 flex-shrink-0">
        {stat.last_updated
          ? new Date(stat.last_updated).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
          : '—'}
      </div>
    </div>
  );
}
