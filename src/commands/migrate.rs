use crate::context::Context;
use crate::error::Result;

/// Very lightweight migrate for the krebslog *cache* DB (not the source tools).
/// In skeleton we just acknowledge; real schema + rusqlite migrations come with caching work.
pub fn handle_migrate(status: bool, dry_run: bool, force: bool, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "command": "migrate",
                "status": status,
                "dry_run": dry_run,
                "force": force,
                "current_version": 0,
                "latest_version": 0,
                "note": "krebslog cache migrations not wired in skeleton; sources are never touched directly"
            })
        );
    } else if !quiet {
        if status {
            println!("krebslog cache schema: v0 (skeleton — no cache DB created yet)");
        } else {
            println!("migrate (skeleton) — cache DB will use simple rusqlite schema when caching is enabled");
        }
    }
    Ok(())
}
