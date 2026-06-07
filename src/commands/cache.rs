use crate::cli::CacheAction;
use crate::context::Context;
use crate::error::Result;
use directories::ProjectDirs;
use std::path::PathBuf;

pub fn handle_cache(action: CacheAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    let db_override = ctx.db.as_deref();
    let db_path = resolve_db_path(db_override);

    match action {
        CacheAction::Clear { force: _ } => {
            // In real impl: DELETE FROM daily_aggregates etc, or just rm the file (with care).
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "cleared": db_path.display().to_string(), "note": "skeleton (no-op)" })
                );
            } else if !quiet {
                println!(
                    "cache clear (skeleton) — would remove or truncate {}",
                    db_path.display()
                );
            }
        }
        CacheAction::Info => {
            let exists = db_path.exists();
            let size = if exists {
                std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "path": db_path.display().to_string(),
                        "exists": exists,
                        "size_bytes": size
                    })
                );
            } else if !quiet {
                println!("krebslog cache");
                println!("  path:   {}", db_path.display());
                println!("  exists: {}", exists);
                println!("  size:   {} bytes", size);
                println!("  (cache is optional and fully disableable with --no-cache)");
            }
        }
    }
    Ok(())
}

fn resolve_db_path(override_path: Option<&str>) -> PathBuf {
    if let Some(p) = override_path {
        return PathBuf::from(p);
    }
    if let Some(proj) = ProjectDirs::from("com", "krebslog", "krebslog") {
        let mut p = proj.data_dir().to_path_buf();
        p.push("krebslog.db");
        // best effort create dir
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
