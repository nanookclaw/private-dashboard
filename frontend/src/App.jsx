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

  return (
    <div className="h-screen overflow-hidden bg-slate-950 text-white flex flex-col p-4">
      {/* Header — compact */}
      <header className="flex items-center justify-between mb-3 flex-shrink-0">
        <div className="flex items-center gap-3">
          <h1 className="text-lg font-semibold tracking-tight">📊 HNR Dashboard</h1>
          <span className="text-xs text-slate-600">Humans Not Required</span>
        </div>
        <div className="flex items-center gap-4 text-xs text-slate-500">
          {health && (
            <span>{health.keys_count} metrics · {health.stats_count} points</span>
          )}
          {lastRefresh && (
            <span>Updated {lastRefresh.toLocaleTimeString()}</span>
          )}
          {error && (
            <span className="text-red-400">⚠ {error}</span>
          )}
        </div>
      </header>

      {/* Stats Grid — fills remaining viewport */}
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
          className="flex-1 grid gap-3 min-h-0"
          style={{
            gridTemplateColumns: `repeat(${cols}, 1fr)`,
            gridTemplateRows: `repeat(${rows}, 1fr)`,
          }}
        >
          {stats.map(stat => (
            <StatCard key={stat.key} stat={stat} />
          ))}
        </div>
      )}
    </div>
  );
}
