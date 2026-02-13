import React, { useState, useEffect } from 'react';
import { fetchStats, fetchHealth } from './api';
import StatCard from './components/StatCard';

const REFRESH_INTERVAL = 60_000; // 60 seconds

export default function App() {
  const [stats, setStats] = useState([]);
  const [health, setHealth] = useState(null);
  const [error, setError] = useState(null);
  const [lastRefresh, setLastRefresh] = useState(null);

  const loadData = async () => {
    try {
      const [statsData, healthData] = await Promise.all([
        fetchStats(),
        fetchHealth(),
      ]);
      setStats(statsData.stats || []);
      setHealth(healthData);
      setError(null);
      setLastRefresh(new Date());
    } catch (err) {
      setError(err.message);
    }
  };

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, REFRESH_INTERVAL);
    return () => clearInterval(interval);
  }, []);

  // Calculate grid layout based on number of cards
  const count = stats.length;
  const cols = count <= 4 ? 2 : count <= 6 ? 3 : count <= 9 ? 3 : 4;
  const rows = Math.ceil(count / cols);

  // Responsive grid classes: 1 col mobile, 2 col tablet, dynamic on desktop
  const desktopColClass = cols <= 2
    ? 'lg:grid-cols-2'
    : cols <= 3
      ? 'lg:grid-cols-3'
      : 'lg:grid-cols-3 xl:grid-cols-4';

  return (
    <div className="min-h-screen lg:h-screen overflow-y-auto lg:overflow-hidden bg-slate-950 text-white flex flex-col p-3 sm:p-4">
      {/* Header — responsive: stacks on mobile */}
      <header className="flex flex-col sm:flex-row sm:items-center sm:justify-between mb-3 flex-shrink-0 gap-1 sm:gap-0">
        <div className="flex items-center gap-3">
          <img src="/logo.svg" alt="The Pack" className="h-6 w-6 sm:h-7 sm:w-7" />
          <h1 className="text-base sm:text-lg font-semibold tracking-tight">The Pack</h1>
          <span className="text-[10px] sm:text-xs text-slate-600">Agent Operations</span>
        </div>
        <div className="flex items-center gap-3 sm:gap-4 text-[10px] sm:text-xs text-slate-500">
          {health && (
            <span>{health.keys_count} metrics · {health.stats_count} pts</span>
          )}
          {lastRefresh && (
            <span className="hidden sm:inline">Updated {lastRefresh.toLocaleTimeString()}</span>
          )}
          {error && (
            <span className="text-red-400">⚠ {error}</span>
          )}
        </div>
      </header>

      {/* Stats Grid — scrollable on mobile, fills viewport on desktop */}
      {stats.length === 0 ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <div className="text-6xl mb-4">📡</div>
            <h2 className="text-xl text-slate-400 mb-2">No Data Yet</h2>
            <p className="text-sm text-slate-600 max-w-md">
              Stats will appear here once the collector cron starts posting data.
              Use <code className="bg-slate-800 px-1.5 py-0.5 rounded text-xs">POST /api/v1/stats</code> to submit metrics.
            </p>
          </div>
        </div>
      ) : (
        <div
          className={`flex-1 dashboard-grid grid gap-2 sm:gap-3 grid-cols-1 sm:grid-cols-2 ${desktopColClass} min-h-0`}
          style={{ '--grid-rows': rows }}
        >
          {stats.map(stat => (
            <StatCard key={stat.key} stat={stat} />
          ))}
        </div>
      )}
    </div>
  );
}
