use crate::cli::ConfigAction;
use crate::context::Context;
use crate::error::Result;

pub fn handle_config(action: ConfigAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    match action {
        ConfigAction::Get { key } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "key": key, "value": null, "note": "config store not implemented in skeleton" })
                );
            } else if !quiet {
                println!("config get {:?} (skeleton — config layer later)", key);
            }
        }
        ConfigAction::Set { key, value } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "set": key, "value": value, "note": "skeleton" })
                );
            } else if !quiet {
                println!("config set {} = {} (skeleton)", key, value);
            }
        }
        ConfigAction::Show => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "config": {}, "note": "skeleton defaults only" })
                );
            } else if !quiet {
                println!("Effective config (skeleton defaults):");
                println!("  nutlog.bin = (auto)");
                println!("  repslog.bin = (auto)");
                println!("  bodylog.bin = (auto)");
                println!("  reports.dir = ~/reports/krebslog");
            }
        }
        ConfigAction::Reset { force: _ } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "reset": true, "note": "skeleton" })
                );
            } else if !quiet {
                println!("config reset (skeleton)");
            }
        }
    }
    Ok(())
}
