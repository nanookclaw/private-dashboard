# 📊 The Pack — Agent Operations Dashboard

Local network stats dashboard for the [Humans Not Required](https://github.com/Humans-Not-Required) agent collective. Tracks operational metrics with trend analysis, sparkline charts, alert history, and custom date ranges across multiple time windows.

## Features

- **Time-series metrics storage** — Submit stat snapshots via API, query with multi-window trends
- **Trend analysis** — 24h / 7d / 30d / 90d windows with percentage change and directional indicators
- **Sparkline charts** — Interactive mini-charts with crosshair hover and value tooltips
- **Metric detail modals** — Click any card for full interactive chart, trend summaries, and data table
- **Custom date ranges** — Query arbitrary time windows with start/end date pickers
- **Alert history** — Auto-recorded significant changes (≥10% alert, ≥25% hot) with 6h debounce
- **Metric grouping** — Development, Network, Moltbook, Work Queue, Social sections
- **Binary metric display** — Health metrics show "Healthy"/"Down" with color instead of raw values
- **Trend alerts** — Pulsing dot + glow border for significant 24h changes (±10% alert, ±25% hot with ⚡)
- **CSV export** — Download metric history as CSV from detail view
- **Data retention** — 90-day auto-prune on startup, manual prune endpoint
- **Dark theme** — Fullscreen dashboard designed for always-on displays
- **Auto-refresh** — Updates every 60 seconds
- **Token auth** — Write-protected with auto-generated manage key
- **Responsive layout** — 1-col mobile, 2-col tablet, viewport-filling desktop
- **Full-viewport modals** — Detail modals fill entire screen on mobile

## Quick Start

```bash
# Build and run
cargo run

# The manage key is printed on first run — save it!
# 🔑 Generated new manage key: dash_xxxx
```

### Submit Metrics

```bash
curl -X POST http://localhost:8000/api/v1/stats \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer dash_xxxx" \
  -d '[
    {"key": "agents_discovered", "value": 828},
    {"key": "tests_total", "value": 1500},
    {"key": "moltbook_health", "value": 1}
  ]'
# → {"accepted": 3}
```

### Query Current Stats

```bash
# All metrics with trends and sparklines
curl http://localhost:8000/api/v1/stats

# Single metric history (default: 24h)
curl http://localhost:8000/api/v1/stats/agents_discovered?period=7d

# Custom date range
curl "http://localhost:8000/api/v1/stats/tests_total?start=2026-02-01&end=2026-02-15"
```

### View Alert History

```bash
# All alerts (newest first)
curl http://localhost:8000/api/v1/alerts

# Filtered by metric
curl "http://localhost:8000/api/v1/alerts?key=agents_discovered&limit=10"
```

### Data Management

```bash
# Manual data retention (deletes stats > 90 days old)
curl -X POST http://localhost:8000/api/v1/stats/prune \
  -H "Authorization: Bearer dash_xxxx"
# → {"deleted": 42, "remaining": 1580}

# Delete all data for a specific metric
curl -X DELETE http://localhost:8000/api/v1/stats/agents_discovered \
  -H "Authorization: Bearer dash_xxxx"
# → {"key": "agents_discovered", "deleted": 340}
```

## API Reference

All endpoints are under `/api/v1`.

### Health

```
GET /api/v1/health
```

Returns service status, version, stat count, data retention info, and oldest stat timestamp.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "stats_count": 1580,
  "unique_keys": 19,
  "retention_days": 90,
  "oldest_stat": "2026-01-15T00:00:00Z"
}
```

### Submit Stats (Batch)

```
POST /api/v1/stats
Authorization: Bearer <manage_key>
Content-Type: application/json
```

Submit up to 100 metric snapshots per request. Each entry requires `key` (1-100 chars) and `value` (number). Optional `metadata` JSON object for context.

**Request body:** Array of `{"key": string, "value": number, "metadata?": object}`

**Response:** `{"accepted": <count>}`

**Errors:** 401 (missing/invalid auth), 422 (validation: empty batch, >100 items, invalid key length)

**Side effects:** Triggers alert checks — if a metric changes ≥10% from its previous value, an alert is auto-logged (debounced to max 1 per key per 6 hours).

### Get All Stats

```
GET /api/v1/stats
```

Returns all metrics with latest value, trend data across 4 windows, sparkline points, and human-readable labels. No auth required.

**Response:**
```json
{
  "stats": [
    {
      "key": "agents_discovered",
      "key_label": "Agents Discovered",
      "current": 828,
      "seq": 4521,
      "trends": {
        "24h": {"start": 820, "end": 828, "change": 8, "pct": 0.98},
        "7d": {"start": 780, "end": 828, "change": 48, "pct": 6.15},
        "30d": {"start": 645, "end": 828, "change": 183, "pct": 28.37},
        "90d": {"start": null, "end": 828, "change": null, "pct": null}
      },
      "sparkline_24h": [820, 822, 824, 825, 826, 828],
      "last_updated": "2026-02-16T03:00:00Z"
    }
  ]
}
```

**Trend calculation:** Each window compares the current value against the earliest recorded value within that time window. If the start value is 0, `pct` is null (avoids division by zero). If no data exists for a window, all trend fields are null.

### Get Stat History

```
GET /api/v1/stats/:key?period=24h|7d|30d|90d
GET /api/v1/stats/:key?start=<ISO-8601>&end=<ISO-8601>
```

Returns time-series data points for a single metric. Default period: `24h`. When `start` and `end` are provided, `period` is ignored. Dates accept `YYYY-MM-DD` or full ISO-8601 timestamps.

**Response:**
```json
{
  "key": "agents_discovered",
  "points": [
    {"value": 820, "recorded_at": "2026-02-15T00:00:00Z", "seq": 4500},
    {"value": 828, "recorded_at": "2026-02-16T03:00:00Z", "seq": 4521}
  ]
}
```

### Prune Stats

```
POST /api/v1/stats/prune
Authorization: Bearer <manage_key>
```

Manually trigger data retention. Deletes all stat entries older than 90 days. Also runs automatically on server startup.

**Response:** `{"deleted": <count>, "remaining": <count>}`

### Delete Stat

```
DELETE /api/v1/stats/:key
Authorization: Bearer <manage_key>
```

Deletes all data points and associated alerts for a specific metric key.

**Response:** `{"key": "<key>", "deleted": <count>}`

**Errors:** 404 if no data exists for the given key.

### Get Alerts

```
GET /api/v1/alerts?key=<optional>&limit=<optional>
```

Returns alert history log — automatically recorded when metrics change significantly on submission.

**Alert levels:**
- **alert** — ≥10% change from previous value
- **hot** — ≥25% change from previous value (with ⚡ indicator)

**Parameters:**
- `key` — Filter by metric key (optional)
- `limit` — Results per page, 1-500 (default: 50)

**Response:**
```json
{
  "alerts": [
    {
      "key": "tests_total",
      "label": "Tests Total",
      "level": "hot",
      "value": 1500,
      "change_pct": 28.5,
      "triggered_at": "2026-02-16T02:30:00Z"
    }
  ],
  "total": 15
}
```

**Debounce:** Max 1 alert per metric per 6 hours to prevent noise from frequent submissions.

### Discovery Endpoints

```
GET /llms.txt         — AI-readable API summary (plain text)
GET /openapi.json     — Full OpenAPI 3.0.3 specification
```

## Authentication

- **Read endpoints** — No auth required (local network only, private by design)
- **Write endpoints** — `Authorization: Bearer <manage_key>` header required
- **Manage key** — Auto-generated on first run, stored in SQLite `config` table, printed to stdout
- **Pattern** — Follows HNR convention: token tied to the resource instance, no user accounts

## Frontend

React single-page app served from the same binary when `STATIC_DIR` is set.

### Dashboard Layout

- **Grouped metrics** — Cards organized into sections: Development (⚡), Network (🌐), Moltbook (📘), Work Queue (📋), Social (📡)
- **Stat cards** — Large current value with unit suffix, trend badge (↑↓ with %), interactive sparkline
- **Period selector** — Switch between 24h / 7d / 30d / 90d / Custom per card
- **Metric detail modal** — Click any card for full-size chart, trend breakdown, data table with CSV export
- **Alert panel** — Recent alerts section below stats grid, color-coded by level and direction
- **Responsive** — Viewport-filling on desktop, natural scroll on mobile/tablet
- **Dark theme** — Designed for always-on monitoring displays

### Known Metric Keys

| Key | Label | Unit | Group |
|-----|-------|------|-------|
| `repos_count` | Repos | repos | Development |
| `commits_total` | Commits | commits | Development |
| `tests_total` | Tests | tests | Development |
| `agents_discovered` | Agents Discovered | agents | Network |
| `siblings_active` | Active Siblings | agents | Network |
| `siblings_count` | Total Siblings | agents | Network |
| `cron_jobs_active` | Cron Jobs | jobs | Network |
| `moltbook_health` | Moltbook Health | (binary) | Moltbook |
| `moltbook_interesting` | Interesting Posts | posts | Moltbook |
| `moltbook_my_posts` | My Posts | posts | Moltbook |
| `moltbook_spam` | Spam Posts | posts | Moltbook |
| `kanban_active` | Active Tasks | tasks | Work Queue |
| `kanban_in_progress` | In Progress | tasks | Work Queue |
| `kanban_review` | In Review | tasks | Work Queue |
| `kanban_backlog` | Backlog | tasks | Work Queue |
| `kanban_up_next` | Up Next | tasks | Work Queue |
| `kanban_done` | Done | tasks | Work Queue |
| `twitter_headlines` | Twitter Headlines | headlines | Social |
| `twitter_accounts` | Twitter Accounts | accounts | Social |
| `outreach_sent` | Outreach Sent | emails | Social |
| `outreach_received` | Outreach Received | emails | Social |

## Configuration

| Env Variable | Default | Description |
|-------------|---------|-------------|
| `ROCKET_PORT` | `8000` | Server port |
| `ROCKET_ADDRESS` | `0.0.0.0` | Bind address |
| `DATABASE_PATH` | `data/dashboard.db` | SQLite database path |
| `STATIC_DIR` | `static/` | Frontend static files directory |

## Docker

```bash
docker compose up -d
# Accessible at http://localhost:3008
```

The Docker image is a multi-stage build: frontend (Bun/Vite) → backend (Rust/Rocket). Database is persisted via a named volume (`dashboard-data`).

**CI/CD:** GitHub Actions builds and pushes to `ghcr.io/humans-not-required/private-dashboard:dev`. Watchtower auto-deploys on staging.

## Python SDK

Complete zero-dependency Python client in `sdk/python/`:

```python
from dashboard import Dashboard

dash = Dashboard("http://localhost:3008", manage_key="dash_xxx")

# Submit metrics (dict shorthand)
dash.submit({"tests_total": 1500, "repos_count": 9})

# Read current values
val = dash.get_value("tests_total")     # → 1500.0
pct = dash.get_trend("tests_total", "7d")  # → 25.0

# Full stats with trends and sparklines
for s in dash.stats():
    print(f"{s['label']}: {s['current']}")

# History
points = dash.history("tests_total", period="7d")
points = dash.history("tests_total", start="2026-02-01", end="2026-02-15")

# Alerts
alerts = dash.alerts(limit=20)
hot = dash.hot_alerts()  # ≥25% change only
```

Features: typed errors (`AuthError`, `NotFoundError`, `ValidationError`, `RateLimitError`), convenience helpers (`get_value`, `get_trend`, `keys`, `is_healthy`), env config (`DASHBOARD_URL`, `DASHBOARD_KEY`). 52 integration tests. See `sdk/python/README.md` for full docs.

## Data Collector

Metrics are collected by a separate cron job (`scripts/dashboard-collector.py`) in the OpenClaw workspace. The collector reads workspace state files (agent-registry.json, moltbook-state.json, etc.) and POSTs to the dashboard API every 30 minutes.

The collector is not part of this repo — it's an operational concern managed separately.

## Tech Stack

- **Backend:** Rust (Rocket 0.5), SQLite (rusqlite)
- **Frontend:** React 19, Tailwind CSS, Vite, Bun
- **Deployment:** Docker multi-stage build, Watchtower auto-deploy
- **CI/CD:** GitHub Actions → ghcr.io

## License

MIT
