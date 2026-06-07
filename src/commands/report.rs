use crate::cli::ReportAction;
use crate::context::Context;
use crate::error::Result;

/// Stubs for report commands. Full implementation comes after data pulling + aggregation engine.
pub fn handle_report(action: ReportAction, ctx: &Context) -> Result<()> {
    // For now we only use the output flags; the rest (db, bins, no_cache) will be
    // relevant once we have real aggregation + caching + bodylog integration.
    let json = ctx.json;
    let quiet = ctx.quiet;
    match action {
        ReportAction::Daily {
            date,
            include_image,
            output_dir,
        } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "command": "report daily",
                        "date": date,
                        "include_image": include_image,
                        "output_dir": output_dir,
                        "note": "skeleton: full daily report + derived krebs/redox not yet implemented"
                    })
                );
            } else if !quiet {
                println!(
                    "report daily --date {} (skeleton — not fully implemented)",
                    date
                );
                if include_image {
                    println!("(would also produce an image)");
                }
            }
        }
        ReportAction::Weekly { since, until } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "report weekly", "since": since, "until": until, "note": "skeleton" })
                );
            } else if !quiet {
                println!("report weekly --since {} (skeleton)", since);
            }
        }
        ReportAction::KrebsFlux { period, output } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "report krebs-flux", "period": period, "output": output, "note": "skeleton" })
                );
            } else if !quiet {
                println!("report krebs-flux --period {} (skeleton)", period);
            }
        }
        ReportAction::RedoxBalance { since, until } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "report redox-balance", "since": since, "until": until, "note": "skeleton" })
                );
            } else if !quiet {
                println!("report redox-balance --since {} (skeleton)", since);
            }
        }
        ReportAction::EnergyBalance {
            since,
            until: _,
            include_body_trends,
        } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "report energy-balance", "since": since, "include_body_trends": include_body_trends, "note": "skeleton" })
                );
            } else if !quiet {
                println!("report energy-balance (skeleton)");
            }
        }
        ReportAction::Correlations { x, y, period } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "report correlations", "x": x, "y": y, "period": period, "note": "skeleton" })
                );
            } else if !quiet {
                println!("report correlations --x {} --y {} (skeleton)", x, y);
            }
        }
    }
    Ok(())
}
