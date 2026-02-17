use rocket::serde::json::Json;
use rocket::http::{ContentType, Status};
use rocket::State;
use std::sync::Arc;
use chrono::{Utc, Duration};

use crate::db::Db;
use crate::auth::ManageKey;
use crate::models::*;

/// Alert thresholds (match frontend logic)
const ALERT_THRESHOLD_PCT: f64 = 10.0;
const HOT_THRESHOLD_PCT: f64 = 25.0;
/// Minimum hours between alerts for the same key (debounce)
const ALERT_DEBOUNCE_HOURS: i64 = 6;

pub const DEFAULT_RETENTION_DAYS: i64 = 90;

#[get("/health")]
pub fn health(db: &State<Arc<Db>>) -> Json<HealthResponse> {
    let keys = db.get_all_keys();
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        stats_count: db.get_stat_count(),
        keys_count: keys.len(),
        retention_days: DEFAULT_RETENTION_DAYS,
        oldest_stat: db.get_oldest_stat_time(),
    })
}

#[post("/stats", format = "json", data = "<stats>")]
pub fn submit_stats(
    db: &State<Arc<Db>>,
    auth: ManageKey,
    stats: Json<Vec<StatInput>>,
) -> Result<Json<StatSubmitResponse>, (Status, Json<serde_json::Value>)> {
    // Validate manage key
    let expected = db.get_manage_key().unwrap_or_default();
    if auth.0 != expected {
        return Err((
            Status::Forbidden,
            Json(serde_json::json!({"error": "Invalid manage key"})),
        ));
    }

    if stats.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Empty stats array"})),
        ));
    }

    if stats.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Too many stats (max 100)"})),
        ));
    }

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let mut accepted = 0;
    let mut keys_submitted = Vec::new();

    for stat in stats.iter() {
        if stat.key.is_empty() || stat.key.len() > 100 {
            continue;
        }
        let meta = stat.metadata.as_ref().map(|m| m.to_string());
        db.insert_stat(&stat.key, stat.value, &now_str, meta.as_deref());
        keys_submitted.push((stat.key.clone(), stat.value));
        accepted += 1;
    }

    // Check for alert conditions on submitted keys
    let debounce_cutoff = (now - Duration::hours(ALERT_DEBOUNCE_HOURS)).to_rfc3339();
    for (key, current_value) in &keys_submitted {
        // Compute 24h trend
        let since_24h = (now - Duration::hours(24)).to_rfc3339();
        if let Some(start_val) = db.get_stat_at_time(key, &since_24h) {
            if start_val != 0.0 {
                let pct = ((current_value - start_val) / start_val) * 100.0;
                let abs_pct = pct.abs();

                if abs_pct >= ALERT_THRESHOLD_PCT {
                    // Check debounce: skip if recent alert exists for this key
                    let should_record = match db.get_last_alert_time(key) {
                        Some(last) => last < debounce_cutoff,
                        None => true,
                    };

                    if should_record {
                        let level = if abs_pct >= HOT_THRESHOLD_PCT { "hot" } else { "alert" };
                        db.insert_alert(key, level, *current_value, pct, &now_str);
                    }
                }
            }
        }
    }

    Ok(Json(StatSubmitResponse { accepted }))
}

#[get("/stats")]
pub fn get_stats(db: &State<Arc<Db>>) -> Json<StatsResponse> {
    let latest = db.get_latest_stats();
    let now = Utc::now();

    let stats: Vec<StatSummary> = latest.iter().map(|s| {
        let trends = Trends {
            h24: compute_trend(db, &s.key, s.value, now - Duration::hours(24)),
            d7: compute_trend(db, &s.key, s.value, now - Duration::days(7)),
            d30: compute_trend(db, &s.key, s.value, now - Duration::days(30)),
            d90: compute_trend(db, &s.key, s.value, now - Duration::days(90)),
        };
        let sparkline = db.get_sparkline(&s.key, &(now - Duration::hours(24)).to_rfc3339(), 12);

        StatSummary {
            key: s.key.clone(),
            label: key_label(&s.key),
            current: s.value,
            trends,
            sparkline_24h: sparkline,
            last_updated: s.recorded_at.clone(),
        }
    }).collect();

    Json(StatsResponse { stats })
}

#[get("/stats/<key>?<period>&<start>&<end>")]
pub fn get_stat_history(
    db: &State<Arc<Db>>,
    key: &str,
    period: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> Result<Json<StatHistoryResponse>, (Status, Json<serde_json::Value>)> {
    // If both start and end are provided, use custom date range
    if let (Some(s), Some(e)) = (start, end) {
        // Validate ISO-8601 format
        let start_dt = chrono::DateTime::parse_from_rfc3339(s)
            .or_else(|_| {
                // Also accept YYYY-MM-DD format (treat as start of day UTC)
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().fixed_offset())
            });
        let end_dt = chrono::DateTime::parse_from_rfc3339(e)
            .or_else(|_| {
                chrono::NaiveDate::parse_from_str(e, "%Y-%m-%d")
                    .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc().fixed_offset())
            });

        match (start_dt, end_dt) {
            (Ok(s_dt), Ok(e_dt)) => {
                if s_dt > e_dt {
                    return Err((
                        Status::BadRequest,
                        Json(serde_json::json!({"error": "start must be before end"})),
                    ));
                }
                let points = db.get_stat_history_range(key, &s_dt.to_rfc3339(), &e_dt.to_rfc3339());
                return Ok(Json(StatHistoryResponse {
                    key: key.to_string(),
                    points: points.iter().map(|p| StatPointOut {
                        value: p.value,
                        recorded_at: p.recorded_at.clone(),
                    }).collect(),
                }));
            }
            _ => {
                return Err((
                    Status::BadRequest,
                    Json(serde_json::json!({"error": "Invalid date format. Use ISO-8601 (e.g. 2026-02-01T00:00:00Z) or YYYY-MM-DD"})),
                ));
            }
        }
    }

    let now = Utc::now();
    let since = match period.unwrap_or("24h") {
        "24h" => now - Duration::hours(24),
        "7d" => now - Duration::days(7),
        "30d" => now - Duration::days(30),
        "90d" => now - Duration::days(90),
        _ => {
            return Err((
                Status::BadRequest,
                Json(serde_json::json!({"error": "Invalid period. Use 24h, 7d, 30d, or 90d"})),
            ));
        }
    };

    let points = db.get_stat_history(key, &since.to_rfc3339());
    Ok(Json(StatHistoryResponse {
        key: key.to_string(),
        points: points.iter().map(|p| StatPointOut {
            value: p.value,
            recorded_at: p.recorded_at.clone(),
        }).collect(),
    }))
}

#[post("/stats/prune")]
pub fn prune_stats(
    db: &State<Arc<Db>>,
    auth: ManageKey,
) -> Result<Json<PruneResponse>, (Status, Json<serde_json::Value>)> {
    let expected = db.get_manage_key().unwrap_or_default();
    if auth.0 != expected {
        return Err((
            Status::Forbidden,
            Json(serde_json::json!({"error": "Invalid manage key"})),
        ));
    }

    let deleted = db.prune_old_stats(DEFAULT_RETENTION_DAYS);
    let remaining = db.get_stat_count();

    Ok(Json(PruneResponse {
        deleted,
        retention_days: DEFAULT_RETENTION_DAYS,
        remaining,
    }))
}

#[delete("/stats/<key>")]
pub fn delete_stat(
    db: &State<Arc<Db>>,
    auth: ManageKey,
    key: &str,
) -> Result<Json<DeleteResponse>, (Status, Json<serde_json::Value>)> {
    let expected = db.get_manage_key().unwrap_or_default();
    if auth.0 != expected {
        return Err((
            Status::Forbidden,
            Json(serde_json::json!({"error": "Invalid manage key"})),
        ));
    }

    let deleted = db.delete_stats_by_key(key);
    if deleted == 0 {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "No stats found for key", "key": key})),
        ));
    }

    Ok(Json(DeleteResponse {
        key: key.to_string(),
        deleted,
    }))
}

#[get("/alerts?<key>&<limit>")]
pub fn get_alerts(
    db: &State<Arc<Db>>,
    key: Option<&str>,
    limit: Option<i64>,
) -> Json<AlertsResponse> {
    let lim = limit.unwrap_or(50).clamp(1, 500);
    let alerts = match key {
        Some(k) => db.get_alerts_for_key(k, lim),
        None => db.get_alerts(lim),
    };
    let total = db.get_alert_count();

    Json(AlertsResponse {
        alerts: alerts.iter().map(|a| AlertOut {
            key: a.key.clone(),
            label: key_label(&a.key),
            level: a.level.clone(),
            value: a.value,
            change_pct: (a.change_pct * 10.0).round() / 10.0,
            triggered_at: a.triggered_at.clone(),
        }).collect(),
        total,
    })
}

// ── llms.txt ──
#[get("/llms.txt")]
pub fn llms_txt() -> (ContentType, &'static str) {
    (ContentType::Plain, "\
# The Pack — Agent Operations Dashboard
> Local network stats dashboard for the agent collective (Nanook + siblings).
> Displays key operational metrics with trend data across multiple time windows.

## API Base: /api/v1

## Endpoints

### GET /api/v1/health
Returns service status, version, and stat counts.

### POST /api/v1/stats
Submit a batch of stat snapshots. Requires `Authorization: Bearer <manage_key>`.
Body: Array of `{\"key\": string, \"value\": number, \"metadata?\": object}`.
Max 100 per batch. Keys must be 1-100 characters.

### GET /api/v1/stats
Returns all metrics with latest value, trend data (24h/7d/30d/90d), sparkline, and human-readable labels.
No auth required.

### GET /api/v1/stats/<key>?period=24h|7d|30d|90d
Returns time-series history for a single metric. Default period: 24h.
Supports custom date range: ?start=YYYY-MM-DD&end=YYYY-MM-DD (or ISO-8601).
When start and end are provided, period is ignored.

### POST /api/v1/stats/prune
Manually trigger data retention. Deletes stats older than 90 days.
Requires `Authorization: Bearer <manage_key>`. Returns deleted count and remaining.

### DELETE /api/v1/stats/<key>
Delete all data points for a specific metric key. Requires `Authorization: Bearer <manage_key>`.
Returns 404 if the key has no data. Response: `{\"key\": string, \"deleted\": number}`.

### GET /api/v1/alerts?key=<optional>&limit=<optional>
Returns alert history log — significant metric changes (>=10% = alert, >=25% = hot).
Auto-recorded on stat submission. Debounced to max 1 alert per key per 6 hours.
Optional filters: `key` for specific metric, `limit` (1-500, default 50).
Response: `{\"alerts\": [{\"key\", \"label\", \"level\", \"value\", \"change_pct\", \"triggered_at\"}], \"total\": number}`.
No auth required.

## Auth
- Read endpoints: No auth (local network only)
- Write endpoints: Bearer token (manage key generated on first run)

## Data Retention
- Auto-prune on startup: stats older than 90 days are automatically deleted
- Manual prune: POST /api/v1/stats/prune (auth required)

## Known Metric Keys
agents_discovered, moltbook_interesting, moltbook_spam, moltbook_health, moltbook_my_posts,
outreach_sent, outreach_received, repos_count, tests_total, deploys_count, commits_total,
twitter_headlines, twitter_accounts, siblings_count, siblings_active, cron_jobs_active

## Agent Skills Discovery
- GET /.well-known/skills/index.json — skills discovery index (Cloudflare RFC). Lists available skills for progressive loading by compatible agents.
- GET /.well-known/skills/private-dashboard/SKILL.md — integration skill with YAML frontmatter (agentskills.io format). Contains quick start, auth, metric keys, and alert thresholds.
")
}

// ── OpenAPI spec ──
#[get("/openapi.json")]
pub fn openapi_spec() -> (ContentType, &'static str) {
    (ContentType::JSON, include_str!("../openapi.json"))
}

// ── Well-Known Skills Discovery (Cloudflare RFC) ──

#[get("/.well-known/skills/index.json")]
pub fn skills_index() -> (ContentType, &'static str) {
    (ContentType::JSON, SKILLS_INDEX_JSON)
}

#[get("/.well-known/skills/private-dashboard/SKILL.md")]
pub fn skills_skill_md() -> (ContentType, &'static str) {
    (ContentType::Markdown, SKILL_MD)
}

const SKILLS_INDEX_JSON: &str = r#"{
  "skills": [
    {
      "name": "private-dashboard",
      "description": "Integrate with The Pack — an agent operations dashboard for tracking metrics, trends, and alerts across an AI agent collective on a local network.",
      "files": [
        "SKILL.md"
      ]
    }
  ]
}"#;

const SKILL_MD: &str = r#"---
name: private-dashboard
description: Integrate with The Pack — an agent operations dashboard for tracking metrics, trends, and alerts across an AI agent collective on a local network.
---

# Private Dashboard Integration

An operations dashboard for AI agent collectives. Track metrics with automatic trend analysis, sparklines, and anomaly alerts. Designed for LAN-first agent deployments.

## Quick Start

1. **Check health:**
   ```
   GET /api/v1/health
   ```

2. **Submit metrics:**
   ```
   POST /api/v1/stats
   Authorization: Bearer <manage_key>
   [{"key": "agents_discovered", "value": 42}]
   ```

3. **Read metrics:**
   ```
   GET /api/v1/stats
   ```
   Returns all metrics with trends (24h/7d/30d/90d), sparklines, and human-readable labels.

4. **View history:**
   ```
   GET /api/v1/stats/agents_discovered?period=7d
   ```

## Auth Model

- **Read endpoints** (GET stats, alerts, health): No auth required — designed for LAN usage
- **Write endpoints** (POST stats, prune, DELETE): `Authorization: Bearer <manage_key>`
- Manage key is auto-generated on first run and printed to stdout

## Core Patterns

### Batch Stat Submission
Submit up to 100 metrics per batch. Each stat needs a `key` (1-100 chars) and `value` (number):
```json
POST /api/v1/stats
[
  {"key": "tests_total", "value": 1264},
  {"key": "repos_count", "value": 9},
  {"key": "siblings_active", "value": 3, "metadata": {"names": ["Forge","Drift","Lux"]}}
]
```

### Automatic Alerts
Alerts fire automatically when a metric changes ≥10% over 24 hours:
- **alert** level: ≥10% change
- **hot** level: ≥25% change
- Debounced to max 1 alert per key per 6 hours

### Time-Series History
```
GET /api/v1/stats/<key>?period=24h    # Last 24 hours
GET /api/v1/stats/<key>?period=7d     # Last 7 days
GET /api/v1/stats/<key>?start=2026-02-01&end=2026-02-15  # Custom range
```

### Data Retention
Stats older than 90 days are auto-pruned on startup. Manual prune: `POST /api/v1/stats/prune`.

## Known Metric Keys

| Key | Description |
|-----|-------------|
| agents_discovered | Total agents tracked |
| moltbook_interesting | Interesting Moltbook posts found |
| moltbook_spam | Spam posts filtered |
| repos_count | Active repositories |
| tests_total | Total test count across repos |
| deploys_count | Deployments completed |
| commits_total | Total commits |
| siblings_count | Sibling agents known |
| siblings_active | Currently active siblings |
| cron_jobs_active | Running cron jobs |

## Gotchas

- Metric keys are case-sensitive and must be 1-100 characters
- Empty stat arrays return 400, max 100 per batch
- Trend percentages are `null` when the starting value was 0 (can't compute % from zero)
- Sparkline data covers last 24h with 12 data points (2-hour buckets)
- Alert `change_pct` is rounded to 1 decimal place

## Full API Reference

See `/llms.txt` for complete endpoint documentation and `/openapi.json` for the OpenAPI 3.0.3 specification.
"#;

fn compute_trend(db: &Db, key: &str, current: f64, since: chrono::DateTime<Utc>) -> TrendData {
    // Try exact point at/before window start, fall back to earliest point within window
    let start = db.get_stat_at_time(key, &since.to_rfc3339())
        .or_else(|| db.get_earliest_stat_since(key, &since.to_rfc3339()));
    match start {
        Some(s) if s != 0.0 => TrendData {
            start: Some(s),
            end: current,
            change: Some(current - s),
            pct: Some(((current - s) / s) * 100.0),
        },
        Some(s) => TrendData {
            start: Some(s),
            end: current,
            change: Some(current - s),
            pct: None,
        },
        None => TrendData {
            start: None,
            end: current,
            change: None,
            pct: None,
        },
    }
}
