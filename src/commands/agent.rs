use crate::cli::{AgentAction, AgentSkillsAction};
use crate::commands::report::build_compact_krebs_status_for_agent;
use crate::context::Context;
use crate::db::{get_body_measurements, get_body_profile, open_db, store_body_profile};
use crate::error::Result;
use crate::utils::{resolve_bin, run_external_json};

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
            generate_prompts,
            focus,
        } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "command": "agent tune", "generate_prompts": generate_prompts, "focus": focus, "note": "skeleton" })
                );
            } else if !quiet {
                println!("agent tune (skeleton)");
            }
        }
        AgentAction::Skills {
            action: skills_action,
        } => match skills_action {
            AgentSkillsAction::Export { to, force } => {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": true, "command": "agent skills export", "to": to, "force": force, "note": "will copy AGENTS.md + skill fragments" })
                    );
                } else if !quiet {
                    println!("agent skills export (skeleton)");
                }
            }
        },
    }
    Ok(())
}
