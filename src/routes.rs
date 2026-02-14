use rocket::serde::json::Json;
use rocket::http::{ContentType, Status};
use rocket::State;
use std::sync::Arc;
use chrono::{Utc, Duration};

use crate::db::Db;
use crate::auth::ManageKey;
use crate::models::*;

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

    let now = Utc::now().to_rfc3339();
    let mut accepted = 0;

    for stat in stats.iter() {
        if stat.key.is_empty() || stat.key.len() > 100 {
            continue;
        }
        let meta = stat.metadata.as_ref().map(|m| m.to_string());
        db.insert_stat(&stat.key, stat.value, &now, meta.as_deref());
        accepted += 1;
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

## Auth
- Read endpoints: No auth (local network only)
- Write endpoints: Bearer token (manage key generated on first run)

## Data Retention
- Auto-prune on startup: stats older than 90 days are automatically deleted
- Manual prune: POST /api/v1/stats/prune (auth required)

## Known Metric Keys
agents_discovered, moltbook_interesting, moltbook_spam, outreach_sent, outreach_received,
repos_count, tests_total, deploys_count, commits_total, twitter_headlines, siblings_count
")
}

// ── OpenAPI spec ──
#[get("/openapi.json")]
pub fn openapi_spec() -> (ContentType, &'static str) {
    (ContentType::JSON, include_str!("../openapi.json"))
}

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
