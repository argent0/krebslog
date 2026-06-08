use crate::error::{KrebslogError, Result};
use chrono::Utc;
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::path::PathBuf;

/// Resolve the krebslog cache DB path (XDG or override).
/// Shared so commands/cache, data pulls, and reports all agree.
pub fn resolve_db_path(override_path: Option<&str>) -> PathBuf {
    if let Some(p) = override_path {
        return PathBuf::from(p);
    }
    if let Some(proj) = ProjectDirs::from("com", "krebslog", "krebslog") {
        let mut p = proj.data_dir().to_path_buf();
        p.push("krebslog.db");
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        p
    } else {
        let mut p = PathBuf::from(std::env::var("HOME").unwrap_or("/tmp".into()));
        p.push(".local/share/krebslog/krebslog.db");
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        p
    }
}

/// Open the cache DB (creating directories as needed) and ensure schema is at latest version.
/// Returns a ready-to-use Connection. Callers must respect ctx.no_cache for data freshness paths.
pub fn open_db(override_path: Option<&str>) -> Result<Connection> {
    let path = resolve_db_path(override_path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(&path).map_err(|e| KrebslogError::Database(e.to_string()))?;
    // Recommended pragmas for a local CLI cache
    let _ = conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    );
    migrate(&conn)?;
    Ok(conn)
}

/// Current target schema version for the krebslog cache.
/// v2 introduced pull_log + body_measurements (per spec/03-bodylog Phase 5).
/// v3 adds simple key/value config table for persisted user prefs and formula overrides.
const TARGET_VERSION: i32 = 3;

/// Ensure the DB is migrated to TARGET_VERSION using PRAGMA user_version.
/// Idempotent; uses simple versioned steps.
fn migrate(conn: &Connection) -> Result<()> {
    let current: i32 = conn
        .query_row("PRAGMA user_version;", [], |row| row.get(0))
        .unwrap_or(0);

    if current >= TARGET_VERSION {
        return Ok(());
    }

    // v1 -> v2: core tables for pull freshness and sparse body data (bodylog first-class)
    if current < 2 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS pull_log (
                source TEXT NOT NULL,
                entity TEXT NOT NULL,
                last_since TEXT,
                last_until TEXT,
                last_count INTEGER,
                pulled_at TEXT NOT NULL,
                PRIMARY KEY (source, entity)
            );

            CREATE TABLE IF NOT EXISTS body_measurements (
                date TEXT PRIMARY KEY,
                weight_kg REAL,
                body_fat_pct REAL,
                skeletal_muscle_pct REAL,
                visceral_fat_level INTEGER,
                bmi REAL,
                resting_metabolism_kcal INTEGER,
                raw TEXT,
                pulled_at TEXT NOT NULL
            );

            -- Lightweight single-row last-known profile/config from bodylog (height, dob, etc.)
            CREATE TABLE IF NOT EXISTS body_profile (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                height_cm REAL,
                date_of_birth TEXT,
                raw TEXT,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|e| KrebslogError::Database(format!("migration v2 failed: {}", e)))?;
    }

    // v2 -> v3: simple kv store for config (prefs + overridable scalars like base metabolism)
    if current < 3 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|e| KrebslogError::Database(format!("migration v3 failed: {}", e)))?;
    }

    // Future versions would add ALTER TABLE or new tables here, then fall through.

    conn.execute(&format!("PRAGMA user_version = {};", TARGET_VERSION), [])
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(())
}

/// Record (or update) a successful pull for freshness tracking in data status.
pub fn record_pull(
    conn: &Connection,
    source: &str,
    entity: &str,
    since: &str,
    until: Option<&str>,
    count: Option<i64>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO pull_log (source, entity, last_since, last_until, last_count, pulled_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(source, entity) DO UPDATE SET
            last_since=excluded.last_since,
            last_until=excluded.last_until,
            last_count=excluded.last_count,
            pulled_at=excluded.pulled_at;",
        params![source, entity, since, until, count, now],
    )
    .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(())
}

/// Return the most recent pulled_at (RFC3339) for a given source/entity, if any.
#[allow(dead_code)]
pub fn get_last_pull(conn: &Connection, source: &str, entity: &str) -> Result<Option<String>> {
    let val: Option<String> = conn
        .query_row(
            "SELECT pulled_at FROM pull_log WHERE source = ?1 AND entity = ?2;",
            params![source, entity],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(val)
}

/// Return the latest pulled_at across any entity for the source (for high-level status display).
pub fn get_last_pull_for_source(conn: &Connection, source: &str) -> Result<Option<String>> {
    let val: Option<String> = conn
        .query_row(
            "SELECT pulled_at FROM pull_log WHERE source = ?1 ORDER BY pulled_at DESC LIMIT 1;",
            params![source],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(val)
}

/// Store/upsert body measurements from a list of measurement records (the shape from
/// bodylog measurement list or the "series" array from bodylog report weight/summary).
/// Each record should have a "date" (YYYY-MM-DD) and optional weight_kg etc.
/// Returns number of rows inserted or replaced.
pub fn store_body_measurements(conn: &Connection, records: &[Value]) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }
    let now = Utc::now().to_rfc3339();
    let mut count = 0usize;

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| KrebslogError::Database(e.to_string()))?;

    for rec in records {
        let date = rec
            .get("date")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());
        if date.is_none() {
            // series items or measurement items must have date per bodylog contract
            continue;
        }
        let date = date.unwrap();

        let weight = rec.get("weight_kg").and_then(|v| v.as_f64());
        let fat = rec.get("body_fat_pct").and_then(|v| v.as_f64());
        let muscle = rec.get("skeletal_muscle_pct").and_then(|v| v.as_f64());
        let visceral = rec
            .get("visceral_fat_level")
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)));
        let bmi = rec.get("bmi").and_then(|v| v.as_f64());
        let rest = rec
            .get("resting_metabolism_kcal")
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)));

        let raw = serde_json::to_string(rec).unwrap_or_default();

        tx.execute(
            "INSERT INTO body_measurements
                (date, weight_kg, body_fat_pct, skeletal_muscle_pct, visceral_fat_level, bmi, resting_metabolism_kcal, raw, pulled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(date) DO UPDATE SET
                weight_kg=excluded.weight_kg,
                body_fat_pct=excluded.body_fat_pct,
                skeletal_muscle_pct=excluded.skeletal_muscle_pct,
                visceral_fat_level=excluded.visceral_fat_level,
                bmi=excluded.bmi,
                resting_metabolism_kcal=excluded.resting_metabolism_kcal,
                raw=excluded.raw,
                pulled_at=excluded.pulled_at;",
            params![
                date,
                weight,
                fat,
                muscle,
                visceral,
                bmi,
                rest,
                raw,
                now
            ],
        )
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
        count += 1;
    }

    tx.commit()
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(count)
}

/// Fetch cached body measurements for the inclusive [since, until] window as an array of
/// measurement objects (newest first to match bodylog list convention).
pub fn get_body_measurements(
    conn: &Connection,
    since: &str,
    until: Option<&str>,
) -> Result<Vec<Value>> {
    // Simple & reliable: query >= since (ordered newest first), then client-filter <= until if given.
    // Body measurement count is tiny; this avoids closure type mismatches in query_map branches.
    let sql = "SELECT raw FROM body_measurements WHERE date >= ?1 ORDER BY date DESC;";
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| KrebslogError::Database(e.to_string()))?;

    let rows = stmt
        .query_map(params![since], |row| {
            let raw: String = row.get(0)?;
            Ok(raw)
        })
        .map_err(|e| KrebslogError::Database(e.to_string()))?;

    let mut out = Vec::new();
    for r in rows {
        let raw = r.map_err(|e| KrebslogError::Database(e.to_string()))?;
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            if let Some(u) = until {
                if let Some(d) = v.get("date").and_then(|x| x.as_str()) {
                    if d > u {
                        continue;
                    }
                }
            }
            out.push(v);
        }
    }
    Ok(out)
}

/// Return the single most recent cached body measurement (if any), parsed.
pub fn get_latest_body_measurement(conn: &Connection) -> Result<Option<Value>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT raw FROM body_measurements ORDER BY date DESC LIMIT 1;",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| KrebslogError::Database(e.to_string()))?;

    match raw {
        Some(s) => match serde_json::from_str::<Value>(&s) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

/// Store last-seen body profile/config (from bodylog config show). Idempotent single row.
pub fn store_body_profile(conn: &Connection, profile: &Value) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let height = profile.get("height_cm").and_then(|v| v.as_f64());
    let dob = profile
        .get("date_of_birth")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let raw = serde_json::to_string(profile).unwrap_or_default();

    conn.execute(
        "INSERT INTO body_profile (id, height_cm, date_of_birth, raw, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
            height_cm=excluded.height_cm,
            date_of_birth=excluded.date_of_birth,
            raw=excluded.raw,
            updated_at=excluded.updated_at;",
        params![height, dob, raw, now],
    )
    .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(())
}

/// Fetch cached body profile if present.
pub fn get_body_profile(conn: &Connection) -> Result<Option<Value>> {
    let raw: Option<String> = conn
        .query_row("SELECT raw FROM body_profile WHERE id = 1;", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| KrebslogError::Database(e.to_string()))?;

    match raw {
        Some(s) => match serde_json::from_str::<Value>(&s) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

/// Clear all cached data (used by `cache clear`).
pub fn clear_cache(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DELETE FROM body_measurements;
         DELETE FROM pull_log;
         DELETE FROM body_profile;
         VACUUM;",
    )
    .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(())
}

/// For cache info: return basic stats about body data presence.
pub fn get_body_cache_stats(conn: &Connection) -> Result<(i64, Option<String>, Option<String>)> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM body_measurements;", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let oldest: Option<String> = conn
        .query_row("SELECT MIN(date) FROM body_measurements;", [], |row| {
            row.get(0)
        })
        .optional()
        .unwrap_or(None);

    let newest: Option<String> = conn
        .query_row("SELECT MAX(date) FROM body_measurements;", [], |row| {
            row.get(0)
        })
        .optional()
        .unwrap_or(None);

    Ok((count, oldest, newest))
}

// ---------------- Config (v3) simple key/value store ----------------

/// Set or update a config value (persisted across runs, overridable via DB).
pub fn set_config_value(conn: &Connection, key: &str, value: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO config (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at;",
        params![key, value, now],
    )
    .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(())
}

/// Get a single config value if present.
pub fn get_config_value(conn: &Connection, key: &str) -> Result<Option<String>> {
    let val: Option<String> = conn
        .query_row(
            "SELECT value FROM config WHERE key = ?1;",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(val)
}

/// Return all config entries as (key, value) pairs (for show / effective config).
pub fn get_all_config(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM config ORDER BY key;")
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| KrebslogError::Database(e.to_string()))?);
    }
    Ok(out)
}

/// Delete all config rows (used by config reset).
pub fn reset_config(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM config;", [])
        .map_err(|e| KrebslogError::Database(e.to_string()))?;
    Ok(())
}
