use crate::context::Context;
use crate::db::{open_db, resolve_db_path};
use crate::error::Result;
use rusqlite::Connection;

/// Real (but minimal) migration for the krebslog *cache* DB (not the source tools).
/// Uses PRAGMA user_version + embedded steps (pull_log + body tables for Phase 5 of spec/03).
pub fn handle_migrate(status: bool, dry_run: bool, force: bool, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    let db_override = ctx.db.as_deref();
    let db_path = resolve_db_path(db_override);

    // Open will run migrate to latest. For status we query without side effects.
    let current = if status || dry_run {
        // Peek at current version without forcing writes if possible.
        match Connection::open(&db_path) {
            Ok(conn) => conn
                .query_row("PRAGMA user_version;", [], |row| row.get(0))
                .unwrap_or(0),
            Err(_) => 0,
        }
    } else {
        // Normal run: open (creates + migrates)
        match open_db(db_override) {
            Ok(conn) => conn
                .query_row("PRAGMA user_version;", [], |row| row.get(0))
                .unwrap_or(0),
            Err(e) => {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": false, "error": e.to_string() })
                    );
                }
                return Err(e);
            }
        }
    };

    // Target is defined inside db::migrate (v3 adds config kv table; v2 introduced body + pull_log per spec/03-bodylog).
    let latest = 3;

    if force && !dry_run && !status {
        // Force: re-open (idempotent) and optionally re-apply by touching a marker.
        // Since our migrate is IF NOT EXISTS + version guard, "force" just ensures open/migrate ran.
        let _ = open_db(db_override)?;
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "command": "migrate",
                "status": status,
                "dry_run": dry_run,
                "force": force,
                "current_version": current,
                "latest_version": latest,
                "path": db_path.display().to_string(),
                "note": "v3 adds config kv (prefs + scalar overrides). v2 adds pull_log (freshness for all sources) + body_measurements + body_profile (sparse cache of bodylog data). Never reads source tool DBs."
            })
        );
    } else if !quiet {
        if status {
            println!(
                "krebslog cache schema: v{} (latest {}) at {}",
                current,
                latest,
                db_path.display()
            );
            if current < latest {
                println!(
                    "  (run `krebslog migrate --force` to apply pending config (or body/pull_log) tables)"
                );
            }
        } else if dry_run {
            println!(
                "migrate (dry-run): current v{}, would ensure v{} (config + body + pull freshness) at {}",
                current,
                latest,
                db_path.display()
            );
        } else {
            println!(
                "migrate: ensured v{} (config + body_measurements + pull_log) at {}",
                latest,
                db_path.display()
            );
        }
    }
    Ok(())
}
