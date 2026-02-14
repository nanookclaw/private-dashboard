import React, { useState, useEffect, useRef } from 'react';

function timeAgo(isoStr) {
  if (!isoStr) return '—';
  try {
    const d = new Date(isoStr);
    const diffMs = Date.now() - d.getTime();
    if (diffMs < 0) return 'just now';
    const mins = Math.floor(diffMs / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    return `${days}d ago`;
  } catch { return '—'; }
}

function formatTimestamp(isoStr) {
  try {
    const d = new Date(isoStr);
    return d.toLocaleDateString([], { month: 'short', day: 'numeric' }) + ' ' +
           d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch { return '—'; }
}

export default function AlertHistory({ alerts, loading, onRefresh }) {
  const [expanded, setExpanded] = useState(false);

  if (!alerts || alerts.length === 0) {
    if (loading) return null;
    return null; // Don't show panel if no alerts
  }

  const displayAlerts = expanded ? alerts : alerts.slice(0, 5);

  return (
    <div className="flex-shrink-0 mt-2 sm:mt-3">
      {/* Header */}
      <div className="flex items-center gap-2 mb-1.5">
        <span className="text-[11px] sm:text-xs">🔔</span>
        <span className="text-[10px] sm:text-[11px] font-medium text-slate-600 uppercase tracking-widest">
          Recent Alerts
        </span>
        <div className="flex-1 h-px bg-slate-800/60" />
        {alerts.length > 5 && (
          <button
            onClick={() => setExpanded(!expanded)}
            className="text-[10px] text-slate-500 hover:text-slate-300 transition-colors"
          >
            {expanded ? 'Show less' : `+${alerts.length - 5} more`}
          </button>
        )}
      </div>

      {/* Alert list */}
      <div className="space-y-1">
        {displayAlerts.map((alert, i) => {
          const isHot = alert.level === 'hot';
          const isUp = alert.change_pct > 0;

          const levelBg = isHot
            ? (isUp ? 'bg-emerald-950/40 border-emerald-800/30' : 'bg-red-950/40 border-red-800/30')
            : (isUp ? 'bg-slate-900/60 border-emerald-900/20' : 'bg-slate-900/60 border-red-900/20');

          const pctColor = isUp ? 'text-emerald-400' : 'text-red-400';
          const arrow = isUp ? '↑' : '↓';
          const icon = isHot ? '⚡' : '•';

          return (
            <div
              key={i}
              className={`flex items-center gap-2 sm:gap-3 px-2.5 sm:px-3 py-1.5 rounded-lg border ${levelBg} transition-colors`}
            >
              {/* Level icon */}
              <span className={`text-[11px] flex-shrink-0 ${isHot ? '' : pctColor}`}>
                {icon}
              </span>

              {/* Metric label */}
              <span className="text-[11px] sm:text-xs text-slate-300 font-medium truncate min-w-0">
                {alert.label}
              </span>

              {/* Change badge */}
              <span className={`text-[10px] sm:text-[11px] font-semibold ${pctColor} flex-shrink-0`}>
                {arrow} {Math.abs(alert.change_pct).toFixed(1)}%
              </span>

              {/* Value */}
              <span className="text-[10px] text-slate-500 flex-shrink-0 hidden sm:inline">
                @ {Number.isInteger(alert.value) ? alert.value.toLocaleString() : alert.value.toFixed(1)}
              </span>

              {/* Time */}
              <span className="text-[10px] text-slate-600 ml-auto flex-shrink-0" title={formatTimestamp(alert.triggered_at)}>
                {timeAgo(alert.triggered_at)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
