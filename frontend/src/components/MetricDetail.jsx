import React, { useState, useEffect, useRef, useCallback } from 'react';
import { fetchStatHistory } from '../api';

const PERIODS = ['24h', '7d', '30d', '90d'];

// Reuse from StatCard
const UNIT_MAP = {
  agents_discovered: 'agents',
  moltbook_interesting: 'items',
  moltbook_health: '',
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

const BINARY_METRICS = {
  moltbook_health: {
    on: { text: 'Healthy', color: 'text-emerald-400' },
    off: { text: 'Down', color: 'text-red-400' },
  },
};

function formatNumber(n) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
  if (n >= 10_000) return (n / 1_000).toFixed(0) + 'K';
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
  if (Number.isInteger(n)) return n.toLocaleString();
  return n.toFixed(1);
}

function formatFull(n) {
  if (Number.isInteger(n)) return n.toLocaleString();
  return n.toFixed(2);
}

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
    return d.toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' }) + ' ' +
           d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch { return '—'; }
}

function DetailChart({ data, timestamps, color }) {
  const [hover, setHover] = useState(null);
  const svgRef = useRef(null);

  if (!data || data.length < 2) {
    return (
      <div className="w-full h-full flex items-center justify-center">
        <span className="text-sm text-slate-600 italic">Not enough data for chart</span>
      </div>
    );
  }

  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;

  const vw = 600;
  const vh = 200;
  const padX = 4;
  const padY = 4;

  const getCoords = (val, i) => {
    const x = padX + (i / (data.length - 1)) * (vw - padX * 2);
    const y = vh - padY - ((val - min) / range) * (vh - padY * 2);
    return { x, y };
  };

  const points = data.map((val, i) => {
    const { x, y } = getCoords(val, i);
    return `${x},${y}`;
  }).join(' ');

  const firstX = padX;
  const lastX = vw - padX;
  const areaPoints = `${firstX},${vh} ${points} ${lastX},${vh}`;
  const gradId = `detail-grad-${color.replace('#', '')}`;

  const handleMouseMove = useCallback((e) => {
    const svg = svgRef.current;
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    const mouseX = ((e.clientX - rect.left) / rect.width) * vw;
    let nearest = 0;
    let minDist = Infinity;
    for (let i = 0; i < data.length; i++) {
      const { x } = getCoords(data[i], i);
      const dist = Math.abs(x - mouseX);
      if (dist < minDist) {
        minDist = dist;
        nearest = i;
      }
    }
    const { x, y } = getCoords(data[nearest], nearest);
    setHover({ idx: nearest, x, y, value: data[nearest] });
  }, [data]);

  const handleMouseLeave = useCallback(() => setHover(null), []);

  const fmtTime = (idx) => {
    if (!timestamps || !timestamps[idx]) return '';
    return formatTimestamp(timestamps[idx]);
  };

  const lastCoords = getCoords(data[data.length - 1], data.length - 1);

  // Y-axis labels
  const yLabels = [min, min + range * 0.25, min + range * 0.5, min + range * 0.75, max];

  return (
    <div className="relative w-full h-full">
      {/* Y-axis labels */}
      <div className="absolute left-0 top-0 bottom-0 w-12 flex flex-col justify-between py-1 pointer-events-none">
        {yLabels.slice().reverse().map((v, i) => (
          <span key={i} className="text-[9px] text-slate-600 text-right pr-1 leading-none">
            {formatNumber(v)}
          </span>
        ))}
      </div>
      <div className="ml-12 h-full">
        <svg
          ref={svgRef}
          viewBox={`0 0 ${vw} ${vh}`}
          preserveAspectRatio="none"
          className="w-full h-full cursor-crosshair"
          onMouseMove={handleMouseMove}
          onMouseLeave={handleMouseLeave}
        >
          <defs>
            <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity="0.3" />
              <stop offset="100%" stopColor={color} stopOpacity="0.03" />
            </linearGradient>
          </defs>

          {/* Grid lines */}
          {[0.25, 0.5, 0.75].map(pct => (
            <line
              key={pct}
              x1={padX} y1={padY + (1 - pct) * (vh - padY * 2)}
              x2={vw - padX} y2={padY + (1 - pct) * (vh - padY * 2)}
              stroke="#1e293b" strokeWidth="1"
              vectorEffect="non-scaling-stroke"
            />
          ))}

          <polygon points={areaPoints} fill={`url(#${gradId})`} />
          <polyline
            points={points}
            fill="none"
            stroke={color}
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            vectorEffect="non-scaling-stroke"
          />

          {/* Data point dots (if few enough) */}
          {data.length <= 48 && data.map((val, i) => {
            const { x, y } = getCoords(val, i);
            return (
              <circle key={i} cx={x} cy={y} r="2.5" fill={color} opacity="0.4" />
            );
          })}

          {/* End dot */}
          <circle cx={lastCoords.x} cy={lastCoords.y} r="4" fill={color} opacity="0.9" />

          {/* Hover */}
          {hover && (
            <>
              <line
                x1={hover.x} y1={0} x2={hover.x} y2={vh}
                stroke="#475569" strokeWidth="1" strokeDasharray="4,3"
                vectorEffect="non-scaling-stroke"
              />
              <line
                x1={0} y1={hover.y} x2={vw} y2={hover.y}
                stroke="#475569" strokeWidth="0.5" strokeDasharray="4,3"
                vectorEffect="non-scaling-stroke"
              />
              <circle cx={hover.x} cy={hover.y} r="5" fill={color} stroke="#fff" strokeWidth="2"
                vectorEffect="non-scaling-stroke" />
              <rect
                x={hover.x < vw / 2 ? hover.x + 10 : hover.x - 130}
                y={Math.max(2, Math.min(hover.y - 32, vh - 40))}
                width="120" height="36"
                rx="6" fill="#0f172a" fillOpacity="0.97" stroke="#334155" strokeWidth="0.5"
              />
              <text
                x={hover.x < vw / 2 ? hover.x + 18 : hover.x - 122}
                y={Math.max(2, Math.min(hover.y - 32, vh - 40)) + 15}
                fill="#f1f5f9" fontSize="12" fontWeight="700" fontFamily="system-ui"
              >
                {formatFull(hover.value)}
              </text>
              <text
                x={hover.x < vw / 2 ? hover.x + 18 : hover.x - 122}
                y={Math.max(2, Math.min(hover.y - 32, vh - 40)) + 29}
                fill="#94a3b8" fontSize="8" fontFamily="system-ui"
              >
                {fmtTime(hover.idx)}
              </text>
            </>
          )}
        </svg>
      </div>
    </div>
  );
}

export default function MetricDetail({ stat, onClose }) {
  const [period, setPeriod] = useState('7d');
  const [historyData, setHistoryData] = useState(null);
  const [loading, setLoading] = useState(true);
  const modalRef = useRef(null);

  const binary = BINARY_METRICS[stat.key];
  const binaryDisplay = binary ? (stat.current >= 1 ? binary.on : binary.off) : null;
  const unit = UNIT_MAP[stat.key] || '';

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    fetchStatHistory(stat.key, period).then(data => {
      if (!cancelled) {
        setHistoryData(data.points || []);
        setLoading(false);
      }
    }).catch(() => {
      if (!cancelled) {
        setHistoryData([]);
        setLoading(false);
      }
    });
    return () => { cancelled = true; };
  }, [stat.key, period]);

  // Close on Escape
  useEffect(() => {
    const handler = (e) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [onClose]);

  // Close on backdrop click
  const handleBackdrop = (e) => {
    if (e.target === modalRef.current) onClose();
  };

  const chartData = historyData ? historyData.map(p => p.value) : [];
  const chartTimestamps = historyData ? historyData.map(p => p.recorded_at) : [];

  const trendColor = (trend) => {
    if (!trend || trend.change === null || trend.change === undefined) return 'text-slate-600';
    return trend.change > 0 ? 'text-emerald-400' : trend.change < 0 ? 'text-red-400' : 'text-slate-500';
  };

  const trendArrow = (trend) => {
    if (!trend || trend.change === null || trend.change === undefined) return '—';
    return trend.change > 0 ? '↑' : trend.change < 0 ? '↓' : '→';
  };

  // Determine color for chart
  const activeTrend = stat.trends?.[period];
  const chartColor = activeTrend?.change > 0 ? '#34d399' : activeTrend?.change < 0 ? '#f87171' : '#60a5fa';

  return (
    <div
      ref={modalRef}
      className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={handleBackdrop}
    >
      <div className="bg-slate-900 border border-slate-700/60 rounded-2xl w-full max-w-2xl max-h-[90vh] overflow-y-auto shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between p-4 sm:p-5 border-b border-slate-800/60">
          <div>
            <h2 className="text-sm sm:text-base font-semibold text-slate-200 uppercase tracking-wider">
              {stat.label}
            </h2>
            <p className="text-[11px] text-slate-600 mt-0.5 font-mono">{stat.key}</p>
          </div>
          <button
            onClick={onClose}
            className="text-slate-500 hover:text-white transition-colors p-1 rounded-lg hover:bg-slate-800"
          >
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M5 5l10 10M15 5L5 15" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {/* Current value + all trends */}
        <div className="p-4 sm:p-5">
          <div className="flex flex-col sm:flex-row sm:items-end gap-3 sm:gap-6 mb-5">
            {/* Big current value */}
            <div>
              {binaryDisplay ? (
                <span className={`text-4xl font-bold ${binaryDisplay.color}`}>
                  {binaryDisplay.text}
                </span>
              ) : (
                <div className="flex items-baseline gap-2">
                  <span className="text-4xl font-bold text-white tabular-nums">
                    {formatNumber(stat.current)}
                  </span>
                  {unit && (
                    <span className="text-sm text-slate-500 font-medium">{unit}</span>
                  )}
                </div>
              )}
              <p className="text-[11px] text-slate-600 mt-1">
                Last updated {timeAgo(stat.last_updated)}
              </p>
            </div>

            {/* Trend summary cards */}
            <div className="flex gap-2 sm:gap-3 flex-wrap">
              {PERIODS.map(p => {
                const trend = stat.trends?.[p];
                return (
                  <div
                    key={p}
                    className={`rounded-lg px-3 py-2 border cursor-pointer transition-all ${
                      p === period
                        ? 'bg-slate-800 border-slate-600'
                        : 'bg-slate-900/50 border-slate-800/40 hover:border-slate-700/60'
                    }`}
                    onClick={() => setPeriod(p)}
                  >
                    <div className="text-[10px] text-slate-500 font-medium uppercase">{p}</div>
                    {trend && trend.change !== null && trend.change !== undefined ? (
                      <>
                        <div className={`text-sm font-semibold ${trendColor(trend)}`}>
                          {trendArrow(trend)} {formatNumber(Math.abs(trend.change))}
                        </div>
                        {trend.pct !== null && trend.pct !== undefined && (
                          <div className="text-[10px] text-slate-600">
                            {trend.pct >= 0 ? '+' : ''}{trend.pct.toFixed(1)}%
                          </div>
                        )}
                      </>
                    ) : (
                      <div className="text-[11px] text-slate-600 italic">no data</div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Chart */}
          <div className="h-48 sm:h-56 mb-4 rounded-xl bg-slate-950/60 border border-slate-800/40 p-2">
            {loading ? (
              <div className="w-full h-full flex items-center justify-center">
                <span className="text-sm text-slate-600">Loading chart…</span>
              </div>
            ) : (
              <DetailChart
                data={chartData}
                timestamps={chartTimestamps}
                color={chartColor}
              />
            )}
          </div>

          {/* Data points table (last 10) */}
          {historyData && historyData.length > 0 && (
            <div>
              <h3 className="text-[11px] font-medium text-slate-500 uppercase tracking-wider mb-2">
                Recent Data Points ({historyData.length} total in {period})
              </h3>
              <div className="rounded-lg border border-slate-800/40 overflow-hidden">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="bg-slate-800/30">
                      <th className="text-left px-3 py-2 text-slate-500 font-medium">Time</th>
                      <th className="text-right px-3 py-2 text-slate-500 font-medium">Value</th>
                      <th className="text-right px-3 py-2 text-slate-500 font-medium">Change</th>
                    </tr>
                  </thead>
                  <tbody>
                    {historyData.slice(-10).reverse().map((point, i, arr) => {
                      const prevPoint = i < arr.length - 1 ? arr[i + 1] : null;
                      const change = prevPoint ? point.value - prevPoint.value : null;
                      return (
                        <tr key={i} className="border-t border-slate-800/30">
                          <td className="px-3 py-1.5 text-slate-400 font-mono text-[11px]">
                            {formatTimestamp(point.recorded_at)}
                          </td>
                          <td className="px-3 py-1.5 text-right text-white tabular-nums font-medium">
                            {formatFull(point.value)}
                          </td>
                          <td className={`px-3 py-1.5 text-right tabular-nums ${
                            change === null ? 'text-slate-600' :
                            change > 0 ? 'text-emerald-400' :
                            change < 0 ? 'text-red-400' : 'text-slate-500'
                          }`}>
                            {change === null ? '—' :
                             change > 0 ? `+${formatFull(change)}` :
                             formatFull(change)}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
