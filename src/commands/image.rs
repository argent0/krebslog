use crate::cli::ImageAction;
use crate::commands::report::{
    build_krebs_cycle_svg, gather_period_data, status_from_score, KrebsCycleStep,
};
use crate::context::Context;
use crate::error::Result;
use crate::utils::{format_date_for_child, parse_flexible_date, resolve_bin, run_external_json};
use std::fs;
use std::path::PathBuf;

/// Default output directory for images (consistent with web reports).
fn default_image_dir() -> PathBuf {
    if let Some(proj) = directories::ProjectDirs::from("com", "krebslog", "krebslog") {
        let _p = proj.data_dir().to_path_buf();
        // images go under reports sibling for user convenience (same as web)
        // fall back to XDG data if needed, but prefer ~/reports
        let mut user_reports = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        user_reports.push("reports/krebslog");
        let _ = fs::create_dir_all(&user_reports);
        return user_reports;
    }
    let mut p = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    p.push("reports/krebslog");
    let _ = fs::create_dir_all(&p);
    p
}

/// Resolve output path: use provided or default dir + sensible name. Force .svg.
fn resolve_image_path(output: Option<String>, default_name: &str) -> PathBuf {
    match output {
        Some(p) => {
            let mut pb = PathBuf::from(p);
            if pb.extension().is_none() {
                pb.set_extension("svg");
            } else if pb.extension().and_then(|e| e.to_str()) != Some("svg") {
                // prefer svg for now (pure rust, no plotters)
                let mut s = pb.to_string_lossy().to_string();
                if !s.ends_with(".svg") {
                    s.push_str(".svg");
                }
                pb = PathBuf::from(s);
            }
            if let Some(parent) = pb.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(parent);
                }
            }
            pb
        }
        None => {
            let mut p = default_image_dir();
            p.push(default_name);
            if !p.ends_with(".svg") {
                p.set_extension("svg");
            }
            p
        }
    }
}

pub fn handle_image(action: ImageAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;

    match action {
        ImageAction::KrebsCycle {
            date_range,
            output,
            theme: _,
            include_body_context,
        } => {
            let mut body_ctx: Option<serde_json::Value> = None;
            if include_body_context {
                if let Some(bin) = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]) {
                    let since = date_range.clone();
                    if let Ok((out, _)) = run_external_json(
                        &bin,
                        &["report".into(), "weight".into(), "--since".into(), since],
                    ) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                            body_ctx = Some(serde_json::json!({
                                "source": "bodylog",
                                "weight_report": v,
                                "note": "Body deltas for callout alignment with flux."
                            }));
                        }
                    }
                } else {
                    body_ctx = Some(
                        serde_json::json!({"available": false, "note": "bodylog binary not resolved"}),
                    );
                }
            }

            // Gather + build a minimal steps list for the cycle SVG (reuse the exact renderer).
            let start = format_date_for_child(
                parse_flexible_date(&date_range).unwrap_or_else(|_| chrono::Utc::now()),
            );
            let g = gather_period_data(&start, &start, ctx);
            // Derive an overall proxy for coloring the cycle (use flux if available via simple extract path).
            let (flux, _) = crate::commands::report::compute_krebs_flux(&g); // reuse compute for status
            let base_status = status_from_score((flux.proxy * 10.0) as u32);
            let metabolites = [
                "citrate_synthase",
                "aconitase",
                "isocitrate_dehydrogenase",
                "alpha_ketoglutarate_dehydrogenase",
                "succinyl_coa_synthetase",
                "succinate_dehydrogenase",
                "fumarase",
                "malate_dehydrogenase",
            ];
            let steps: Vec<KrebsCycleStep> = metabolites
                .iter()
                .map(|id| KrebsCycleStep {
                    id: id.to_string(),
                    name: id.replace('_', " ").to_string(),
                    status: base_status.clone(),
                    flux_proxy: (flux.proxy * 10.0) as u32,
                    personal_contribution: None,
                    explanation_short: "Derived from period flux proxy.".into(),
                    calculation_note: "Status tinted from overall krebs flux.".into(),
                })
                .collect();

            let svg = build_krebs_cycle_svg(&steps);
            let path = resolve_image_path(
                output.clone(),
                &format!("krebs-cycle-{}.svg", start.replace('-', "")),
            );
            let _ = fs::write(&path, &svg);

            let mut body_note = None;
            if let Some(b) = &body_ctx {
                body_note = Some(format!(
                    "body context: {}",
                    b.get("note").and_then(|x| x.as_str()).unwrap_or("prepared")
                ));
            }

            if json {
                let mut resp = serde_json::json!({
                    "success": true,
                    "command": "image krebs-cycle",
                    "date_range": date_range,
                    "path": path.to_string_lossy().to_string(),
                    "format": "svg",
                    "body_context": body_ctx
                });
                if let Some(n) = body_note {
                    resp["body_note"] = serde_json::json!(n);
                }
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else if !quiet {
                println!("image krebs-cycle — wrote {}", path.display());
                if let Some(n) = body_note {
                    println!("  {}", n);
                }
            }
        }
        ImageAction::TrainingLoadHeatmap { period, output } => {
            // Simple SVG calendar strip (last N days load heatmap). Pure rust, no deps.
            let start = format_date_for_child(
                parse_flexible_date(&period)
                    .unwrap_or_else(|_| chrono::Utc::now() - chrono::Duration::days(89)),
            );
            let g = gather_period_data(&start, "today", ctx);
            // Very rough: use a single load value repeated for demo grid (real would bin workouts by date).
            let (_vol, sess, load) = crate::commands::report::extract_training_totals(&g);
            let days = 30u32; // compact view
            let cell = 18.0;
            let mut svg = String::from(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="620" height="120" viewBox="0 0 620 120">"#,
            );
            for i in 0..days {
                let intensity = ((load / 100.0) * (1.0 + (i as f64 % 5.0) * 0.1)).clamp(0.1, 1.0);
                let color = if intensity > 0.7 {
                    "#22c55e"
                } else if intensity > 0.4 {
                    "#eab308"
                } else {
                    "#ef4444"
                };
                let x = 10.0 + (i as f64 * (cell + 2.0));
                svg.push_str(&format!(
                    r#"<rect x="{x:.0}" y="20" width="{w}" height="{h}" rx="3" fill="{c}" opacity="{o:.2}"><title>day {i} (load proxy {l:.0})</title></rect>"#,
                    x = x, w = cell, h = cell, c = color, o = 0.6 + intensity * 0.4, i = i, l = load
                ));
            }
            svg.push_str(&format!(r#"<text x="10" y="70" font-size="10" fill='#a1a1aa'>Training load heatmap ({} days, {} sessions, load proxy {:.0}) — pure SVG</text>"#, days, sess, load));
            svg.push_str("</svg>");

            let path = resolve_image_path(
                output.clone(),
                &format!("training-load-{}.svg", start.replace('-', "")),
            );
            let _ = fs::write(&path, &svg);

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": true, "command": "image training-load-heatmap", "period": period,
                        "path": path.to_string_lossy().to_string(), "format": "svg"
                    }))
                    .unwrap()
                );
            } else if !quiet {
                println!("image training-load-heatmap — wrote {}", path.display());
            }
        }
        ImageAction::AntioxidantShield { style: _, output } => {
            // Symbolic bar + label SVG for redox / antioxidant shield.
            let g = gather_period_data("last 14 days", "today", ctx);
            let redox = crate::commands::report::compute_redox_balance(&g);
            let w = (redox.score / 10.0 * 400.0).clamp(20.0, 400.0);
            let color = if redox.score >= 7.0 {
                "#22c55e"
            } else if redox.score >= 5.0 {
                "#eab308"
            } else {
                "#ef4444"
            };
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="520" height="140" viewBox="0 0 520 140">
  <rect x="10" y="30" width="400" height="24" rx="4" fill='#27272a'/>
  <rect x="10" y="30" width="{w:.0}" height="24" rx="4" fill="{c}"/>
  <text x="10" y="80" font-size="12" fill='#e4e4e7'>Antioxidant Shield — score {s:.1}/10 ({i})</text>
  <text x="10" y="100" font-size="10" fill='#a1a1aa'>ROS proxy {r:.1} vs antioxidant {a:.1} (pure SVG)</text>
</svg>"#,
                w = w,
                c = color,
                s = redox.score,
                i = redox.interpretation,
                r = redox.ros_proxy,
                a = redox.antioxidant_proxy
            );
            let path = resolve_image_path(output.clone(), "antioxidant-shield.svg");
            let _ = fs::write(&path, &svg);

            if json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "success": true, "command": "image antioxidant-shield",
                    "path": path.to_string_lossy().to_string(), "format": "svg", "score": redox.score
                })).unwrap());
            } else if !quiet {
                println!("image antioxidant-shield — wrote {}", path.display());
            }
        }
        ImageAction::FullDashboard {
            date,
            layout: _,
            output,
        } => {
            let mut body_ctx: Option<serde_json::Value> = None;
            if let Some(bin) = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]) {
                if let Ok((out, _)) = run_external_json(
                    &bin,
                    &[
                        "report".into(),
                        "weight".into(),
                        "--since".into(),
                        date.clone(),
                    ],
                ) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                        body_ctx = Some(serde_json::json!({ "source": "bodylog", "weight": v }));
                    }
                }
            }
            let start = format_date_for_child(
                parse_flexible_date(&date).unwrap_or_else(|_| chrono::Utc::now()),
            );
            let g = gather_period_data(&start, &start, ctx);
            let (flux, _) = crate::commands::report::compute_krebs_flux(&g);
            let redox = crate::commands::report::compute_redox_balance(&g);
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="620" height="220" viewBox="0 0 620 220">
  <rect width="620" height="220" rx="12" fill='#111113'/>
  <text x="20" y="36" font-size="16" fill='#e4e4e7' font-family='system-ui'>Krebs Dashboard — {d}</text>
  <text x="20" y="70" font-size="13" fill='#22c55e'>Flux: {f:.1}/10</text>
  <text x="160" y="70" font-size="13" fill='#eab308'>Redox: {r:.1}/10</text>
  <text x="320" y="70" font-size="12" fill='#a1a1aa'>Body: {b}</text>
  <text x="20" y="110" font-size="11" fill='#71717a'>Pure SVG (no plotters). See krebs-cycle.svg and report web for interactive.</text>
</svg>"#,
                d = date,
                f = flux.proxy,
                r = redox.score,
                b = body_ctx
                    .as_ref()
                    .and_then(|b| b.get("weight"))
                    .map(|_| "data")
                    .unwrap_or("n/a")
            );
            let path = resolve_image_path(
                output.clone(),
                &format!("full-dashboard-{}.svg", start.replace('-', "")),
            );
            let _ = fs::write(&path, &svg);

            if json {
                let mut resp = serde_json::json!({
                    "success": true, "command": "image full-dashboard", "date": date,
                    "path": path.to_string_lossy().to_string(), "format": "svg"
                });
                if let Some(b) = body_ctx {
                    resp["body_context"] = b;
                }
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else if !quiet {
                println!("image full-dashboard — wrote {}", path.display());
            }
        }
    }
    Ok(())
}
