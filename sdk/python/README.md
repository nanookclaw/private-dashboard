# Private Dashboard Python SDK

Zero-dependency Python client for **The Pack** — an agent operations dashboard that tracks metrics, trends, sparklines, and anomaly alerts.

## Install

Copy `dashboard.py` into your project. No `pip install` needed — stdlib only, Python 3.8+.

## Quick Start

```python
from dashboard import Dashboard

dash = Dashboard("http://localhost:3008", manage_key="dash_abc123")

# Submit metrics (dict shorthand)
dash.submit({"tests_total": 1500, "repos_count": 9})

# Read all metrics with trends
for s in dash.stats():
    pct = s["trends"]["24h"].get("pct")
    trend = f"{pct:+.1f}%" if pct is not None else "n/a"
    print(f"{s['label']}: {s['current']} ({trend} 24h)")

# Single metric
val = dash.get_value("tests_total")  # → 1500.0

# History
points = dash.history("tests_total", period="7d")
for p in points:
    print(f"  {p['recorded_at']}: {p['value']}")
```

## Configuration

```python
# Explicit
dash = Dashboard("http://192.168.0.79:3008", manage_key="dash_xxx")

# Environment variables
# DASHBOARD_URL=http://192.168.0.79:3008
# DASHBOARD_KEY=dash_xxx
dash = Dashboard()
```

## API Coverage

| Method | Endpoint | Auth |
|--------|----------|------|
| `health()` | GET /api/v1/health | No |
| `stats()` | GET /api/v1/stats | No |
| `stat(key)` | GET /api/v1/stats (filtered) | No |
| `history(key, period=, start=, end=)` | GET /api/v1/stats/:key | No |
| `submit(metrics)` | POST /api/v1/stats | Yes |
| `submit_one(key, value, metadata=)` | POST /api/v1/stats | Yes |
| `delete(key)` | DELETE /api/v1/stats/:key | Yes |
| `prune()` | POST /api/v1/stats/prune | Yes |
| `alerts(key=, limit=)` | GET /api/v1/alerts | No |
| `alert_count()` | GET /api/v1/alerts | No |
| `llms_txt()` | GET /llms.txt | No |
| `openapi()` | GET /openapi.json | No |
| `skills_index()` | GET /.well-known/skills/index.json | No |
| `skill_md()` | GET /.well-known/skills/.../SKILL.md | No |

## Submitting Metrics

```python
# Dict shorthand (key → value)
dash.submit({"agents_discovered": 475, "tests_total": 1500})

# Full form with metadata
dash.submit([
    {"key": "deploys_count", "value": 42, "metadata": {"last_repo": "watchpost"}},
])

# Single metric
dash.submit_one("siblings_active", 3, metadata={"names": ["Forge", "Drift", "Lux"]})
```

## Reading Metrics

```python
# All metrics with trends and sparklines
stats = dash.stats()

# Single metric lookup
s = dash.stat("tests_total")
print(s["current"])        # 1500.0
print(s["label"])          # "Total Tests"
print(s["sparkline_24h"])  # [1480, 1490, 1500, ...]
print(s["trends"]["7d"])   # {"start": 1200, "end": 1500, "change": 300, "pct": 25.0}

# Convenience helpers
val = dash.get_value("repos_count")     # 9.0 or None
pct = dash.get_trend("tests_total", "7d")  # 25.0 or None
keys = dash.keys()                       # ["agents_discovered", "tests_total", ...]
```

## History & Date Ranges

```python
# Standard periods
points = dash.history("tests_total", period="24h")  # also: 7d, 30d, 90d

# Custom date range (ISO-8601 or YYYY-MM-DD)
points = dash.history("tests_total", start="2026-02-01", end="2026-02-15")
points = dash.history("tests_total",
    start="2026-02-01T00:00:00Z",
    end="2026-02-15T23:59:59Z")

for p in points:
    print(f"{p['recorded_at']}: {p['value']}")
```

## Alerts

Alerts fire automatically when metrics change ≥10% over 24 hours:

```python
# All recent alerts
alerts = dash.alerts(limit=20)
for a in alerts:
    print(f"[{a['level']}] {a['label']}: {a['change_pct']:+.1f}% → {a['value']}")

# Filter by key
alerts = dash.alerts(key="tests_total")

# Hot alerts only (≥25% change)
hot = dash.hot_alerts()

# Total alert count
count = dash.alert_count()
```

## Error Handling

```python
from dashboard import (
    DashboardError,   # Base class
    AuthError,        # 403 — invalid manage key
    NotFoundError,    # 404 — resource not found
    ValidationError,  # 400 — bad request
    RateLimitError,   # 429 — rate limited (has .retry_after)
    ServerError,      # 500+ — server error
)

try:
    dash.submit({"key": 1.0})
except AuthError:
    print("Bad manage key")
except ValidationError as e:
    print(f"Bad request: {e}")
except DashboardError as e:
    print(f"Error {e.status}: {e}")
```

## Service Discovery

```python
# AI-readable docs
print(dash.llms_txt())

# OpenAPI spec
spec = dash.openapi()

# Agent skills discovery (Cloudflare RFC)
index = dash.skills_index()
skill = dash.skill_md()
```

## Tests

```bash
# Run against staging
DASHBOARD_URL=http://192.168.0.79:3008 \
DASHBOARD_KEY=dash_xxx \
python3 test_sdk.py
```

52 integration tests covering all endpoints, error paths, and edge cases.
