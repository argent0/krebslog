use crate::cli::TelegramAction;
use crate::context::Context;
use crate::error::Result;

pub fn handle_telegram(action: TelegramAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    match action {
        TelegramAction::Daily { date, with_json } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "command": "telegram daily",
                        "date": date,
                        "with_json": with_json,
                        "note": "skeleton: produces MarkdownV2 + image path in real impl"
                    })
                );
            } else if !quiet {
                println!("telegram daily --date {} (skeleton)", date);
            }
        }
        TelegramAction::Report {
            r#type,
            period,
            post_ready,
        } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "type": r#type, "period": period, "post_ready": post_ready, "note": "skeleton" })
                );
            } else if !quiet {
                println!(
                    "telegram report --type {} --period {} (skeleton)",
                    r#type, period
                );
            }
        }
    }
    Ok(())
}
