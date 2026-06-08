use crate::cli::ImageAction;
use crate::context::Context;
use crate::error::Result;
use crate::utils::{resolve_bin, run_external_json};

pub fn handle_image(action: ImageAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;

    match action {
        ImageAction::KrebsCycle {
            date_range,
            output,
            theme,
            include_body_context,
        } => {
            let mut body_ctx: Option<serde_json::Value> = None;
            if include_body_context {
                // Best-effort live pull so the (future) real renderer has deltas + verdict ready.
                if let Some(bin) = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]) {
                    // Use the date_range as "since" for body trends.
                    let since = date_range.clone();
                    if let Ok((out, _)) = run_external_json(
                        &bin,
                        &["report".into(), "weight".into(), "--since".into(), since],
                    ) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                            body_ctx = Some(serde_json::json!({
                                "source": "bodylog",
                                "weight_report": v,
                                "note": "Supply to renderer for bottom callout strip: deltas + qualitative alignment with krebs flux."
                            }));
                        }
                    }
                } else {
                    body_ctx = Some(
                        serde_json::json!({"available": false, "note": "bodylog binary not resolved"}),
                    );
                }
            }

            let detail = format!(
                "range={}, theme={}, out={:?}, body_context={}",
                date_range, theme, output, include_body_context
            );

            if json {
                let mut resp = serde_json::json!({
                    "success": true,
                    "command": "image krebs-cycle",
                    "detail": detail,
                    "note": "Image generation (plotters + custom SVG) is behind the images feature flag / future work. Body context data is prepared for the renderer when --include-body-context is used."
                });
                if let Some(b) = body_ctx {
                    resp["body_context"] = b;
                }
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else if !quiet {
                println!("image krebs-cycle — {}", detail);
                if body_ctx.is_some() {
                    println!("  body_context prepared for renderer");
                }
                println!("(see spec/04 Phase 3 for body callout expectations)");
            }
        }
        ImageAction::TrainingLoadHeatmap { period, output } => {
            let detail = format!("period={}, out={:?}", period, output);
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "image training-load-heatmap", "detail": detail, "note": "not implemented (skeleton)" })
                );
            } else if !quiet {
                println!("image training-load-heatmap — {}", detail);
            }
        }
        ImageAction::AntioxidantShield { style, output } => {
            let detail = format!("style={}, out={:?}", style, output);
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "image antioxidant-shield", "detail": detail, "note": "not implemented (skeleton)" })
                );
            } else if !quiet {
                println!("image antioxidant-shield — {}", detail);
            }
        }
        ImageAction::FullDashboard {
            date,
            layout,
            output,
        } => {
            let detail = format!("date={}, layout={}, out={:?}", date, layout, output);
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "image full-dashboard", "detail": detail, "note": "not implemented (skeleton); body context available via krebs-cycle --include-body-context" })
                );
            } else if !quiet {
                println!("image full-dashboard — {}", detail);
            }
        }
    }
    Ok(())
}
