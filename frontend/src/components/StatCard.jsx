import React, { useState, useEffect } from 'react';
import Sparkline from './Sparkline';
import { fetchStatHistory } from '../api';

const PERIODS = ['24h', '7d', '30d', '90d'];

// Contextual unit suffixes for metrics
const UNIT_MAP = {
  agents_discovered: 'agents',
  moltbook_interesting: 'items',
  moltbook_health: '', // binary — handled specially
  moltbook_my_posts: 'posts',
  moltbook_spam: 'posts',
  twitter_headlines: 'tweets',
  twitter_accounts: 'accounts',
  outreach_sent: 'sent',
  outreach_received: 'received',
  repos_count: 'repos',
  commits_total: 'commits',
  tests_total: 'tests',
  siblings_active: 'online',
  siblings_count: 'agents',
};

// Binary metrics: show status text instead of numbers
const BINARY_METRICS = {
  moltbook_health: {
    on: { text: 'Healthy', color: 'text-emerald-400', dot: 'bg-emerald-500' },
    off: { text: 'Down', color: 'text-red-400', dot: 'bg-red-500' },
  },
};

function isBinaryMetric(key) {
  return key in BINARY_METRICS;
}

function getBinaryDisplay(key, value) {
  const config = BINARY_METRICS[key];
  if (!config) return null;
  return value >= 1 ? config.on : config.off;
}

function getUnit(key) {
  return UNIT_MAP[key] || '';
}

function formatNumber(n) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
  if (n >= 10_000) return (n / 1_000).toFixed(0) + 'K';
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
  if (Number.isInteger(n)) return n.toLocaleString();
  return n.toFixed(1);
}

function timeAgo(isoStr) {
  if (!isoStr) return '—';
  try {
    const d = new Date(isoStr);
    const now = Date.now();
    const diffMs = now - d.getTime();
    if (diffMs < 0) return 'just now';
    const mins = Math.floor(diffMs / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    return `${days}d ago`;
  } catch {
    return '—';
  }
}

// Trend alert thresholds
const ALERT_THRESHOLD = 10;  // ±10% → alert (pulsing dot)
const HOT_THRESHOLD = 25;    // ±25% → hot (glow border + pulse)

function getTrendAlertLevel(stat) {
  // Always check the 24h trend for alerts (most recent signal)
  const trend24h = stat.trends?.['24h'];
  if (!trend24h || trend24h.pct === null || trend24h.pct === undefined) return null;
  const absPct = Math.abs(trend24h.pct);
  if (absPct >= HOT_THRESHOLD) return 'hot';
  if (absPct >= ALERT_THRESHOLD) return 'alert';
  return null;
}

function getTrendDirection(stat) {
  const trend24h = stat.trends?.['24h'];
  if (!trend24h || !trend24h.change) return 'neutral';
  return trend24h.change > 0 ? 'up' : 'down';
}

function TrendBadge({ trend, period }) {
  if (!trend || trend.change === null || trend.change === undefined) {
    return <span className="text-[11px] text-slate-600 italic">no data yet</span>;
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

export default function StatCard({ stat, onClick }) {
  const [period, setPeriod] = useState('24h');
  const [periodData, setPeriodData] = useState(null);
  const [loading, setLoading] = useState(false);

  const trend = stat.trends?.[period];
  const binary = getBinaryDisplay(stat.key, stat.current);
  const sparkColor = trend?.change > 0 ? '#34d399' : trend?.change < 0 ? '#f87171' : '#60a5fa';

  // Trend alert detection
  const alertLevel = getTrendAlertLevel(stat);
  const alertDir = getTrendDirection(stat);

  // Status indicator dot color
  const dotColor = binary
    ? binary.dot
    : trend?.change > 0 ? 'bg-emerald-500' : trend?.change < 0 ? 'bg-red-500' : 'bg-slate-600';

  // Alert classes
  const dotAlert = alertLevel ? 'alert-dot' : '';
  const cardGlow = alertLevel === 'hot'
    ? (alertDir === 'up' ? 'alert-glow-up' : alertDir === 'down' ? 'alert-glow-down' : '')
    : '';
  const borderAlert = alertLevel === 'hot'
    ? (alertDir === 'up' ? 'border-emerald-800/40' : alertDir === 'down' ? 'border-red-800/40' : '')
    : alertLevel === 'alert'
    ? (alertDir === 'up' ? 'border-emerald-900/30' : alertDir === 'down' ? 'border-red-900/30' : '')
    : '';

  // Fetch per-period sparkline data when period changes
  useEffect(() => {
    if (period === '24h') {
      setPeriodData(null); // Use default sparkline_24h
      return;
    }
    let cancelled = false;
    setLoading(true);
    fetchStatHistory(stat.key, period).then(data => {
      if (!cancelled && data.points) {
        setPeriodData(data.points);
      }
      setLoading(false);
    }).catch(() => {
      if (!cancelled) setLoading(false);
    });
    return () => { cancelled = true; };
  }, [stat.key, period]);

  // Determine sparkline data based on period
  let sparkData, sparkTimestamps;
  if (period === '24h' || !periodData) {
    sparkData = stat.sparkline_24h;
    sparkTimestamps = null; // No timestamps for default sparkline
  } else {
    // Downsample to 24 points max for readability
    const maxPts = 24;
    if (periodData.length <= maxPts) {
      sparkData = periodData.map(p => p.value);
      sparkTimestamps = periodData.map(p => p.recorded_at);
    } else {
      const step = periodData.length / maxPts;
      sparkData = [];
      sparkTimestamps = [];
      for (let i = 0; i < maxPts; i++) {
        const idx = Math.min(Math.floor(i * step), periodData.length - 1);
        sparkData.push(periodData[idx].value);
        sparkTimestamps.push(periodData[idx].recorded_at);
      }
    }
  }

  return (
    <div
      className={`bg-slate-900/80 border rounded-xl px-3 sm:px-4 pt-3 pb-2 flex flex-col min-h-[140px] lg:min-h-0 hover:border-slate-700/60 transition-colors cursor-pointer ${borderAlert || 'border-slate-800/60'} ${cardGlow}`}
      onClick={onClick}
    >
      {/* Header row */}
      <div className="flex items-center justify-between flex-shrink-0 mb-1">
        <div className="flex items-center gap-2 min-w-0">
          <div className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${dotColor} ${dotAlert}`} />
          <h3 className="text-[11px] sm:text-xs font-medium text-slate-400 uppercase tracking-wider truncate">
            {stat.label}
          </h3>
        </div>
        {/* Period selector — touch-friendly on mobile */}
        <div className="flex gap-0.5 sm:gap-0.5 flex-shrink-0">
          {PERIODS.map(p => (
            <button
              key={p}
              onClick={() => setPeriod(p)}
              className={`period-btn text-[10px] sm:text-[10px] px-1.5 sm:px-1.5 py-1 sm:py-0.5 rounded transition-colors ${
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

      {/* Big number + unit + trend */}
      <div className="flex items-baseline gap-2 sm:gap-3 flex-shrink-0">
        {binary ? (
          <span className={`text-2xl sm:text-3xl font-bold leading-tight ${binary.color}`}>
            {binary.text}
          </span>
        ) : (
          <>
            <span className="text-2xl sm:text-3xl font-bold text-white tabular-nums leading-tight">
              {formatNumber(stat.current)}
            </span>
            {getUnit(stat.key) && (
              <span className="text-xs sm:text-sm text-slate-500 font-medium -ml-1">
                {getUnit(stat.key)}
              </span>
            )}
          </>
        )}
        {alertLevel === 'hot' && (
          <span className="text-[11px] opacity-80" title={`${Math.abs(stat.trends?.['24h']?.pct || 0).toFixed(0)}% change in 24h`}>⚡</span>
        )}
        <TrendBadge trend={trend} period={period} />
      </div>

      {/* Sparkline — fills remaining space, edge-to-edge */}
      <div className="flex-1 mt-1 -mx-3 sm:-mx-4 min-h-[40px] lg:min-h-0 relative">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center z-10">
            <span className="text-[10px] text-slate-600">loading…</span>
          </div>
        )}
        <Sparkline data={sparkData} color={sparkColor} timestamps={sparkTimestamps} />
      </div>

      {/* Last updated — relative time */}
      <div className="text-[10px] text-slate-700 mt-0.5 flex-shrink-0">
        {timeAgo(stat.last_updated)}
      </div>
    </div>
  );
}
