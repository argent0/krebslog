use crate::cli::TelegramAction;
use crate::commands::report::build_compact_krebs_status_for_agent;
use crate::context::Context;
use crate::error::Result;
use crate::utils::{resolve_bin, run_external_json};

pub fn handle_telegram(action: TelegramAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    match action {
        TelegramAction::Daily { date, with_json } => {
            // Real daily: reuse compact krebs status (or light body) + produce usable MDV2 caption + suggested image.
            let body_note = get_telegram_body_note(&date, ctx);
            let ks = crate::commands::report::build_compact_krebs_status_for_agent(&date, ctx);
            let one = ks
                .get("one_sentence")
                .and_then(|v| v.as_str())
                .unwrap_or("Daily metabolic snapshot.");
            let flux = ks.get("flux_proxy").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let caption = format!(
                "*Daily Krebs* \\({}\\)\n\n{}\n\nFlux: *{:.1}/10* \\| Body: `{}`\n\n`krebslog report daily --date \"{}\"` or `image krebs-cycle`.",
                date, one, flux, ks.get("body_weight_trend").and_then(|v| v.as_str()).unwrap_or("?"), date
            );
            let suggested = format!(
                "~/reports/krebslog/krebs-cycle-{}.svg",
                date.replace('-', "")
            );

            if json {
                let mut out = serde_json::json!({
                    "success": true,
                    "command": "telegram daily",
                    "date": date,
                    "with_json": with_json,
                    "caption_markdownv2": caption,
                    "suggested_image": suggested,
                    "krebs_status": ks
                });
                if let Some(b) = body_note {
                    out["body"] = b;
                }
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            } else if !quiet {
                println!(
                    "{}",
                    caption
                        .replace("\\(", "(")
                        .replace("\\)", ")")
                        .replace("\\|", "|")
                );
                println!("Suggested image: {}", suggested);
                if with_json {
                    println!("(JSON block emitted above for bots)");
                }
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
                // Real support for other common types by delegating to compact or simple summary.
                let ks =
                    crate::commands::report::build_compact_krebs_status_for_agent(&period, ctx);
                let flux = ks.get("flux_proxy").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let caption = match r#type.as_str() {
                    "daily" => format!("*Daily* \\({}\\)\nFlux {:.1}/10 — use report daily or image krebs-cycle.", period, flux),
                    "energy" => format!("*Energy Balance* \\({}\\)\nFlux {:.1}/10 — body trend validates or challenges estimate.", period, flux),
                    "redox" => format!("*Redox Balance* \\({}\\)\nFlux {:.1}/10 — see full report redox-balance.", period, flux),
                    _ => format!("*{}* \\({}\\)\nFlux {:.1}/10. Use `krebslog report --help` for details.", r#type, period, flux),
                };
                let suggested = format!(
                    "~/reports/krebslog/{}-{}.svg",
                    r#type,
                    period.replace(' ', "_")
                );

                if json {
                    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                        "success": true, "type": r#type, "period": period, "post_ready": post_ready,
                        "caption_markdownv2": caption, "suggested_image": suggested, "krebs_status": ks
                    })).unwrap());
                } else if !quiet {
                    println!(
                        "{}",
                        caption
                            .replace("\\(", "(")
                            .replace("\\)", ")")
                            .replace("\\|", "|")
                    );
                    println!("Suggested image: {}", suggested);
                    if post_ready {
                        println!("(post this caption + the image with your Telegram bot)");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Cheap body note for telegram daily (latest weight near the date, via cache or one live call).
fn get_telegram_body_note(date: &str, ctx: &Context) -> Option<serde_json::Value> {
    // Reuse the light daily helper logic without pulling the whole report module into telegram if possible.
    // Simple inline version to keep telegram light.
    let cache_enabled = !ctx.no_cache;
    if cache_enabled {
        if let Ok(conn) = crate::db::open_db(ctx.db.as_deref()) {
            if let Ok(rows) = crate::db::get_body_measurements(&conn, date, Some(date)) {
                if let Some(m) = rows.first() {
                    return Some(serde_json::json!({
                        "weight_kg": m.get("weight_kg"),
                        "date": m.get("date")
                    }));
                }
            }
            if let Ok(Some(latest)) = crate::db::get_latest_body_measurement(&conn) {
                return Some(serde_json::json!({
                    "weight_kg": latest.get("weight_kg"),
                    "date": latest.get("date"),
                    "note": "latest (not exact date)"
                }));
            }
        }
    }
    if let Some(bin) = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]) {
        if let Ok((out, _)) = run_external_json(
            &bin,
            &[
                "measurement".into(),
                "list".into(),
                "--since".into(),
                date.into(),
            ],
        ) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                if let Some(arr) = v.as_array() {
                    if let Some(first) = arr.first() {
                        if cache_enabled {
                            if let Ok(conn) = crate::db::open_db(ctx.db.as_deref()) {
                                if let Some(a) = v.as_array() {
                                    let _ = crate::db::store_body_measurements(&conn, a);
                                }
                            }
                        }
                        return Some(serde_json::json!({
                            "weight_kg": first.get("weight_kg"),
                            "date": first.get("date")
                        }));
                    }
                }
            }
        }
    }
    None
}
