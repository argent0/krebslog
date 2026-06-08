use crate::cli::{AgentAction, AgentSkillsAction};
use crate::commands::report::build_compact_krebs_status_for_agent;
use crate::context::Context;
use crate::db::{get_body_measurements, get_body_profile, open_db, store_body_profile};
use crate::error::Result;
use crate::utils::{resolve_bin, run_external_json};
use std::fs;
use std::path::{Path, PathBuf};

pub fn handle_agent(action: AgentAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    match action {
        AgentAction::Context {
            r#for,
            since,
            output,
        } => {
            let bodylog_bin = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]);
            let body_available = bodylog_bin.is_some();
            let cache_enabled = !ctx.no_cache;

            // Rich body section per spec/03 (Phase 4): latest + trends for the window + profile.
            let mut body_obj: Option<serde_json::Value> = None;

            if body_available {
                let mut latest: Option<serde_json::Value> = None;
                let mut profile: Option<serde_json::Value> = None;
                let mut trends: Option<serde_json::Value> = None;

                // 1. Profile (config) — prefer cache, else live + store.
                if cache_enabled {
                    if let Ok(conn) = open_db(ctx.db.as_deref()) {
                        profile = get_body_profile(&conn).ok().flatten();
                    }
                }
                if profile.is_none() {
                    if let Some(bin) = &bodylog_bin {
                        if let Ok((out, _)) =
                            run_external_json(bin, &["config".into(), "show".into()])
                        {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                                profile = Some(v.clone());
                                if cache_enabled {
                                    if let Ok(conn) = open_db(ctx.db.as_deref()) {
                                        let _ = store_body_profile(&conn, &v);
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Latest + window measurements for trends (cache first, then live list for the `since`).
                let mut measurements: Vec<serde_json::Value> = vec![];
                if cache_enabled {
                    if let Ok(conn) = open_db(ctx.db.as_deref()) {
                        if let Ok(rows) = get_body_measurements(&conn, &since, None) {
                            measurements = rows;
                        }
                    }
                }
                if measurements.is_empty() {
                    if let Some(bin) = &bodylog_bin {
                        if let Ok((out, _)) = run_external_json(
                            bin,
                            &[
                                "measurement".into(),
                                "list".into(),
                                "--since".into(),
                                since.clone(),
                            ],
                        ) {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                                if let Some(arr) = v.as_array() {
                                    measurements = arr.clone();
                                    if cache_enabled {
                                        if let Ok(conn) = open_db(ctx.db.as_deref()) {
                                            let _ = crate::db::store_body_measurements(&conn, arr);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(first) = measurements.first().cloned() {
                    latest = Some(first);
                }

                // Compute simple trends from the measurements we have for the requested since (or last 2+).
                if measurements.len() >= 2 {
                    // measurements from cache/live list are newest-first; sort ascending for delta.
                    let mut sorted = measurements.clone();
                    sorted.sort_by(|a, b| {
                        let da = a.get("date").and_then(|d| d.as_str()).unwrap_or("");
                        let db = b.get("date").and_then(|d| d.as_str()).unwrap_or("");
                        da.cmp(db)
                    });
                    let old = &sorted[0];
                    let new = &sorted[sorted.len() - 1];
                    let w_old = old.get("weight_kg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let w_new = new.get("weight_kg").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let m_old = old
                        .get("skeletal_muscle_pct")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let m_new = new
                        .get("skeletal_muscle_pct")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    trends = Some(serde_json::json!({
                        "weight_delta_kg": ((w_new - w_old) * 100.0).round() / 100.0,
                        "weight_trend": if w_new > w_old + 0.15 { "up" } else if w_new < w_old - 0.15 { "down" } else { "flat" },
                        "muscle_delta_pct": ((m_new - m_old) * 100.0).round() / 100.0,
                        "period_points": sorted.len(),
                        "start_date": old.get("date"),
                        "end_date": new.get("date")
                    }));
                }

                let mut b = serde_json::json!({ "available": true });
                if let Some(l) = latest {
                    b["latest"] = l;
                }
                if let Some(t) = trends {
                    b["trends"] = t; // covers the "trends_14d" spirit for the requested window
                }
                if let Some(p) = profile {
                    b["profile"] = p;
                }
                body_obj = Some(b);
            }

            // Build a compact krebs_status using the shared (body-validated) pipeline.
            let krebs_status = build_compact_krebs_status_for_agent(&since, ctx);

            if json {
                let mut payload = serde_json::json!({
                    "success": true,
                    "command": "agent context",
                    "for": r#for,
                    "since": since,
                    "output": output,
                    "bodylog_available": body_available,
                    "krebs_status": krebs_status,
                });
                if let Some(b) = body_obj {
                    payload["body"] = b;
                } else if body_available {
                    payload["body"] =
                        serde_json::json!({ "available": true, "note": "no body data for window" });
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&payload).expect("in-memory")
                );
            } else if !quiet {
                println!(
                    "agent context --for {} --since {} (bodylog_available={}, krebs_status included)",
                    r#for, since, body_available
                );
            }
        }
        AgentAction::Tune {
            generate_prompts: _,
            focus,
        } => {
            let f = focus
                .as_deref()
                .unwrap_or("general metabolic interpretation");
            let prompt = format!(
                "You are an expert biohacker agent. Given a krebs_status JSON (flux 0-10 with components, redox, energy with body_validation, body_adaptation, insights, assumptions):\n\n\
                 1. Explain in 1-2 sentences whether the Krebs cycle appears efficient given the body outcome.\n\
                 2. Flag the single highest-leverage adjustment (nutrition timing, antioxidant tags, deload, etc.).\n\
                 3. Quote the exact flux formula and body_validation interpretation from the data.\n\n\
                 Focus: {}.\n\nExample input (abbrev): {{\"krebs_flux\":{{\"proxy\":7.8,\"formula\":\"0.35*carb...\"}},\"body_validation\":{{\"interpretation\":\"...\"}}}}",
                f
            );
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "command": "agent tune",
                        "focus": focus,
                        "prompt_fragment": prompt
                    }))
                    .unwrap()
                );
            } else if !quiet {
                println!("agent tune (focus: {})", f);
                println!("{}", prompt);
            }
        }
        AgentAction::Skills {
            action: skills_action,
        } => match skills_action {
            AgentSkillsAction::Export { to, force } => {
                let target = to.clone().unwrap_or_else(|| "./krebs-skills".to_string());
                let target_dir = PathBuf::from(&target);
                let _ = fs::create_dir_all(&target_dir);
                let mut written = vec![];
                let agents_src = PathBuf::from("AGENTS.md");
                let dst = target_dir.join("AGENTS.md");
                if agents_src.exists()
                    && (force || !dst.exists())
                    && fs::copy(&agents_src, &dst).is_ok()
                {
                    written.push(dst.to_string_lossy().to_string());
                }
                // Also copy a couple of high-value docs for the agent
                for doc in &["docs/agent-usage.md", "docs/reporting.md"] {
                    let src = PathBuf::from(doc);
                    if src.exists() {
                        let name = Path::new(doc).file_name().unwrap_or_default();
                        let d = target_dir.join(name);
                        if (force || !d.exists()) && fs::copy(&src, &d).is_ok() {
                            written.push(d.to_string_lossy().to_string());
                        }
                    }
                }
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "success": true,
                            "command": "agent skills export",
                            "to": target,
                            "written": written,
                            "force": force
                        }))
                        .unwrap()
                    );
                } else if !quiet {
                    println!("agent skills export to {}", target);
                    for w in &written {
                        println!("  copied {}", w);
                    }
                    if written.is_empty() {
                        println!("  (nothing new written; use --force to overwrite)");
                    }
                }
            }
        },
    }
    Ok(())
}
