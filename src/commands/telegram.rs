use crate::cli::TelegramAction;
use crate::commands::report::build_compact_krebs_status_for_agent;
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
            let is_krebs = r#type == "krebs" || r#type == "krebs-status";
            if is_krebs {
                // Produce a real Telegram-ready bundle for krebs-status using the shared compact builder (Phase 3 polish).
                let ks = build_compact_krebs_status_for_agent(&period, ctx);
                let one = ks
                    .get("one_sentence")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Krebs status available via report krebs-status.");
                let flux = ks.get("flux_proxy").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let caption = format!(
                    "*Krebs Status* \\({}\\)\n\n{}\n\nFlux: *{:.1}/10* \\| Body trend: `{}`\n\nUse `krebslog report krebs-status --since \"{}\"` for full details \\+ body validation\\.",
                    period, one, flux, ks.get("body_weight_trend").and_then(|v| v.as_str()).unwrap_or("?"), period
                );
                let suggested_image = format!(
                    "~/reports/krebslog/krebs-status-{}.png",
                    period.replace(' ', "_")
                );

                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "success": true,
                            "type": "krebs-status",
                            "period": period,
                            "post_ready": post_ready,
                            "caption_markdownv2": caption,
                            "suggested_image": suggested_image,
                            "krebs_status": ks
                        }))
                        .unwrap()
                    );
                } else if !quiet {
                    println!(
                        "{}",
                        caption
                            .replace("\\(", "(")
                            .replace("\\)", ")")
                            .replace("\\|", "|")
                    );
                    println!("Suggested image: {}", suggested_image);
                    if post_ready {
                        println!("(post this caption + the image with your Telegram bot)");
                    }
                }
            } else {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": true, "type": r#type, "period": period, "post_ready": post_ready, "note": "skeleton (only krebs-status produces real output today)" })
                    );
                } else if !quiet {
                    println!(
                        "telegram report --type {} --period {} (skeleton)",
                        r#type, period
                    );
                }
            }
        }
    }
    Ok(())
}
