import React, { useState, useEffect } from 'react';
import { fetchStats, fetchHealth, fetchAlerts } from './api';
import StatCard from './components/StatCard';
import MetricDetail from './components/MetricDetail';
import AlertHistory from './components/AlertHistory';

const REFRESH_INTERVAL = 60_000; // 60 seconds

// Metric grouping: ordered groups with their metric keys
const METRIC_GROUPS = [
  {
    id: 'development',
    label: 'Development',
    icon: '⚡',
    keys: ['repos_count', 'commits_total', 'tests_total'],
  },
  {
    id: 'network',
    label: 'Network',
    icon: '🌐',
    keys: ['agents_discovered', 'siblings_active', 'siblings_count'],
  },
  {
    id: 'moltbook',
    label: 'Moltbook',
    icon: '📘',
    keys: ['moltbook_health', 'moltbook_interesting', 'moltbook_my_posts', 'moltbook_spam'],
  },
  {
    id: 'social',
    label: 'Social',
    icon: '📡',
    keys: ['twitter_headlines', 'twitter_accounts', 'outreach_sent', 'outreach_received'],
  },
];

// Build grouped stats from flat array
function groupStats(stats) {
  const byKey = {};
  for (const s of stats) byKey[s.key] = s;

  const grouped = [];
  const placed = new Set();

  for (const group of METRIC_GROUPS) {
    const items = group.keys.map(k => byKey[k]).filter(Boolean);
    if (items.length > 0) {
      grouped.push({ ...group, stats: items });
      items.forEach(s => placed.add(s.key));
    }
  }

  // Any ungrouped metrics go into "Other"
  const other = stats.filter(s => !placed.has(s.key));
  if (other.length > 0) {
    grouped.push({ id: 'other', label: 'Other', icon: '📊', stats: other });
  }

  return grouped;
}

export default function App() {
  const [stats, setStats] = useState([]);
  const [health, setHealth] = useState(null);
  const [alerts, setAlerts] = useState([]);
  const [alertsLoading, setAlertsLoading] = useState(true);
  const [error, setError] = useState(null);
  const [lastRefresh, setLastRefresh] = useState(null);
  const [selectedStat, setSelectedStat] = useState(null);

  const loadData = async () => {
    try {
      const [statsData, healthData, alertsData] = await Promise.all([
        fetchStats(),
        fetchHealth(),
        fetchAlerts(20),
      ]);
      setStats(statsData.stats || []);
      setHealth(healthData);
      setAlerts(alertsData.alerts || []);
      setAlertsLoading(false);
      setError(null);
      setLastRefresh(new Date());
    } catch (err) {
      setError(err.message);
      setAlertsLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    const interval = setInterval(loadData, REFRESH_INTERVAL);
    return () => clearInterval(interval);
  }, []);

  const groups = groupStats(stats);

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

      {/* Stats — grouped layout */}
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
        <div className="lg:flex-1 flex flex-col gap-3 sm:gap-4 lg:min-h-0 overflow-y-auto lg:overflow-hidden">
          {groups.map(group => (
            <div key={group.id} className="lg:flex-1 lg:min-h-0 flex flex-col">
              {/* Group label */}
              <div className="flex items-center gap-2 mb-1.5 flex-shrink-0">
                <span className="text-[11px] sm:text-xs">{group.icon}</span>
                <span className="text-[10px] sm:text-[11px] font-medium text-slate-600 uppercase tracking-widest">
                  {group.label}
                </span>
                <div className="flex-1 h-px bg-slate-800/60" />
              </div>
              {/* Group cards */}
              <div className={`lg:flex-1 grid gap-2 sm:gap-3 grid-cols-1 sm:grid-cols-2 lg:min-h-0 ${
                group.stats.length <= 2 ? 'lg:grid-cols-2' :
                group.stats.length === 3 ? 'lg:grid-cols-3' :
                'lg:grid-cols-4'
              }`}>
                {group.stats.map(stat => (
                  <StatCard key={stat.key} stat={stat} onClick={() => setSelectedStat(stat)} />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
      {/* Alert history */}
      <AlertHistory alerts={alerts} loading={alertsLoading} />

      {/* Metric detail modal */}
      {selectedStat && (
        <MetricDetail stat={selectedStat} onClose={() => setSelectedStat(null)} />
      )}
      <footer className="text-center py-2 px-4 text-[0.65rem] text-slate-600 flex-shrink-0">
        Made for AI, by AI.{' '}
        <a href="https://github.com/Humans-Not-Required" target="_blank" rel="noopener noreferrer" className="text-indigo-400 hover:text-indigo-300">Humans not required</a>.
      </footer>
    </div>
  );
}
