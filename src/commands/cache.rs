use crate::cli::CacheAction;
use crate::context::Context;
use crate::db::{clear_cache, get_body_cache_stats, open_db, resolve_db_path};
use crate::error::Result;

pub fn handle_cache(action: CacheAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    let db_override = ctx.db.as_deref();
    let db_path = resolve_db_path(db_override);

    match action {
        CacheAction::Clear { force: _ } => {
            // Best-effort: open (which migrates) then clear known tables.
            // Even under --no-cache the user may want to manage the cache file.
            match open_db(db_override) {
                Ok(conn) => {
                    let _ = clear_cache(&conn);
                }
                Err(_) => {
                    // If open fails (permissions etc), fall back to removing the file.
                    let _ = std::fs::remove_file(&db_path);
                }
            }
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "cleared": db_path.display().to_string() })
                );
            } else if !quiet {
                println!("cache cleared: {}", db_path.display());
            }
        }
        CacheAction::Info => {
            let exists = db_path.exists();
            let size = if exists {
                std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };

            let (body_count, body_oldest, body_newest) = if exists {
                match open_db(db_override) {
                    Ok(conn) => {
                        let (c, o, n) = get_body_cache_stats(&conn).unwrap_or((0, None, None));
                        (c, o, n)
                    }
                    Err(_) => (0, None, None),
                }
            } else {
                (0, None, None)
            };

            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "path": db_path.display().to_string(),
                        "exists": exists,
                        "size_bytes": size,
                        "body_measurements": {
                            "count": body_count,
                            "oldest": body_oldest,
                            "newest": body_newest
                        },
                        "note": "Body measurements (from bodylog) are cached sparsely when --no-cache is not used. Full daily grain + krebs derived rows land with the aggregation engine."
                    })
                );
            } else if !quiet {
                println!("krebslog cache");
                println!("  path:   {}", db_path.display());
                println!("  exists: {}", exists);
                println!("  size:   {} bytes", size);
                println!(
                    "  body:   {} measurements ({}..{})",
                    body_count,
                    body_oldest.as_deref().unwrap_or("-"),
                    body_newest.as_deref().unwrap_or("-")
                );
                println!("  (body data cached from live bodylog pulls; fully disableable with --no-cache)");
            }
        }
    }
    Ok(())
}
