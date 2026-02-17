# Private Dashboard — Design Document

## Purpose

Local network stats dashboard for the HNR ecosystem. Displays key operational metrics with trend data across multiple time windows. Designed for a single fullscreen TV/monitor display.

## Architecture

**Backend:** Rust + Rocket + SQLite  
**Frontend:** React + Vite + Tailwind CSS  
**Port:** 3008 (staging: 192.168.0.79:3008)  
**Database:** SQLite (single file, volume-mounted in Docker)

## Data Model

### Stats Table

```sql
CREATE TABLE stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL,           -- metric identifier (e.g., "agents_discovered")
    value REAL NOT NULL,         -- numeric value
    recorded_at TEXT NOT NULL,   -- ISO-8601 UTC timestamp
    metadata TEXT,               -- optional JSON blob for context
    seq INTEGER NOT NULL         -- monotonic sequence number
);
CREATE INDEX idx_stats_key_time ON stats(key, recorded_at);
CREATE INDEX idx_stats_seq ON stats(seq);
```

Each row is a point-in-time snapshot of a metric. The collector cron POSTs new snapshots periodically.

### Metric Keys (v1)

| Key | Description | Source |
|-----|-------------|--------|
| `agents_discovered` | Total agents in registry | agent-registry.json |
| `moltbook_interesting` | Interesting Moltbook posts (cumulative) | moltbook-state.json |
| `moltbook_spam` | Spam posts seen (cumulative) | moltbook-state.json |
| `outreach_sent` | Outreach emails sent | email-outreach.json |
| `outreach_received` | Inbound emails received | email-outreach.json |
| `repos_count` | Number of HNR repos | GitHub API |
| `tests_total` | Total tests across all repos | STATUS.md files |
| `deploys_count` | Successful deploys | GitHub Actions |
| `commits_total` | Total commits across repos | git log |
| `twitter_headlines` | Flagged tweets count | twitter-state.json |
| `siblings_count` | Active sibling agents | Proxmox API |

## API

### Health
```
GET /api/v1/health → { "status": "ok", "version": "0.1.0" }
```

### Submit Stats (batch)
```
POST /api/v1/stats
Authorization: Bearer <manage_key>
Body: [
  { "key": "agents_discovered", "value": 645 },
  { "key": "moltbook_interesting", "value": 42, "metadata": { "source": "feed-scan" } }
]
Response: { "accepted": 2 }
```

### Get Current Stats (all metrics, latest value + trends)
```
GET /api/v1/stats
Response: {
  "stats": [
    {
      "key": "agents_discovered",
      "current": 645,
      "trends": {
        "24h": { "start": 640, "end": 645, "change": 5, "pct": 0.78 },
        "7d":  { "start": 600, "end": 645, "change": 45, "pct": 7.5 },
        "30d": { "start": 500, "end": 645, "change": 145, "pct": 29.0 },
        "90d": { "start": 200, "end": 645, "change": 445, "pct": 222.5 }
      },
      "sparkline_24h": [640, 641, 642, 643, 644, 645],
      "last_updated": "2026-02-10T18:00:00Z"
    }
  ]
}
```

### Get Single Stat History
```
GET /api/v1/stats/:key?period=24h|7d|30d|90d
Response: {
  "key": "agents_discovered",
  "points": [
    { "value": 640, "recorded_at": "2026-02-10T00:00:00Z" },
    { "value": 645, "recorded_at": "2026-02-10T18:00:00Z" }
  ]
}
```

## Auth

- **Read:** No auth required (local network only, private by design)
- **Write:** Bearer token required for POST /api/v1/stats
- **Manage key:** Generated on first run, stored in DB, printed to stdout
- Follows HNR pattern: token tied to the dashboard instance

## Frontend

Single fullscreen page. Dark theme. Grid of stat cards.

**Each card shows:**
- Metric name (human-readable label)
- Big number (current value, formatted)
- Trend indicator (↑↓ with percentage, color-coded green/red)
- Mini sparkline (24h)
- Time window selector (24h/7d/30d/90d)

**Layout:** Responsive CSS Grid — fills available screen. Cards auto-size.

**Refresh:** Auto-refresh every 60 seconds.

## Deployment

Docker Compose on staging (192.168.0.79), port 3008. Watchtower auto-deploys from ghcr.io.

## Collector (Phase 2)

A separate cron job reads workspace state files and POSTs to the dashboard API. NOT part of this repo — lives in the OpenClaw workspace as a playbook/cron job.

## Python SDK

Complete zero-dependency Python client library in `sdk/python/dashboard.py`:

- **Zero deps** — stdlib only, Python 3.8+
- **Dict shorthand** — `dash.submit({"tests_total": 1500})` for ergonomic submission
- **Full API coverage** — health, stats, history, submit, delete, prune, alerts, discovery
- **Typed errors** — `AuthError`, `NotFoundError`, `ValidationError`, `RateLimitError`
- **Convenience helpers** — `get_value()`, `get_trend()`, `keys()`, `is_healthy()`, `hot_alerts()`
- **Env config** — `DASHBOARD_URL` and `DASHBOARD_KEY` environment variables
- **52 integration tests** in `sdk/python/test_sdk.py`

## Non-Goals (v1)

- No user accounts
- No historical data export
- No alerting (Watchpost handles that)
- No public access (local network only)
- No authentication for reads
