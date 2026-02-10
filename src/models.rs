use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct StatInput {
    pub key: String,
    pub value: f64,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct StatSubmitResponse {
    pub accepted: usize,
}

#[derive(Debug, Serialize)]
pub struct TrendData {
    pub start: Option<f64>,
    pub end: f64,
    pub change: Option<f64>,
    pub pct: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct StatSummary {
    pub key: String,
    pub label: String,
    pub current: f64,
    pub trends: Trends,
    pub sparkline_24h: Vec<f64>,
    pub last_updated: String,
}

#[derive(Debug, Serialize)]
pub struct Trends {
    #[serde(rename = "24h")]
    pub h24: TrendData,
    #[serde(rename = "7d")]
    pub d7: TrendData,
    #[serde(rename = "30d")]
    pub d30: TrendData,
    #[serde(rename = "90d")]
    pub d90: TrendData,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub stats: Vec<StatSummary>,
}

#[derive(Debug, Serialize)]
pub struct StatHistoryResponse {
    pub key: String,
    pub points: Vec<StatPointOut>,
}

#[derive(Debug, Serialize)]
pub struct StatPointOut {
    pub value: f64,
    pub recorded_at: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub stats_count: i64,
    pub keys_count: usize,
    pub retention_days: i64,
    pub oldest_stat: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PruneResponse {
    pub deleted: i64,
    pub retention_days: i64,
    pub remaining: i64,
}

/// Human-readable labels for stat keys
pub fn key_label(key: &str) -> String {
    match key {
        "agents_discovered" => "Agents Discovered".into(),
        "moltbook_interesting" => "Moltbook Interesting".into(),
        "moltbook_spam" => "Moltbook Spam".into(),
        "outreach_sent" => "Outreach Sent".into(),
        "outreach_received" => "Outreach Received".into(),
        "repos_count" => "Repos".into(),
        "tests_total" => "Total Tests".into(),
        "deploys_count" => "Deploys".into(),
        "commits_total" => "Total Commits".into(),
        "twitter_headlines" => "Twitter Headlines".into(),
        "siblings_count" => "Sibling Agents".into(),
        "siblings_active" => "Siblings Active".into(),
        "moltbook_health" => "Moltbook Health".into(),
        "moltbook_my_posts" => "Moltbook Posts".into(),
        "twitter_accounts" => "Twitter Accounts".into(),
        _ => key.replace('_', " ").to_string(),
    }
}
