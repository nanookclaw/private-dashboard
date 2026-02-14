use rusqlite::{Connection, params};
use std::sync::Mutex;

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL,
                value REAL NOT NULL,
                recorded_at TEXT NOT NULL,
                metadata TEXT,
                seq INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_stats_key_time ON stats(key, recorded_at);
            CREATE INDEX IF NOT EXISTS idx_stats_seq ON stats(seq);
            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS alert_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL,
                level TEXT NOT NULL,
                value REAL NOT NULL,
                change_pct REAL NOT NULL,
                triggered_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_alerts_key_time ON alert_log(key, triggered_at);
            CREATE INDEX IF NOT EXISTS idx_alerts_time ON alert_log(triggered_at);"
        )?;

        Ok(Db { conn: Mutex::new(conn) })
    }

    pub fn insert_stat(&self, key: &str, value: f64, recorded_at: &str, metadata: Option<&str>) -> i64 {
        let conn = self.conn.lock().unwrap();
        let seq = {
            let max: Option<i64> = conn
                .query_row("SELECT MAX(seq) FROM stats", [], |row| row.get(0))
                .unwrap_or(None);
            max.unwrap_or(0) + 1
        };
        conn.execute(
            "INSERT INTO stats (key, value, recorded_at, metadata, seq) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![key, value, recorded_at, metadata, seq],
        ).unwrap();
        seq
    }

    pub fn get_latest_stats(&self) -> Vec<LatestStat> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.key, s.value, s.recorded_at, s.metadata
             FROM stats s
             INNER JOIN (SELECT key, MAX(seq) as max_seq FROM stats GROUP BY key) latest
             ON s.key = latest.key AND s.seq = latest.max_seq
             ORDER BY s.key"
        ).unwrap();

        stmt.query_map([], |row| {
            Ok(LatestStat {
                key: row.get(0)?,
                value: row.get(1)?,
                recorded_at: row.get(2)?,
                metadata: row.get(3)?,
            })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_stat_at_time(&self, key: &str, at: &str) -> Option<f64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM stats WHERE key = ?1 AND recorded_at <= ?2 ORDER BY recorded_at DESC LIMIT 1",
            params![key, at],
            |row| row.get(0),
        ).ok()
    }

    /// Get the earliest stat value at or after a given time (fallback for trends when no data before window start)
    pub fn get_earliest_stat_since(&self, key: &str, since: &str) -> Option<f64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM stats WHERE key = ?1 AND recorded_at >= ?2 ORDER BY recorded_at ASC LIMIT 1",
            params![key, since],
            |row| row.get(0),
        ).ok()
    }

    pub fn get_stat_history(&self, key: &str, since: &str) -> Vec<StatPoint> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT value, recorded_at FROM stats WHERE key = ?1 AND recorded_at >= ?2 ORDER BY recorded_at ASC"
        ).unwrap();

        stmt.query_map(params![key, since], |row| {
            Ok(StatPoint {
                value: row.get(0)?,
                recorded_at: row.get(1)?,
            })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_stat_history_range(&self, key: &str, start: &str, end: &str) -> Vec<StatPoint> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT value, recorded_at FROM stats WHERE key = ?1 AND recorded_at >= ?2 AND recorded_at <= ?3 ORDER BY recorded_at ASC"
        ).unwrap();

        stmt.query_map(params![key, start, end], |row| {
            Ok(StatPoint {
                value: row.get(0)?,
                recorded_at: row.get(1)?,
            })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_sparkline(&self, key: &str, since: &str, points: usize) -> Vec<f64> {
        let history = self.get_stat_history(key, since);
        if history.is_empty() {
            return vec![];
        }
        if history.len() <= points {
            return history.iter().map(|p| p.value).collect();
        }
        // Downsample: pick evenly spaced points
        let step = history.len() as f64 / points as f64;
        (0..points)
            .map(|i| {
                let idx = (i as f64 * step) as usize;
                history[idx.min(history.len() - 1)].value
            })
            .collect()
    }

    pub fn get_manage_key(&self) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM config WHERE key = 'manage_key'",
            [],
            |row| row.get(0),
        ).ok()
    }

    pub fn set_manage_key(&self, key: &str) {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES ('manage_key', ?1)",
            params![key],
        ).unwrap();
    }

    pub fn get_all_keys(&self) -> Vec<String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT DISTINCT key FROM stats ORDER BY key").unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    pub fn get_stat_count(&self) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM stats", [], |row| row.get(0)).unwrap_or(0)
    }

    /// Delete stats older than `days` days. Returns number of rows deleted.
    pub fn prune_old_stats(&self, days: i64) -> i64 {
        let conn = self.conn.lock().unwrap();
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();
        conn.execute(
            "DELETE FROM stats WHERE recorded_at < ?1",
            rusqlite::params![cutoff_str],
        ).unwrap_or(0) as i64
    }

    /// Delete all stats for a given key. Returns number of rows deleted.
    pub fn delete_stats_by_key(&self, key: &str) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM stats WHERE key = ?1",
            rusqlite::params![key],
        ).unwrap_or(0) as i64
    }

    /// Insert an alert log entry.
    pub fn insert_alert(&self, key: &str, level: &str, value: f64, change_pct: f64, triggered_at: &str) {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO alert_log (key, level, value, change_pct, triggered_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![key, level, value, change_pct, triggered_at],
        ).unwrap();
    }

    /// Get the most recent alert for a key (to debounce).
    pub fn get_last_alert_time(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT triggered_at FROM alert_log WHERE key = ?1 ORDER BY triggered_at DESC LIMIT 1",
            params![key],
            |row| row.get(0),
        ).ok()
    }

    /// Get recent alerts, newest first.
    pub fn get_alerts(&self, limit: i64) -> Vec<AlertEntry> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT key, level, value, change_pct, triggered_at FROM alert_log ORDER BY triggered_at DESC LIMIT ?1"
        ).unwrap();

        stmt.query_map(params![limit], |row| {
            Ok(AlertEntry {
                key: row.get(0)?,
                level: row.get(1)?,
                value: row.get(2)?,
                change_pct: row.get(3)?,
                triggered_at: row.get(4)?,
            })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    /// Get alerts for a specific key.
    pub fn get_alerts_for_key(&self, key: &str, limit: i64) -> Vec<AlertEntry> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT key, level, value, change_pct, triggered_at FROM alert_log WHERE key = ?1 ORDER BY triggered_at DESC LIMIT ?2"
        ).unwrap();

        stmt.query_map(params![key, limit], |row| {
            Ok(AlertEntry {
                key: row.get(0)?,
                level: row.get(1)?,
                value: row.get(2)?,
                change_pct: row.get(3)?,
                triggered_at: row.get(4)?,
            })
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    /// Count total alerts.
    pub fn get_alert_count(&self) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM alert_log", [], |row| row.get(0)).unwrap_or(0)
    }

    /// Get the oldest recorded_at timestamp across all stats.
    pub fn get_oldest_stat_time(&self) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT MIN(recorded_at) FROM stats",
            [],
            |row| row.get(0),
        ).ok()
    }
}

pub struct LatestStat {
    pub key: String,
    pub value: f64,
    pub recorded_at: String,
    #[allow(dead_code)]
    pub metadata: Option<String>,
}

pub struct StatPoint {
    pub value: f64,
    pub recorded_at: String,
}

pub struct AlertEntry {
    pub key: String,
    pub level: String,
    pub value: f64,
    pub change_pct: f64,
    pub triggered_at: String,
}
