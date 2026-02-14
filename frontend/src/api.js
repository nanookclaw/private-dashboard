const BASE = '/api/v1';

export async function fetchStats() {
  const res = await fetch(`${BASE}/stats`);
  if (!res.ok) throw new Error(`Failed to fetch stats: ${res.status}`);
  return res.json();
}

export async function fetchStatHistory(key, period = '24h', start = null, end = null) {
  let url;
  if (start && end) {
    url = `${BASE}/stats/${encodeURIComponent(key)}?start=${encodeURIComponent(start)}&end=${encodeURIComponent(end)}`;
  } else {
    url = `${BASE}/stats/${encodeURIComponent(key)}?period=${period}`;
  }
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch stat history: ${res.status}`);
  return res.json();
}

export async function fetchHealth() {
  const res = await fetch(`${BASE}/health`);
  if (!res.ok) throw new Error(`Failed to fetch health: ${res.status}`);
  return res.json();
}
