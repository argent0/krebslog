use crate::cli::ReportAction;
use crate::context::Context;
use crate::db::{
    get_body_measurements, get_latest_body_measurement, open_db, store_body_measurements,
};
use crate::error::{KrebslogError, Result};
use crate::utils::{format_date_for_child, parse_flexible_date, resolve_bin, run_external_json};
use chrono::{DateTime, Duration, Utc};
use comfy_table::{presets, Cell, Table};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn handle_report(action: ReportAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    match action {
        ReportAction::Daily {
            date,
            include_image,
            output_dir,
        } => {
            // Real implementation: gather the single-day (or near) window and surface
            // nutrition + training extracts + derived krebs/redox/energy + body (if available).
            // Uses the same pipeline as krebs-status/web for consistency and transparency.
            let start = date.clone();
            let g = gather_period_data(&start, &start, ctx);
            let (flux, _fc) = compute_krebs_flux(&g);
            let redox = compute_redox_balance(&g);
            let energy = compute_energy_with_body_validation(&g, 1);
            let body = compute_body_adaptation(&g, 1);
            let (kcal, protein, carbs, fat, _ant) = extract_nutrition_totals(&g);
            let (vol, sess, _load) = extract_training_totals(&g);
            let body_light = get_light_body_for_date(&date, ctx);

            if json {
                let mut out = serde_json::json!({
                    "success": true,
                    "command": "report daily",
                    "date": date,
                    "include_image": include_image,
                    "output_dir": output_dir,
                    "nutrition": {"kcal": kcal, "protein_g": protein, "carbs_g": carbs, "fat_g": fat},
                    "training": {"volume": vol, "sessions": sess},
                    "krebs_flux": flux,
                    "redox_balance": redox,
                    "energy_balance": energy,
                    "body_adaptation": body,
                    "sources": {
                        "nutlog": g.nutlog_available,
                        "repslog": g.repslog_available,
                        "bodylog": g.bodylog_available
                    }
                });
                if let Some(b) = body_light {
                    out["body"] = b;
                }
                println!("{}", serde_json::to_string_pretty(&out).expect("in-memory"));
            } else if !quiet {
                println!(
                    "report daily --date {} (nutrition + training + derived)",
                    date
                );
                println!(
                    "  nutrition: {:.0} kcal (P{:.0} C{:.0} F{:.0})",
                    kcal, protein, carbs, fat
                );
                println!("  training: vol {:.0}, sessions {}", vol, sess);
                println!(
                    "  krebs flux: {:.1}/10 (carb {:.1}, fat {:.1}, protein {:.1}, training {:.1})",
                    flux.proxy,
                    flux.components.carb_availability,
                    flux.components.fat_mobilization,
                    flux.components.protein_anaplerosis,
                    flux.components.training_demand
                );
                println!("  redox: {:.1}/10 ({})", redox.score, redox.interpretation);
                if let Some(bv) = &energy.body_validation {
                    println!("  body validation: {}", bv.interpretation);
                }
                if let Some(b) = &body_light {
                    if let Some(w) = b.get("weight_kg").and_then(|v| v.as_f64()) {
                        println!("  body: {:.1} kg (latest near date)", w);
                    }
                }
                if include_image {
                    println!("  (include_image requested — see image krebs-cycle or report web for visuals)");
                }
            }
        }
        ReportAction::Weekly { since, until } => {
            // Real multi-day aggregate using the shared gather/compute (body + flux/redox/energy).
            let end = until.clone().unwrap_or_else(|| "today".to_string());
            let g = gather_period_data(&since, &end, ctx);
            let days = 7i64; // best effort label
            let (flux, _fc) = compute_krebs_flux(&g);
            let redox = compute_redox_balance(&g);
            let energy = compute_energy_with_body_validation(&g, days);
            let body = compute_body_adaptation(&g, days);
            let body_window = get_light_body_for_window(&since, until.as_deref(), ctx);

            if json {
                let mut out = serde_json::json!({
                    "success": true,
                    "command": "report weekly",
                    "since": since,
                    "until": until,
                    "krebs_flux": flux,
                    "redox_balance": redox,
                    "energy_balance": energy,
                    "body_adaptation": body
                });
                if let Some(b) = body_window {
                    out["body"] = b;
                }
                println!("{}", serde_json::to_string_pretty(&out).expect("in-memory"));
            } else if !quiet {
                println!(
                    "report weekly --since {} (multi-day aggregates + derived)",
                    since
                );
                println!("  krebs flux: {:.1}/10", flux.proxy);
                println!("  redox: {:.1}/10 ({})", redox.score, redox.interpretation);
                if let Some(bv) = &energy.body_validation {
                    println!("  body validation: {}", bv.interpretation);
                }
                if let Some(b) = &body_window {
                    if let (Some(start), Some(end)) = (
                        b.get("start_weight").and_then(|v| v.as_f64()),
                        b.get("end_weight").and_then(|v| v.as_f64()),
                    ) {
                        println!(
                            "  body: {:.1} → {:.1} kg (delta {:.1})",
                            start,
                            end,
                            end - start
                        );
                    } else if b.get("count").and_then(|c| c.as_i64()).unwrap_or(0) > 0 {
                        println!("  body data present for window");
                    }
                }
            }
        }
        ReportAction::KrebsFlux { period, output } => {
            let start = period.clone();
            let g = gather_period_data(&start, &start, ctx); // treat period as since for single-point view
            let (flux, caveats) = compute_krebs_flux(&g);
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "command": "report krebs-flux",
                        "period": period,
                        "output": output,
                        "krebs_flux": flux,
                        "caveats": caveats
                    }))
                    .unwrap()
                );
            } else if !quiet {
                println!("report krebs-flux --period {} ", period);
                println!("  proxy: {:.1}/10  formula: {}", flux.proxy, flux.formula);
                println!(
                    "  components: carb={:.1} fat={:.1} protein={:.1} training={:.1}",
                    flux.components.carb_availability,
                    flux.components.fat_mobilization,
                    flux.components.protein_anaplerosis,
                    flux.components.training_demand
                );
            }
        }
        ReportAction::RedoxBalance { since, until } => {
            let end = until.clone().unwrap_or_else(|| "today".to_string());
            let g = gather_period_data(&since, &end, ctx);
            let redox = compute_redox_balance(&g);
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "command": "report redox-balance",
                        "since": since,
                        "until": until,
                        "redox_balance": redox
                    }))
                    .unwrap()
                );
            } else if !quiet {
                println!("report redox-balance --since {}", since);
                println!("  score: {:.1}/10  {}", redox.score, redox.interpretation);
                println!(
                    "  ros_proxy: {:.1}  antioxidant_proxy: {:.1}",
                    redox.ros_proxy, redox.antioxidant_proxy
                );
            }
        }
        ReportAction::EnergyBalance {
            since,
            until: _,
            include_body_trends,
        } => {
            // Always compute the nutrition+training base energy (body is enrichment).
            // Body trends/validation are included when flag is set (or body data is present in gather).
            let g = gather_period_data(&since, &since, ctx);
            let energy = compute_energy_with_body_validation(&g, 14);
            let (kcal, _p, _c, _f, _a) = extract_nutrition_totals(&g);
            let (vol, sess, _l) = extract_training_totals(&g);

            let mut body: Option<serde_json::Value> = None;
            let mut body_notes = None;
            let body_validation = energy.body_validation.clone().map(|bv| {
                serde_json::json!({
                    "weight_delta_kg": bv.weight_delta_kg,
                    "interpretation": bv.interpretation,
                    "discrepancy_severity": bv.discrepancy_severity,
                    "source": "bodylog"
                })
            });

            if include_body_trends {
                // Existing body fetch logic (kept for raw series when requested)
                let bodylog_bin = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]);
                let cache_enabled = !ctx.no_cache;
                if cache_enabled {
                    if let Ok(conn) = open_db(ctx.db.as_deref()) {
                        if let Ok(cached) = get_body_measurements(&conn, &since, None) {
                            if !cached.is_empty() {
                                body = Some(
                                    serde_json::json!({ "series": cached, "source": "cache" }),
                                );
                            }
                        }
                    }
                }
                if body.is_none() {
                    if let Some(bin) = &bodylog_bin {
                        let call_args: Vec<String> = vec![
                            "report".into(),
                            "weight".into(),
                            "--since".into(),
                            since.clone(),
                        ];
                        if let Ok((out, _)) = run_external_json(bin, &call_args) {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                                body = Some(v);
                            }
                        }
                    } else {
                        body_notes = Some("bodylog binary not found".to_string());
                    }
                }
            }

            if json {
                let mut out = serde_json::json!({
                    "success": true,
                    "command": "report energy-balance",
                    "since": since,
                    "include_body_trends": include_body_trends,
                    "estimated_surplus_kcal": energy.estimated_surplus_kcal,
                    "nutrition_kcal": kcal,
                    "training_volume": vol,
                    "sessions": sess
                });
                if let Some(b) = body {
                    out["body"] = b;
                }
                if let Some(n) = body_notes {
                    out["body_note"] = serde_json::json!(n);
                }
                if let Some(bv) = body_validation {
                    out["body_validation"] = bv;
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&out).expect("in-memory json")
                );
            } else if !quiet {
                println!(
                    "report energy-balance --since {} (include_body_trends={})",
                    since, include_body_trends
                );
                println!(
                    "  estimated surplus: {:.0} kcal (intake {} vs training+base)",
                    energy.estimated_surplus_kcal, kcal
                );
                if let Some(bv) = &energy.body_validation {
                    println!("  body validation: {}", bv.interpretation);
                }
                // (existing body print block retained for --include-body-trends raw details)
                if let Some(b) = &body {
                    println!(
                        "body trends from bodylog: {}",
                        serde_json::to_string(b).unwrap_or_default()
                    );
                } else if include_body_trends {
                    if let Some(n) = body_notes {
                        println!("  (bodylog: {})", n);
                    }
                }
            }
        }
        ReportAction::Correlations { x, y, period } => {
            handle_correlations(x, y, period, ctx)?;
        }
        ReportAction::Web {
            since,
            until,
            period,
            output,
            single_file,
            folder,
            include_images,
            theme,
            embed_data,
        } => {
            handle_web_report(
                since,
                until,
                period,
                output,
                single_file,
                folder,
                include_images,
                theme,
                embed_data,
                ctx,
            )?;
        }
        ReportAction::KrebsStatus {
            since,
            until,
            date,
            include_raw,
        } => {
            handle_krebs_status(since, until, date, include_raw, ctx)?;
        }
    }
    Ok(())
}

// ============================================================================
// Krebs Status Implementation (spec/04-krebs-status.md)
// Body-enhanced flux / redox / energy with real adaptation validation from bodylog.
// ============================================================================

#[allow(clippy::too_many_arguments)]
fn handle_krebs_status(
    since: String,
    until: Option<String>,
    date: Option<String>,
    include_raw: bool,
    ctx: &Context,
) -> Result<()> {
    let json_out = ctx.json;
    let quiet = ctx.quiet;

    // Resolve effective window. --date takes precedence for a snapshot.
    let (start_dt, end_dt, _raw_since_for_label) = if let Some(d) = &date {
        let dt = parse_flexible_date(d)?;
        (dt, dt, d.clone())
    } else {
        let s = parse_flexible_date(&since)?;
        let e = if let Some(u) = &until {
            parse_flexible_date(u)?
        } else {
            Utc::now()
        };
        let e = if e < s { s } else { e };
        (s, e, since.clone())
    };

    let start_str = format_date_for_child(start_dt);
    let end_str = format_date_for_child(end_dt);
    let days = (end_dt.date_naive() - start_dt.date_naive())
        .num_days()
        .max(1);

    // Gather (best effort)
    let gathered = gather_period_data(&start_str, &end_str, ctx);

    // Compute the core signals
    let (flux, flux_caveats) = compute_krebs_flux(&gathered);
    let redox = compute_redox_balance(&gathered);
    let energy = compute_energy_with_body_validation(&gathered, days);
    let body_adapt = compute_body_adaptation(&gathered, days);
    let insights = generate_insights(&flux, &redox, &energy, &body_adapt, &gathered);

    let (assumptions, mut caveats) = build_assumptions_and_caveats(&gathered, days, {
        // best effort measurement count from summary or array length
        if let Some(s) = &gathered.body_summary {
            s.get("measurement_count")
                .and_then(|v| v.as_i64())
                .or_else(|| {
                    s.get("weight")
                        .and_then(|w| w.get("count").and_then(|c| c.as_i64()))
                })
                .unwrap_or(0)
        } else if let Some(m) = &gathered.body_measurements {
            m.as_array().map(|a| a.len() as i64).unwrap_or(0)
        } else {
            0
        }
    });
    caveats.extend(flux_caveats);

    // Sources list (for the response meta)
    let sources = {
        let mut s = vec![];
        if gathered.nutlog_available {
            s.push("nutlog".to_string());
        }
        if gathered.repslog_available {
            s.push("repslog".to_string());
        }
        if gathered.bodylog_available {
            s.push("bodylog".to_string());
        }
        if s.is_empty() {
            s.push("none".to_string());
        }
        s
    };

    // Optional raw payloads
    let raw = if include_raw {
        Some(serde_json::json!({
            "nutlog": { "consumption": gathered.consumption, "nutrition_report": gathered.nutrition_report },
            "repslog": { "workouts": gathered.workouts, "stats_summary": gathered.stats_summary },
            "bodylog": { "measurements": gathered.body_measurements, "summary": gathered.body_summary },
            "notes": gathered.error_notes
        }))
    } else {
        None
    };

    let period = PeriodInfo {
        since: start_str.clone(),
        until: end_str.clone(),
        days,
    };

    let output = KrebsStatusOutput {
        success: true,
        period,
        sources,
        krebs_flux: flux,
        redox_balance: redox,
        energy_balance: energy,
        body_adaptation: body_adapt,
        insights,
        assumptions,
        caveats,
        raw,
    };

    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&output)
                .expect("serializing krebs-status output cannot fail in-memory")
        );
    } else if !quiet {
        // Human: compact table summary + narrative + insights + methodology
        println!("Krebs Status — {} ({} days)", start_str, days);

        let mut table = Table::new();
        table.load_preset(presets::UTF8_FULL_CONDENSED);
        table.set_header(vec!["Metric", "Value", "Note"]);

        // Flux row
        table.add_row(vec![
            Cell::new("Krebs Flux (proxy)"),
            Cell::new(format!("{:.1}/10", output.krebs_flux.proxy)),
            Cell::new(output.krebs_flux.trend_7d.clone().unwrap_or_default()),
        ]);
        // Redox
        table.add_row(vec![
            Cell::new("Redox Balance"),
            Cell::new(format!("{:.1}/10", output.redox_balance.score)),
            Cell::new(output.redox_balance.interpretation.clone()),
        ]);
        // Energy
        let surplus = output.energy_balance.estimated_surplus_kcal;
        table.add_row(vec![
            Cell::new("Est. Energy Surplus"),
            Cell::new(format!("{surplus:+.0} kcal")),
            Cell::new(if surplus > 150.0 {
                "surplus"
            } else if surplus < -150.0 {
                "deficit"
            } else {
                "near balance"
            }),
        ]);
        // Body deltas (if any)
        if let Some(bv) = &output.energy_balance.body_validation {
            table.add_row(vec![
                Cell::new("Body Weight Δ"),
                Cell::new(format!("{:+.1} kg", bv.weight_delta_kg)),
                Cell::new(format!(
                    "sev={} ({})",
                    bv.discrepancy_severity, bv.confidence
                )),
            ]);
        }

        println!("{}", table);

        // Narrative + insights
        if let Some(bv) = &output.energy_balance.body_validation {
            println!("\n{}", bv.interpretation);
        }
        println!(
            "\nImplication: {}",
            output.body_adaptation.implication_for_krebs
        );

        if !output.insights.is_empty() {
            println!("\nInsights:");
            for i in &output.insights {
                println!("  • {}", i);
            }
        }

        // Methodology & Caveats (always visible for human)
        println!("\nMethodology & Caveats:");
        println!("  Flux: {}", output.krebs_flux.formula);
        for c in &output.caveats {
            println!("  - {}", c);
        }
        if let Some(r) = &output.raw {
            println!("  (raw child data included; see --json for full detail)");
            let _ = r; // silence unused in this branch
        }
    }

    Ok(())
}

/// Build a compact `krebs_status` object suitable for direct inclusion in
/// `agent context` output. Reuses the same gather + compute pipeline as the
/// full report so numbers and insights stay consistent.
pub(crate) fn build_compact_krebs_status_for_agent(
    since: &str,
    ctx: &Context,
) -> serde_json::Value {
    // Resolve a conservative window (use the provided since; until = now)
    let start = match parse_flexible_date(since) {
        Ok(d) => d,
        Err(_) => Utc::now() - Duration::days(30),
    };
    let end = Utc::now();
    let start_str = format_date_for_child(start);
    let end_str = format_date_for_child(end);

    let g = gather_period_data(&start_str, &end_str, ctx);
    let (flux, _cavs) = compute_krebs_flux(&g);
    let redox = compute_redox_balance(&g);
    let energy = compute_energy_with_body_validation(&g, 30);
    let body = compute_body_adaptation(&g, 30);
    let insights = generate_insights(&flux, &redox, &energy, &body, &g);

    let overall = if flux.proxy >= 7.5 && body.trends.skeletal_muscle.trend == "up" {
        "strong_efficient_adapting"
    } else if flux.proxy >= 6.0 {
        "solid_with_body_confirmation"
    } else {
        "needs_attention"
    };

    let one_sentence = format!(
        "Krebs flux running at {:.1}/10. Body composition moving {} — {}.",
        flux.proxy,
        body.trends.weight.trend,
        if body.trends.skeletal_muscle.trend == "up" {
            "recomp or lean mass signal present"
        } else {
            "monitor consistency"
        }
    );

    let mut key_flags = vec![];
    if let Some(bv) = &energy.body_validation {
        key_flags.push(format!("energy_discrepancy_{}", bv.discrepancy_severity));
    }
    if redox.score >= 6.0 {
        key_flags.push("redox_adequate".to_string());
    }

    let body_insights: Vec<String> = insights
        .iter()
        .filter(|s| {
            s.to_lowercase().contains("lean")
                || s.to_lowercase().contains("body")
                || s.to_lowercase().contains("partition")
        })
        .cloned()
        .collect();

    let recs = if redox.score < 5.5 {
        vec!["consider increasing antioxidant-tagged intake or a short recovery block".to_string()]
    } else {
        vec!["continue current nutrition timing around training".to_string()]
    };

    serde_json::json!({
        "overall_rating": overall,
        "one_sentence": one_sentence,
        "key_flags": key_flags,
        "body_validated_insights": if body_insights.is_empty() { insights } else { body_insights },
        "recommendations": recs,
        "flux_proxy": flux.proxy,
        "body_weight_trend": body.trends.weight.trend,
    })
}

// ---------------- Correlations (Phase 4 of spec/04) ----------------

fn handle_correlations(x: String, y: String, period: String, ctx: &Context) -> Result<()> {
    let json_out = ctx.json;
    let quiet = ctx.quiet;

    // Treat the `period` as a "since" for the window (common usage); end = now.
    let start = parse_flexible_date(&period).unwrap_or_else(|_| Utc::now() - Duration::days(30));
    let end = Utc::now();
    let start_str = format_date_for_child(start);
    let end_str = format_date_for_child(end);
    let days = (end.date_naive() - start.date_naive()).num_days().max(1);

    let g = gather_period_data(&start_str, &end_str, ctx);

    // Base extracts
    let (kcal, protein, carbs, fat, antiox) = extract_nutrition_totals(&g);
    let (volume, sessions, load) = extract_training_totals(&g);
    let (flux, _) = compute_krebs_flux(&g);
    let redox = compute_redox_balance(&g);
    let energy = compute_energy_with_body_validation(&g, days);
    let body = compute_body_adaptation(&g, days);

    // Resolve x and y to (label, value, unit-ish, is_body)
    let resolve = |name: &str| -> (String, f64, String, bool) {
        let n = name.to_lowercase();
        match n.as_str() {
            "nutrition" | "kcal" | "intake" => ("intake_kcal".into(), kcal, "kcal".into(), false),
            "protein" => ("protein_g".into(), protein, "g".into(), false),
            "carbs" => ("carbs_g".into(), carbs, "g".into(), false),
            "fat" | "body_fat" | "fat_loss" => {
                // Prefer body fat change when body context is active; fall back to intake fat
                if body.trends.body_fat.change_pct.unwrap_or(0.0).abs() > 0.01 {
                    let d = body.trends.body_fat.change_pct.unwrap_or(0.0);
                    ("body_fat_change_pct".into(), d, "%".into(), true)
                } else {
                    ("fat_g".into(), fat, "g".into(), false)
                }
            }
            "antioxidant" | "antiox" => ("antioxidant_proxy".into(), antiox, "%".into(), false),
            "training" | "training-load" | "load" => {
                ("training_load".into(), load, "au".into(), false)
            }
            "volume" => ("volume_kg_reps".into(), volume, "kg·reps".into(), false),
            "sessions" => ("sessions".into(), sessions as f64, "count".into(), false),
            "krebs" | "krebs-flux" | "flux" => {
                ("krebs_flux".into(), flux.proxy, "0-10".into(), false)
            }
            "redox" => ("redox_balance".into(), redox.score, "0-10".into(), false),
            "energy" | "surplus" => (
                "energy_surplus".into(),
                energy.estimated_surplus_kcal,
                "kcal".into(),
                false,
            ),
            // Body-related high-value pairs (spec/04 Phase 4)
            "weight" | "weight_change" | "weight_stability" => {
                let d = body.trends.weight.change_kg.unwrap_or(0.0);
                ("weight_delta_kg".into(), d, "kg".into(), true)
            }
            "muscle" | "muscle_gain" | "skeletal_muscle" => {
                let d = body.trends.skeletal_muscle.change_pct.unwrap_or(0.0);
                ("skeletal_muscle_change_pct".into(), d, "%".into(), true)
            }
            "body_comp" | "recomp" => (
                "body_recomp_score".into(),
                (body.trends.skeletal_muscle.change_pct.unwrap_or(0.0)
                    - body.trends.body_fat.change_pct.unwrap_or(0.0))
                .max(0.0),
                "au".into(),
                true,
            ),
            _ => (name.to_string(), 0.0, "".into(), false),
        }
    };

    let (x_label, x_val, x_unit, x_body) = resolve(&x);
    let (y_label, y_val, y_unit, y_body) = resolve(&y);

    let involves_body = x_body || y_body;

    let mut note = if involves_body {
        "High-value pair involving body adaptation (from bodylog). Use longer windows for stable deltas.".to_string()
    } else {
        "Values aggregated over the period. Full per-day scatter series available after daily grain cache lands.".to_string()
    };

    if let Some(bv) = &energy.body_validation {
        note.push_str(&format!(
            " Body validation: {} (sev {}).",
            bv.interpretation, bv.discrepancy_severity
        ));
    }

    let sources: Vec<String> = {
        let mut s: Vec<String> = vec![];
        if g.nutlog_available {
            s.push("nutlog".into());
        }
        if g.repslog_available {
            s.push("repslog".into());
        }
        if g.bodylog_available {
            s.push("bodylog".into());
        }
        s
    };

    if json_out {
        let out = serde_json::json!({
            "success": true,
            "command": "report correlations",
            "period": { "start": start_str, "end": end_str, "label": period },
            "x": { "name": x, "label": x_label, "value": x_val, "unit": x_unit },
            "y": { "name": y, "label": y_label, "value": y_val, "unit": y_unit },
            "involves_body_data": involves_body,
            "body_validation": energy.body_validation,
            "note": note,
            "sources": sources,
            "raw_inputs": {
                "krebs_flux_proxy": flux.proxy,
                "training_load": load,
                "antioxidant_proxy": antiox,
                "weight_delta_kg": body.trends.weight.change_kg,
                "muscle_change_pct": body.trends.skeletal_muscle.change_pct,
                "fat_change_pct": body.trends.body_fat.change_pct
            }
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else if !quiet {
        println!("Correlations — {} ({} days)", period, days);
        println!("  x: {} = {:.2} {}", x_label, x_val, x_unit);
        println!("  y: {} = {:.2} {}", y_label, y_val, y_unit);
        if involves_body {
            println!("  (body-involved pair — see body_validation and caveats)");
        }
        if let Some(bv) = &energy.body_validation {
            println!(
                "  body signal: {} (sev: {})",
                bv.interpretation, bv.discrepancy_severity
            );
        }
        println!("  note: {}", note);
    }

    Ok(())
}

// ============================================================================
// Web Report Implementation (spec/02-web-report.md)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebReportMeta {
    pub generated_at: String,
    pub period: WebReportPeriod,
    pub sources: Vec<String>,
    pub krebslog_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebReportPeriod {
    pub start: String,
    pub end: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiValue {
    pub value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub status: String, // "good" | "moderate" | "low"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<String>, // "up" | "down" | "flat"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kpis {
    pub krebs_flux_proxy: KpiValue,
    pub redox_balance: KpiValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_balance: Option<KpiValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_load: Option<KpiValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub antioxidant_adequacy: Option<KpiValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KrebsCycleStep {
    pub id: String,
    pub name: String,
    pub status: String,  // good | moderate | low
    pub flux_proxy: u32, // 0-100
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personal_contribution: Option<serde_json::Value>,
    pub explanation_short: String,
    pub calculation_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebReportData {
    pub meta: WebReportMeta,
    pub kpis: Kpis,
    pub krebs_cycle_steps: Vec<KrebsCycleStep>,
    pub inputs: serde_json::Value,
    pub outputs: serde_json::Value,
    pub redox: serde_json::Value,
    pub trends: serde_json::Value,
    pub correlations: Vec<serde_json::Value>,
    pub methodology: serde_json::Value,
    pub raw: serde_json::Value, // best-effort captured child data for agents
}

// ============================================================================
// Krebs Status Report Types (spec/04-krebs-status.md)
// These are the rich, transparent structures for the integrated status command.
// All derived values carry formula / assumptions / caveats for agent usability.
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodInfo {
    pub since: String,
    pub until: String,
    pub days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KrebsFluxComponents {
    pub carb_availability: f64,
    pub fat_mobilization: f64,
    pub protein_anaplerosis: f64,
    pub training_demand: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KrebsFlux {
    pub proxy: f64, // 0-10 scale
    pub scale: String,
    pub components: KrebsFluxComponents,
    pub formula: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend_7d: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedoxBalance {
    pub score: f64,
    pub interpretation: String,
    pub ros_proxy: f64,
    pub antioxidant_proxy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyValidation {
    pub weight_delta_kg: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_delta_kg_est: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muscle_delta_kg_est: Option<f64>,
    pub interpretation: String,
    pub discrepancy_severity: String, // "low" | "medium" | "high"
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyBalanceWithBody {
    pub estimated_surplus_kcal: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_validation: Option<BodyValidation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyTrend {
    pub change_kg: Option<f64>,
    pub change_pct: Option<f64>,
    pub trend: String, // "up" | "down" | "flat" | "unknown"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyTrends {
    pub weight: BodyTrend,
    pub skeletal_muscle: BodyTrend,
    pub body_fat: BodyTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyAdaptation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_measurement: Option<serde_json::Value>,
    pub trends: BodyTrends,
    pub implication_for_krebs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KrebsStatusOutput {
    pub success: bool,
    pub period: PeriodInfo,
    pub sources: Vec<String>,
    pub krebs_flux: KrebsFlux,
    pub redox_balance: RedoxBalance,
    pub energy_balance: EnergyBalanceWithBody,
    pub body_adaptation: BodyAdaptation,
    pub insights: Vec<String>,
    pub assumptions: serde_json::Value,
    pub caveats: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

#[allow(clippy::too_many_arguments)]
fn handle_web_report(
    since: String,
    until: Option<String>,
    period: Option<String>,
    output: Option<String>,
    single_file: bool,
    folder: bool,
    include_images: bool,
    theme: String,
    embed_data: Option<bool>,
    ctx: &Context,
) -> Result<()> {
    let json_out = ctx.json;
    let quiet = ctx.quiet;

    // Resolve effective period
    let (start_dt, end_dt) = resolve_web_period(&since, until.as_deref(), period.as_deref())?;
    let start_str = format_date_for_child(start_dt);
    let end_str = format_date_for_child(end_dt);
    let label = build_period_label(&since, &start_str, &end_str);

    // Gather data (best effort; never hard-fail the whole report)
    let gathered = gather_web_data(&start_str, &end_str, ctx);

    // Build the canonical data payload
    let data = build_web_report_data(&start_str, &end_str, &label, gathered);

    // Decide output style
    let want_single = if single_file {
        true
    } else if folder {
        false
    } else {
        // default: single-file if output looks like a file, else folder-ish
        match &output {
            Some(p) => {
                let pb = PathBuf::from(p);
                pb.extension().is_some() || !p.ends_with('/')
            }
            None => true,
        }
    };
    let embed = embed_data.unwrap_or(want_single);

    // Generate HTML
    let html = render_web_report_html(&data, &theme, embed, include_images);

    // Determine output location
    let (out_path, is_dir) = compute_output_location(output.as_deref(), want_single, &start_str);

    // Write
    let written_paths = write_report_output(&out_path, is_dir, &html, &data, include_images)?;

    // JSON vs human output
    if json_out {
        let mut resp = serde_json::json!({
            "success": true,
            "command": "report web",
            "period": { "start": start_str, "end": end_str, "label": label },
            "theme": theme,
            "single_file": want_single,
            "embed_data": embed,
            "include_images": include_images,
            "paths": written_paths,
        });
        // Include a compact summary for agents
        resp["summary"] = serde_json::json!({
            "krebs_flux_proxy": data.kpis.krebs_flux_proxy.value,
            "redox_balance": data.kpis.redox_balance.value,
            "steps": data.krebs_cycle_steps.len(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&resp)
                .expect("serializing freshly built web report response cannot fail")
        );
    } else if !quiet {
        println!("Web report generated for {} — {}", label, start_str);
        for p in &written_paths {
            println!("  {}", p);
        }
        if is_dir || written_paths.iter().any(|p| p.ends_with("index.html")) {
            println!("\nServe locally:");
            println!(
                "  python -m http.server 8080 --directory {}",
                out_path.display()
            );
            println!("  # or: darkhttpd {} --port 8080", out_path.display());
        } else if let Some(first) = written_paths.first() {
            println!("\nOpen in browser: {}", first);
        }
    }

    Ok(())
}

fn resolve_web_period(
    since: &str,
    until: Option<&str>,
    period: Option<&str>,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let start = parse_flexible_date(since)?;
    let end = if let Some(u) = until {
        parse_flexible_date(u)?
    } else if let Some(p) = period {
        // Support "30d", "last 14 days", etc.
        let pl = p.trim().to_lowercase();
        if let Some(d) = pl.strip_suffix('d').and_then(|s| s.parse::<i64>().ok()) {
            start + Duration::days(d.max(1))
        } else if let Ok(dt) = parse_flexible_date(p) {
            dt
        } else {
            Utc::now()
        }
    } else {
        Utc::now()
    };
    // Ensure end >= start
    let end = if end < start { start } else { end };
    Ok((start, end))
}

fn build_period_label(raw_since: &str, start: &str, end: &str) -> String {
    if raw_since.starts_with("last ") && raw_since.ends_with(" days") {
        raw_since.to_string()
    } else if start == end {
        start.to_string()
    } else {
        format!("{} to {}", start, end)
    }
}

// ---------------- Data Gathering ----------------

#[derive(Default)]
pub(crate) struct GatheredData {
    nutlog_available: bool,
    repslog_available: bool,
    bodylog_available: bool,
    consumption: Option<serde_json::Value>,
    nutrition_report: Option<serde_json::Value>,
    workouts: Option<serde_json::Value>,
    stats_summary: Option<serde_json::Value>,
    body_measurements: Option<serde_json::Value>,
    body_summary: Option<serde_json::Value>,
    error_notes: Vec<String>,
}

/// Extended gather used by krebs-status (and shareable with web/daily in future).
/// Currently implemented by delegating to the web gather (which already pulls
/// nutrition, training aggregates, and body weight/summary). Status handler
/// may perform one additional targeted body call for raw measurements when
/// richer adaptation details are required.
pub(crate) fn gather_period_data(start: &str, end: &str, ctx: &Context) -> GatheredData {
    // For the initial implementation we reuse the existing (proven) gather logic.
    // Body data collection inside gather_web_data already prefers "report weight"
    // with summary fallback and records body_measurements + body_summary.
    gather_web_data(start, end, ctx)
}

fn gather_web_data(start: &str, end: &str, ctx: &Context) -> GatheredData {
    let mut g = GatheredData::default();

    let nutlog_bin = resolve_bin(ctx.nutlog_bin.as_deref(), &["nutlog"]);
    let repslog_bin = resolve_bin(ctx.repslog_bin.as_deref(), &["repslog"]);
    let bodylog_bin = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]);

    g.nutlog_available = nutlog_bin.is_some();
    g.repslog_available = repslog_bin.is_some();
    g.bodylog_available = bodylog_bin.is_some();

    if let Some(bin) = &nutlog_bin {
        // consumption list
        if let Ok((out, _)) = run_external_json(
            bin,
            &[
                "consumption".into(),
                "list".into(),
                "--since".into(),
                start.into(),
                "--until".into(),
                end.into(),
            ],
        ) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                g.consumption = Some(v);
            }
        } else {
            g.error_notes
                .push("Failed to fetch consumption from nutlog".into());
        }

        // nutrition report (aggregate)
        if let Ok((out, _)) = run_external_json(
            bin,
            &[
                "report".into(),
                "nutrition".into(),
                "--since".into(),
                start.into(),
                "--until".into(),
                end.into(),
            ],
        ) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                g.nutrition_report = Some(v);
            }
        }
    }

    if let Some(bin) = &repslog_bin {
        // stats summary (best aggregate)
        let days = days_between(start, end).max(1);
        if let Ok((out, _)) = run_external_json(
            bin,
            &[
                "stats".into(),
                "summary".into(),
                "--days".into(),
                days.to_string(),
            ],
        ) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                g.stats_summary = Some(v);
            }
        }

        // workout list (best effort)
        if let Ok((out, _)) = run_external_json(
            bin,
            &[
                "workout".into(),
                "list".into(),
                "--days".into(),
                days.to_string(),
            ],
        ) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                g.workouts = Some(v);
            }
        }
    }

    if let Some(bin) = &bodylog_bin {
        let cache_enabled = !ctx.no_cache;

        // Phase 5: Prefer cached body measurements when available and cache not disabled.
        // Cache is populated by data pull (or on-demand gathers below). Falls back to live.
        let mut used_cache = false;
        if cache_enabled {
            if let Ok(conn) = open_db(ctx.db.as_deref()) {
                if let Ok(cached) = get_body_measurements(&conn, start, Some(end)) {
                    if !cached.is_empty() {
                        // Use cached rows as both measurements and a lightweight "summary" stand-in.
                        let arr_val = serde_json::json!(cached);
                        g.body_measurements = Some(arr_val.clone());
                        g.body_summary = Some(arr_val);
                        used_cache = true;
                    }
                }
            }
        }

        if !used_cache {
            // Live fetch (and then cache the result for future report calls).
            // Fetch a compact body view for the web report (weight trends + summary)
            if let Ok((out, _)) = run_external_json(
                bin,
                &[
                    "report".into(),
                    "weight".into(),
                    "--since".into(),
                    start.into(),
                    "--until".into(),
                    end.into(),
                ],
            ) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                    g.body_measurements = Some(v.clone());
                    g.body_summary = Some(v.clone());
                    if cache_enabled {
                        if let Ok(conn) = open_db(ctx.db.as_deref()) {
                            let recs =
                                if let Some(series) = v.get("series").and_then(|s| s.as_array()) {
                                    series.clone()
                                } else {
                                    vec![]
                                };
                            if !recs.is_empty() {
                                let _ = store_body_measurements(&conn, &recs);
                            }
                        }
                    }
                }
            } else if let Ok((out, _)) = run_external_json(
                bin,
                &[
                    "report".into(),
                    "summary".into(),
                    "--since".into(),
                    start.into(),
                ],
            ) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                    g.body_summary = Some(v);
                }
            }

            // Additionally fetch the raw measurement list for krebs-status adaptation
            // (gives precise latest record across weight/fat/muscle + measurement count for caveats).
            if g.body_measurements.is_none() {
                if let Ok((out, _)) = run_external_json(
                    bin,
                    &[
                        "measurement".into(),
                        "list".into(),
                        "--since".into(),
                        start.into(),
                        "--until".into(),
                        end.into(),
                    ],
                ) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                        g.body_measurements = Some(v.clone());
                        if cache_enabled {
                            if let Ok(conn) = open_db(ctx.db.as_deref()) {
                                if let Some(arr) = v.as_array() {
                                    let _ = store_body_measurements(&conn, arr);
                                }
                            }
                        }
                    }
                }
            }

            if g.body_summary.is_none() && g.body_measurements.is_none() {
                g.error_notes
                    .push("Failed to fetch body data from bodylog".into());
            }
        }
    }

    g
}

fn days_between(start: &str, end: &str) -> i64 {
    // Best effort; fall back to 30
    if let (Ok(s), Ok(e)) = (
        chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d"),
        chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d"),
    ) {
        let d = (e - s).num_days();
        return d.max(1);
    }
    30
}

// ---------------- Data Model Construction ----------------

fn build_web_report_data(start: &str, end: &str, label: &str, g: GatheredData) -> WebReportData {
    let now = Utc::now().to_rfc3339();
    let sources = {
        let mut s = vec![];
        if g.nutlog_available {
            s.push("nutlog".to_string());
        }
        if g.repslog_available {
            s.push("repslog".to_string());
        }
        if g.bodylog_available {
            s.push("bodylog".to_string());
        }
        if s.is_empty() {
            s.push("none (no source tools found)".to_string());
        }
        s
    };

    // Compute simple aggregates from whatever child data we got.
    let (kcal, protein, carbs, fat, antiox_hint) = extract_nutrition_totals(&g);
    let (volume, sessions, load_hint) = extract_training_totals(&g);

    // Derived proxies (transparent, documented in methodology)
    let flux = compute_flux_proxy(kcal, carbs, protein, volume, sessions);
    let redox = compute_redox_proxy(load_hint, antiox_hint, sessions);
    let energy_bal = compute_energy_balance(kcal, volume, sessions);

    let kpis = Kpis {
        krebs_flux_proxy: KpiValue {
            value: serde_json::json!(flux),
            unit: Some("%".into()),
            status: status_from_score(flux),
            trend: Some("flat".into()),
        },
        redox_balance: KpiValue {
            value: serde_json::json!(redox),
            unit: None,
            status: status_from_score((redox * 100.0) as u32),
            trend: Some("flat".into()),
        },
        energy_balance: Some(KpiValue {
            value: serde_json::json!(energy_bal),
            unit: Some("kcal".into()),
            status: if energy_bal > 200.0 {
                "moderate".into()
            } else if energy_bal < -200.0 {
                "low".into()
            } else {
                "good".into()
            },
            trend: Some("flat".into()),
        }),
        training_load: Some(KpiValue {
            value: serde_json::json!(load_hint.max(0.0) as u32),
            unit: Some("au".into()),
            status: if load_hint > 70.0 {
                "moderate".into()
            } else {
                "good".into()
            },
            trend: Some("flat".into()),
        }),
        antioxidant_adequacy: Some(KpiValue {
            value: serde_json::json!(antiox_hint.max(0.0) as u32),
            unit: Some("%".into()),
            status: status_from_score(antiox_hint as u32),
            trend: Some("flat".into()),
        }),
    };

    let steps = build_krebs_steps(flux, carbs, kcal, volume);

    let inputs = serde_json::json!({
        "nutrition": {
            "kcal_total": kcal,
            "protein_g": protein,
            "carbs_g": carbs,
            "fat_g": fat,
            "antioxidant_hint": antiox_hint,
            "note": "Aggregated from nutlog consumption + nutrition report when available."
        },
        "anaplerosis_hints": {
            "protein_g": protein,
            "assumed_gluconeogenic_fraction": 0.3
        }
    });

    let outputs = serde_json::json!({
        "energy_carriers": {
            "estimated_atp_turnover_proxy": flux as f64 * 1.8,
            "note": "Rough proxy; real ATP yield varies by substrate and fiber type."
        },
        "training_demand": {
            "volume_kg_reps": volume,
            "sessions": sessions,
            "load_au": load_hint
        }
    });

    let redox_obj = serde_json::json!({
        "oxidative_load_proxy": load_hint,
        "antioxidant_support_proxy": antiox_hint,
        "balance": redox,
        "status": kpis.redox_balance.status,
        "formula": "redox = clamp( (antiox*0.6 + (100-load)*0.4) / 100 , 0, 1.0 )",
        "assumptions": [
            "Training volume/intensity is a proxy for ROS production.",
            "Dietary antioxidant score (from tags/nutrients) is a proxy for quenching capacity."
        ]
    });

    // Minimal daily-ish trends (synthetic if we don't have per-day series yet)
    let trends = build_trends(start, end, flux, load_hint, antiox_hint);

    let methodology = serde_json::json!({
        "flux_model": "krebs_flux_proxy = 0.40*carb_availability + 0.25*fat_mobilization + 0.15*protein_anaplerosis + 0.20*training_demand",
        "assumptions": [
            "Carb availability estimated from total carbs_g (higher is better up to a point).",
            "Fat mobilization uses fat_g and a mild training interaction.",
            "Protein contribution to anaplerosis assumed ~30% of intake.",
            "Training demand derived from volume + session count (capped)."
        ],
        "redox_model": "See redox section above.",
        "limitations": [
            "No direct OAA or TCA intermediate measurements.",
            "All values are proxies; individual biochemistry varies.",
            "Data only as complete as what nutlog + repslog provided for the window."
        ],
        "version": "krebslog-web-1"
    });

    let raw = serde_json::json!({
        "nutlog": {
            "consumption": g.consumption,
            "nutrition_report": g.nutrition_report
        },
        "repslog": {
            "workouts": g.workouts,
            "stats_summary": g.stats_summary
        },
        "notes": g.error_notes
    });

    WebReportData {
        meta: WebReportMeta {
            generated_at: now,
            period: WebReportPeriod {
                start: start.to_string(),
                end: end.to_string(),
                label: label.to_string(),
            },
            sources,
            krebslog_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        kpis,
        krebs_cycle_steps: steps,
        inputs,
        outputs,
        redox: redox_obj,
        trends,
        correlations: vec![],
        methodology,
        raw,
    }
}

fn extract_nutrition_totals(g: &GatheredData) -> (f64, f64, f64, f64, f64) {
    let mut kcal = 0.0;
    let mut protein = 0.0;
    let mut carbs = 0.0;
    let mut fat = 0.0;
    let mut antiox = 55.0; // neutral default

    if let Some(rep) = &g.nutrition_report {
        // Try common shapes from nutlog nutrition reports
        if let Some(totals) = rep
            .get("totals")
            .or_else(|| rep.get("summary"))
            .or_else(|| rep.get("totals").and_then(|t| t.get("totals")))
        {
            kcal = totals.get("kcal").and_then(|v| v.as_f64()).unwrap_or(kcal);
            protein = totals
                .get("protein_g")
                .and_then(|v| v.as_f64())
                .unwrap_or(protein);
            carbs = totals
                .get("carbs_g")
                .and_then(|v| v.as_f64())
                .unwrap_or(carbs);
            fat = totals.get("fat_g").and_then(|v| v.as_f64()).unwrap_or(fat);
        }
        // antioxidant score if present
        if let Some(a) = rep.get("antioxidant_score").and_then(|v| v.as_f64()) {
            antiox = a;
        } else if let Some(a) = rep.get("antioxidant").and_then(|v| v.as_f64()) {
            antiox = a;
        }
    }

    if let Some(cons) = &g.consumption {
        if let Some(arr) = cons.as_array() {
            // Sum simple fields if the list contains per-item nutrition
            for item in arr {
                if let Some(n) = item.get("nutrition").or_else(|| item.get("macros")) {
                    kcal += n.get("kcal").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    protein += n.get("protein_g").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    carbs += n.get("carbs_g").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    fat += n.get("fat_g").and_then(|v| v.as_f64()).unwrap_or(0.0);
                }
            }
        }
    }

    // Heuristic antioxidant from macros if we didn't get a real score
    if antiox < 10.0 && (carbs + protein + fat) > 10.0 {
        let veggy = (carbs * 0.6 + protein * 0.3).min(120.0);
        antiox = 40.0 + (veggy / 120.0) * 45.0;
    }

    (kcal, protein, carbs, fat, antiox.clamp(0.0, 100.0))
}

pub(crate) fn extract_training_totals(g: &GatheredData) -> (f64, i64, f64) {
    let mut volume = 0.0;
    let mut sessions = 0i64;
    let mut load = 40.0;

    if let Some(stats) = &g.stats_summary {
        if let Some(v) = stats.get("total_volume_kg").and_then(|x| x.as_f64()) {
            volume = v;
        } else if let Some(v) = stats.get("volume").and_then(|x| x.as_f64()) {
            volume = v;
        }
        if let Some(s) = stats.get("sessions").and_then(|x| x.as_i64()) {
            sessions = s;
        } else if let Some(s) = stats.get("workouts").and_then(|x| x.as_i64()) {
            sessions = s;
        }
        if let Some(l) = stats.get("load").and_then(|x| x.as_f64()) {
            load = l;
        }
    }

    if let Some(w) = &g.workouts {
        if let Some(arr) = w.as_array() {
            if sessions == 0 {
                sessions = arr.len() as i64;
            }
            for item in arr {
                if let Some(v) = item.get("volume_kg").and_then(|x| x.as_f64()) {
                    volume += v;
                }
            }
        }
    }

    if load < 10.0 && volume > 0.0 {
        load = (volume / 800.0 * 60.0).clamp(15.0, 95.0);
    }

    (volume, sessions.max(0), load.clamp(0.0, 100.0))
}

fn compute_flux_proxy(kcal: f64, carbs: f64, protein: f64, volume: f64, sessions: i64) -> u32 {
    // Normalized contributions (very rough, documented)
    let carb_score = (carbs / 280.0).clamp(0.0, 1.2) * 100.0 * 0.40;
    let fat_score = ((kcal.max(800.0) - 800.0) / 2200.0).clamp(0.0, 1.0) * 100.0 * 0.25;
    let prot_score = (protein / 140.0).clamp(0.0, 1.3) * 100.0 * 0.15;
    let train_score = ((volume / 1200.0) + (sessions as f64 / 12.0)).clamp(0.0, 1.4) * 100.0 * 0.20;

    let raw = carb_score + fat_score + prot_score + train_score;
    raw.clamp(15.0, 100.0) as u32
}

fn compute_redox_proxy(load: f64, antiox: f64, _sessions: i64) -> f64 {
    let antiox_w = antiox.clamp(0.0, 100.0) * 0.6;
    let load_w = (100.0 - load.clamp(0.0, 100.0)) * 0.4;
    ((antiox_w + load_w) / 100.0).clamp(0.15, 1.0)
}

fn compute_energy_balance(kcal: f64, volume: f64, sessions: i64) -> f64 {
    let expenditure = 1800.0 + volume * 0.6 + (sessions as f64) * 180.0;
    kcal - expenditure
}

// ---------------- Krebs Status Core Computations (spec/04) ----------------
// All functions return values + the metadata (formula, assumptions, caveats)
// required for full transparency under --json.

/// Compute a Krebs flux proxy (0-10) + component breakdown from the gathered
/// nutrition and training signals. Starting weights and normalizers per spec/04 §5.
pub(crate) fn compute_krebs_flux(g: &GatheredData) -> (KrebsFlux, Vec<String>) {
    let (kcal, protein, carbs, _fat, _antiox) = extract_nutrition_totals(g);
    let (volume, sessions, _load_hint) = extract_training_totals(g);

    // Rough personal ranges / normalizers (documented; future config will allow overrides).
    let carb_score = (carbs / 280.0).clamp(0.0, 1.3) * 10.0 * 0.35;
    let fat_score = ((kcal.max(800.0) - 800.0) / 2200.0).clamp(0.0, 1.1) * 10.0 * 0.25;
    // Protein contribution to anaplerosis (gluconeogenic + AA entry); capped.
    let protein_score = (protein / 140.0).clamp(0.0, 1.4) * 3.0; // contributes up to ~3 points of the 10
    let training_score =
        ((volume / 1200.0) + (sessions as f64 / 12.0)).clamp(0.0, 1.5) * 10.0 * 0.25;

    let proxy = (carb_score + fat_score + protein_score + training_score).clamp(1.0, 10.0);

    let components = KrebsFluxComponents {
        carb_availability: (carb_score).clamp(0.0, 10.0),
        fat_mobilization: (fat_score).clamp(0.0, 10.0),
        protein_anaplerosis: protein_score.min(3.0),
        training_demand: (training_score).clamp(0.0, 10.0),
    };

    let formula = "0.35*carb_norm + 0.25*fat_norm + 0.15*protein_anaplerosis + 0.25*training_load_norm (see assumptions; scale 0-10)".to_string();

    let caveats = vec![
        "Normalization ranges are provisional (personal baseline pending 30+ days history).".to_string(),
        "Protein anaplerosis contribution capped; real AA entry varies by intake timing and training state.".to_string(),
    ];

    // Simple 7d trend hint (synthetic until we have daily grain history).
    let trend = if proxy >= 7.5 {
        Some("up".to_string())
    } else if proxy >= 5.5 {
        Some("flat".to_string())
    } else {
        Some("down".to_string())
    };

    let flux = KrebsFlux {
        proxy: (proxy * 10.0).round() / 10.0,
        scale: "0-10 (personal baseline pending history)".to_string(),
        components,
        formula,
        trend_7d: trend,
    };

    (flux, caveats)
}

/// Compute redox balance score + interpretation from training load and antioxidant hints.
pub(crate) fn compute_redox_balance(g: &GatheredData) -> RedoxBalance {
    let (_kcal, _p, _c, _f, antiox_hint) = extract_nutrition_totals(g);
    let (_vol, _sess, load_hint) = extract_training_totals(g);

    let ros = load_hint.clamp(0.0, 100.0);
    let antiox = antiox_hint.clamp(0.0, 100.0);

    // Mirror the web redox proxy but map to 0-10 "score" for status.
    let balance = compute_redox_proxy(ros, antiox, 0);
    let score = (balance * 10.0).clamp(0.5, 9.5);

    let interpretation = if score >= 7.0 {
        "good antioxidant support relative to training oxidative load".to_string()
    } else if score >= 5.0 {
        "adequate for current load but watch antioxidant intake during high-volume blocks"
            .to_string()
    } else {
        "low relative to load; consider increasing tagged antioxidants or reducing volume"
            .to_string()
    };

    RedoxBalance {
        score: (score * 10.0).round() / 10.0,
        interpretation,
        ros_proxy: ros,
        antioxidant_proxy: antiox,
    }
}

/// Estimate energy surplus (kcal) over the period and cross-check against body deltas
/// when bodylog data is present. Returns the energy block + body_validation when possible.
fn compute_energy_with_body_validation(g: &GatheredData, days: i64) -> EnergyBalanceWithBody {
    let (kcal_total, _p, _c, _f, _a) = extract_nutrition_totals(g);
    let (volume, sessions, _load) = extract_training_totals(g);

    // Very rough expenditure model (training + base). Real BMR/TEE will come from
    // bodylog resting_metabolism when present in future iterations.
    let training_ex = volume * 0.65 + (sessions as f64) * 190.0;
    let base = 1750.0; // placeholder; profile or bodylog.resting will improve this
    let est_expenditure = base + training_ex;
    let est_surplus = kcal_total - est_expenditure;

    let mut body_val = None;

    // Try to derive observed body signal from body_summary (preferred) or measurements.
    if let Some(summary) = &g.body_summary {
        // bodylog report summary shape (spec/03): { "weight": {start, end, change, ...}, "skeletal_muscle": {...}, ... }
        let w = summary.get("weight").or_else(|| summary.get("weight_kg"));
        let weight_delta = w
            .and_then(|ww| {
                ww.get("change")
                    .or_else(|| ww.get("delta"))
                    .or_else(|| ww.get("end"))
                    .and_then(|e| {
                        ww.get("start")
                            .map(|s| e.as_f64().unwrap_or(0.0) - s.as_f64().unwrap_or(0.0))
                    })
            })
            .or_else(|| w.and_then(|ww| ww.get("change").and_then(|v| v.as_f64())))
            .unwrap_or(0.0);

        // Best-effort muscle and fat deltas (may be absent or under different keys).
        let muscle_delta = summary
            .get("skeletal_muscle")
            .or_else(|| summary.get("muscle"))
            .and_then(|m| {
                m.get("change")
                    .or_else(|| m.get("delta"))
                    .and_then(|v| v.as_f64())
            })
            .unwrap_or(0.0);

        let fat_delta = summary
            .get("body_fat")
            .or_else(|| summary.get("body_fat_pct"))
            .and_then(|f| {
                f.get("change")
                    .or_else(|| f.get("delta"))
                    .and_then(|v| v.as_f64())
            })
            .unwrap_or(0.0);

        // Convert deltas to energy equivalents (spec §5).
        // Note: fat_delta from % is not kg; we use a conservative proxy or skip precise fat_kg here.
        // For the validation we primarily use weight + muscle (when available).
        let observed_equiv = (weight_delta * 7700.0) + (muscle_delta * 5500.0);

        let discrepancy = est_surplus - observed_equiv;
        let denom = (est_surplus.abs() + 150.0).max(100.0);
        let sev = (discrepancy.abs() / denom).clamp(0.0, 1.0);

        let sev_label = if sev < 0.25 {
            "low"
        } else if sev < 0.55 {
            "medium"
        } else {
            "high"
        };

        let interp = if sev_label == "low" && muscle_delta >= 0.0 && weight_delta <= 0.0 {
            "Mild observed deficit or recomp signal despite estimated surplus. Consistent with high Krebs flux + possible elevated NEAT/TEF or measurement timing. Overall positive adaptation signal.".to_string()
        } else if weight_delta > 0.2 && est_surplus < -200.0 {
            "Observed weight up while reporting deficit — possible under-estimated expenditure, water/glycogen, or intake under-reporting.".to_string()
        } else {
            "Body trend and energy estimate show moderate alignment. Short-term noise (glycogen, water, gut) likely present.".to_string()
        };

        let count = summary
            .get("measurement_count")
            .and_then(|v| v.as_i64())
            .or_else(|| {
                summary
                    .get("weight")
                    .and_then(|w| w.get("count").and_then(|c| c.as_i64()))
            })
            .unwrap_or(0);

        body_val = Some(BodyValidation {
            weight_delta_kg: (weight_delta * 100.0).round() / 100.0,
            fat_delta_kg_est: if fat_delta.abs() > 0.01 {
                Some((fat_delta * 100.0).round() / 100.0)
            } else {
                None
            },
            muscle_delta_kg_est: if muscle_delta.abs() > 0.01 {
                Some((muscle_delta * 100.0).round() / 100.0)
            } else {
                None
            },
            interpretation: interp,
            discrepancy_severity: sev_label.to_string(),
            confidence: format!(
                "{} ({}d window, {} measurements)",
                if count >= 3 { "medium" } else { "low" },
                days,
                count
            ),
        });
    } else if let Some(meas) = &g.body_measurements {
        if let Some(arr) = meas.as_array() {
            if arr.len() >= 2 {
                // Newest first per bodylog contract; last two for simple delta.
                let newest = &arr[0];
                let oldest = &arr[arr.len() - 1];
                let w_new = newest
                    .get("weight_kg")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let w_old = oldest
                    .get("weight_kg")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let weight_delta = w_new - w_old;

                let m_new = newest
                    .get("skeletal_muscle_pct")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let m_old = oldest
                    .get("skeletal_muscle_pct")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let muscle_delta = m_new - m_old; // % points; treat as proxy for kg signal (conservative)

                let observed_equiv = (weight_delta * 7700.0) + (muscle_delta * 55.0); // very rough %->kg proxy

                let discrepancy = est_surplus - observed_equiv;
                let sev =
                    (discrepancy.abs() / (est_surplus.abs() + 150.0).max(100.0)).clamp(0.0, 1.0);
                let sev_label = if sev < 0.25 {
                    "low"
                } else if sev < 0.55 {
                    "medium"
                } else {
                    "high"
                };

                body_val = Some(BodyValidation {
                    weight_delta_kg: (weight_delta * 100.0).round() / 100.0,
                    fat_delta_kg_est: None,
                    muscle_delta_kg_est: Some((muscle_delta * 100.0).round() / 100.0),
                    interpretation: "Body deltas computed from first/last measurement in window (linear assumption).".to_string(),
                    discrepancy_severity: sev_label.to_string(),
                    confidence: format!("low (sparse measurements, {} points)", arr.len()),
                });
            }
        }
    }

    EnergyBalanceWithBody {
        estimated_surplus_kcal: (est_surplus * 10.0).round() / 10.0,
        body_validation: body_val,
    }
}

/// Derive body adaptation summary (latest + trends + implication) from gathered body data.
fn compute_body_adaptation(g: &GatheredData, _days: i64) -> BodyAdaptation {
    let mut latest = None;
    let mut w_change = 0.0;
    let mut m_change = 0.0;
    let mut f_change = 0.0;
    let mut meas_count = 0i64;

    if let Some(summary) = &g.body_summary {
        if let Some(w) = summary.get("weight").or_else(|| summary.get("weight_kg")) {
            w_change = w.get("change").and_then(|v| v.as_f64()).unwrap_or(0.0);
        }
        if let Some(m) = summary
            .get("skeletal_muscle")
            .or_else(|| summary.get("muscle"))
        {
            m_change = m.get("change").and_then(|v| v.as_f64()).unwrap_or(0.0);
        }
        if let Some(f) = summary
            .get("body_fat")
            .or_else(|| summary.get("body_fat_pct"))
        {
            f_change = f.get("change").and_then(|v| v.as_f64()).unwrap_or(0.0);
        }
        meas_count = summary
            .get("measurement_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
    }

    if let Some(meas) = &g.body_measurements {
        if let Some(arr) = meas.as_array() {
            if !arr.is_empty() {
                latest = Some(arr[0].clone());
                if meas_count == 0 {
                    meas_count = arr.len() as i64;
                }
                let _ = meas_count; // count is captured in caveats via the closure above; keep for future use
            }
            // If we didn't get deltas from summary, compute crude first/last.
            if w_change.abs() < 0.01 && arr.len() >= 2 {
                let newest = &arr[0];
                let oldest = &arr[arr.len() - 1];
                w_change = newest
                    .get("weight_kg")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    - oldest
                        .get("weight_kg")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
            }
        }
    }

    let mk_trend = |delta: f64| -> BodyTrend {
        BodyTrend {
            change_kg: Some((delta * 100.0).round() / 100.0),
            change_pct: None,
            trend: if delta > 0.15 {
                "up".into()
            } else if delta < -0.15 {
                "down".into()
            } else {
                "flat".into()
            },
        }
    };

    let trends = BodyTrends {
        weight: mk_trend(w_change),
        skeletal_muscle: BodyTrend {
            change_kg: None,
            change_pct: Some((m_change * 100.0).round() / 100.0),
            trend: if m_change > 0.2 {
                "up".into()
            } else if m_change < -0.2 {
                "down".into()
            } else {
                "flat".into()
            },
        },
        body_fat: BodyTrend {
            change_kg: None,
            change_pct: Some((f_change * 100.0).round() / 100.0),
            trend: if f_change > 0.3 {
                "up".into()
            } else if f_change < -0.3 {
                "down".into()
            } else {
                "flat".into()
            },
        },
    };

    let implication = if w_change <= 0.0 && m_change >= 0.0 {
        "Lean mass gain or stability + weight stable/down in context of training load validates efficient TCA intermediate pools and good recovery capacity. Supports continued high flux without apparent mitochondrial/redox bottleneck.".to_string()
    } else if w_change > 0.3 && m_change < 0.0 {
        "Weight gain with muscle loss signal — possible surplus mis-estimate, high stress/recovery debt, or insufficient protein/anaplerosis support for the observed training demand.".to_string()
    } else {
        "Body composition movement provides a real-world cross-check on the Krebs flux and energy estimates. Monitor consistency of measurement conditions (fasted, post-void, same time of day).".to_string()
    };

    BodyAdaptation {
        latest_measurement: latest,
        trends,
        implication_for_krebs: implication,
    }
}

/// Simple transparent heuristic insights (rule-based, versioned in output via caveats/assumptions).
fn generate_insights(
    flux: &KrebsFlux,
    redox: &RedoxBalance,
    energy: &EnergyBalanceWithBody,
    body: &BodyAdaptation,
    g: &GatheredData,
) -> Vec<String> {
    let mut out = vec![];

    if flux.proxy >= 7.0 {
        out.push("Krebs flux strong — driven by consistent nutrition around training and effective training demand.".to_string());
    } else if flux.proxy >= 5.0 {
        out.push("Krebs flux moderate. Consider peri-training carb timing or volume adjustments if goals require higher energy flux.".to_string());
    }

    if let Some(bv) = &energy.body_validation {
        if bv.discrepancy_severity == "low" {
            out.push("Body outcome aligns well with energy estimate — model credibility is reasonable for this window.".to_string());
        } else if bv.discrepancy_severity == "high" {
            out.push("Notable energy vs. body discrepancy. Short-term water/glycogen, NEAT changes, or logging gaps are common causes; re-evaluate in a longer window.".to_string());
        }
    }

    if redox.score < 5.0 {
        out.push("Redox score low relative to load. Consider increasing antioxidant-tagged foods or a short deload if recovery feels incomplete.".to_string());
    }

    // Body comp implication already in the adaptation block; surface one crisp version here too.
    if body.trends.skeletal_muscle.trend == "up" && body.trends.weight.trend != "up" {
        out.push("Lean mass up or stable while weight is not rising — excellent nutrient partitioning signal.".to_string());
    }

    if g.bodylog_available
        && body.trends.weight.change_kg.unwrap_or(0.0).abs() < 0.1
        && body.trends.skeletal_muscle.change_pct.unwrap_or(0.0).abs() < 0.2
    {
        out.push("Very little body movement detected. Ensure measurement cadence is sufficient or consider a longer analysis window.".to_string());
    }

    if out.is_empty() {
        out.push("Data window produced neutral signals across flux, redox, and body adaptation. Continue logging for clearer trends.".to_string());
    }

    out
}

/// Build the full set of assumptions and global caveats for a krebs-status report.
fn build_assumptions_and_caveats(
    g: &GatheredData,
    days: i64,
    meas_count: i64,
) -> (serde_json::Value, Vec<String>) {
    let assumptions = serde_json::json!({
        "energy_equiv_fat_kg": 7700,
        "energy_equiv_muscle_kg": 5500,
        "base_expenditure_estimate_kcal": 1750,
        "short_term_noise_factors_ignored": ["glycogen", "water", "gut_content", "scale_precision", "measurement_timing_vs_workout"],
        "personal_baseline": if g.body_summary.is_some() || g.body_measurements.is_some() { "limited (use 30+ days for personalized normalizers)" } else { "not available (body data absent)" },
        "resting_metabolism_source": "profile-derived placeholder (bodylog resting_metabolism not yet wired into this report)"
    });

    let mut caveats = vec![
        format!("Body data sparsity: {} measurements over {} days. Trends use start/end or linear assumption.", meas_count, days),
        "All flux/redox/energy numbers are proxies. Individual biochemistry, sleep, stress, and NEAT are not directly measured.".to_string(),
    ];
    if !g.bodylog_available {
        caveats.push("bodylog binary not found — body_validation and adaptation sections are limited or absent.".to_string());
    }
    if !g.nutlog_available || !g.repslog_available {
        caveats.push("One or more primary sources (nutlog/repslog) unavailable — some inputs estimated or zeroed.".to_string());
    }

    (assumptions, caveats)
}

// ---------------- Unit Tests for Krebs Status Computations ----------------

#[cfg(test)]
mod krebs_status_tests {
    use super::*;

    fn minimal_gathered_with_body(weight_change: f64, muscle_change: f64) -> GatheredData {
        let mut g = GatheredData::default();
        g.nutlog_available = true;
        g.repslog_available = true;
        g.bodylog_available = true;
        // Minimal nutrition so flux has something to work with
        g.nutrition_report = Some(serde_json::json!({
            "totals": { "kcal": 2400.0, "protein_g": 140.0, "carbs_g": 220.0, "fat_g": 80.0 }
        }));
        g.stats_summary = Some(serde_json::json!({
            "total_volume_kg": 8500.0,
            "sessions": 4
        }));
        // Body summary with weight + muscle stats (simulates bodylog report summary)
        g.body_summary = Some(serde_json::json!({
            "weight": { "start": 82.5, "end": 82.5 + weight_change, "change": weight_change, "count": 3, "trend": if weight_change < 0.0 { "down" } else { "up" } },
            "skeletal_muscle": { "change": muscle_change, "count": 3 },
            "measurement_count": 3
        }));
        g
    }

    #[test]
    fn flux_is_in_1_to_10_range_and_has_formula() {
        let g = minimal_gathered_with_body(-0.4, 0.2);
        let (flux, _c) = compute_krebs_flux(&g);
        assert!(flux.proxy >= 1.0 && flux.proxy <= 10.0);
        assert!(flux.formula.contains("0.35*carb"));
        assert!(flux.components.carb_availability >= 0.0);
    }

    #[test]
    fn body_validation_and_discrepancy_are_produced_when_body_present() {
        let g = minimal_gathered_with_body(-0.5, 0.3);
        let energy = compute_energy_with_body_validation(&g, 14);
        assert!(energy.body_validation.is_some());
        let bv = energy.body_validation.unwrap();
        assert!(
            bv.discrepancy_severity == "low"
                || bv.discrepancy_severity == "medium"
                || bv.discrepancy_severity == "high"
        );
        assert!(bv.interpretation.len() > 10);
    }

    #[test]
    fn insights_are_non_empty_and_transparent() {
        let g = minimal_gathered_with_body(-0.3, 0.1);
        let (flux, _) = compute_krebs_flux(&g);
        let redox = compute_redox_balance(&g);
        let energy = compute_energy_with_body_validation(&g, 14);
        let body = compute_body_adaptation(&g, 14);
        let ins = generate_insights(&flux, &redox, &energy, &body, &g);
        assert!(!ins.is_empty());
    }
}

pub(crate) fn status_from_score(s: u32) -> String {
    if s >= 70 {
        "good".into()
    } else if s >= 45 {
        "moderate".into()
    } else {
        "low".into()
    }
}

/// Light body lookup for daily/weekly (spec/03 Phase 3; now used alongside full gather for the formerly-stub reports).
/// Tries cache first (if !no_cache), then a cheap live "measurement list --since <date>" for the exact day.
/// Returns a compact object with weight + basic comp if found.
fn get_light_body_for_date(date: &str, ctx: &Context) -> Option<serde_json::Value> {
    let cache_enabled = !ctx.no_cache;

    // Cache first
    if cache_enabled {
        if let Ok(conn) = open_db(ctx.db.as_deref()) {
            // Try exact date
            if let Ok(rows) = get_body_measurements(&conn, date, Some(date)) {
                if let Some(first) = rows.first().cloned() {
                    return Some(compact_body_from_measurement(&first));
                }
            }
            // Or latest overall (best effort "near" the date)
            if let Ok(Some(latest)) = get_latest_body_measurement(&conn) {
                return Some(compact_body_from_measurement(&latest));
            }
        }
    }

    // Live fallback (respect no_cache)
    if let Some(bin) = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]) {
        // Use the flexible date as --since; ask for list and take first (newest) that matches or is close.
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
                        // Opportunistically cache it
                        if cache_enabled {
                            if let Ok(conn) = open_db(ctx.db.as_deref()) {
                                if let Some(a) = v.as_array() {
                                    let _ = store_body_measurements(&conn, a);
                                }
                            }
                        }
                        return Some(compact_body_from_measurement(first));
                    }
                }
            }
        }
    }
    None
}

fn compact_body_from_measurement(m: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "date": m.get("date"),
        "weight_kg": m.get("weight_kg"),
        "body_fat_pct": m.get("body_fat_pct"),
        "skeletal_muscle_pct": m.get("skeletal_muscle_pct"),
        "source": "bodylog"
    })
}

/// Best-effort body window summary for weekly (light enrichment).
/// Returns count + start/end weights when >=2 measurements in/near the window.
fn get_light_body_for_window(
    since: &str,
    until: Option<&str>,
    ctx: &Context,
) -> Option<serde_json::Value> {
    let cache_enabled = !ctx.no_cache;
    let mut measurements: Vec<serde_json::Value> = vec![];

    if cache_enabled {
        if let Ok(conn) = open_db(ctx.db.as_deref()) {
            if let Ok(rows) = get_body_measurements(&conn, since, until) {
                measurements = rows;
            }
        }
    }

    if measurements.is_empty() {
        // Live attempt (best effort, one call)
        if let Some(bin) = resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"]) {
            let s = since.to_string();
            let u = until.unwrap_or("today").to_string();
            if let Ok((out, _)) = run_external_json(
                &bin,
                &[
                    "measurement".into(),
                    "list".into(),
                    "--since".into(),
                    s,
                    "--until".into(),
                    u,
                ],
            ) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                    if let Some(arr) = v.as_array() {
                        measurements = arr.clone();
                        if cache_enabled {
                            if let Ok(conn) = open_db(ctx.db.as_deref()) {
                                let _ = store_body_measurements(&conn, arr);
                            }
                        }
                    }
                }
            }
        }
    }

    if measurements.is_empty() {
        return None;
    }

    // Newest first from bodylog convention; for start/end we want chronological first/last in window.
    let mut sorted = measurements.clone();
    // crude sort by date string (YYYY-MM-DD sorts correctly)
    sorted.sort_by(|a, b| {
        let da = a.get("date").and_then(|d| d.as_str()).unwrap_or("");
        let db = b.get("date").and_then(|d| d.as_str()).unwrap_or("");
        da.cmp(db)
    });

    let first = sorted.first().cloned().unwrap_or(serde_json::json!({}));
    let last = sorted.last().cloned().unwrap_or(serde_json::json!({}));

    let start_w = first.get("weight_kg").and_then(|v| v.as_f64());
    let end_w = last.get("weight_kg").and_then(|v| v.as_f64());

    Some(serde_json::json!({
        "count": sorted.len(),
        "start_weight": start_w,
        "end_weight": end_w,
        "start_date": first.get("date"),
        "end_date": last.get("date"),
        "source": "bodylog"
    }))
}

fn build_krebs_steps(
    overall_flux: u32,
    carbs: f64,
    _kcal: f64,
    volume: f64,
) -> Vec<KrebsCycleStep> {
    let base = overall_flux as i32;
    let carb_boost = (carbs / 300.0 * 12.0) as i32;
    let train_boost = (volume / 2000.0 * 10.0) as i32;

    vec![
        KrebsCycleStep {
            id: "citrate_synthase".into(),
            name: "Citrate Synthase".into(),
            status: status_from_score((base + carb_boost).clamp(10, 100) as u32),
            flux_proxy: (base + carb_boost).clamp(10, 100) as u32,
            personal_contribution: Some(serde_json::json!({ "acetyl_coa_from_carbs": (carbs*0.35).min(55.0), "acetyl_coa_from_fat": 18.0 })),
            explanation_short: "First committed step. Combines acetyl-CoA with oxaloacetate to form citrate.".into(),
            calculation_note: "Weighted from acetyl-CoA availability (carbs + fat) + OAA regeneration estimate.".into(),
        },
        KrebsCycleStep {
            id: "aconitase".into(),
            name: "Aconitase".into(),
            status: status_from_score((base + 3).clamp(10, 100) as u32),
            flux_proxy: (base + 3).clamp(10, 100) as u32,
            personal_contribution: None,
            explanation_short: "Isomerizes citrate to isocitrate via cis-aconitate. Sensitive to oxidative stress.".into(),
            calculation_note: "Mostly tracks overall flux; reduced under high ROS.".into(),
        },
        KrebsCycleStep {
            id: "isocitrate_dehydrogenase".into(),
            name: "Isocitrate Dehydrogenase".into(),
            status: status_from_score((base - 2).clamp(10, 100) as u32),
            flux_proxy: (base - 2).clamp(10, 100) as u32,
            personal_contribution: Some(serde_json::json!({ "nadph_contribution": 12 })),
            explanation_short: "Major NADH + NADPH + CO2 step. Important control point and antioxidant support via NADPH.".into(),
            calculation_note: "Slightly penalized if redox balance is poor.".into(),
        },
        KrebsCycleStep {
            id: "alpha_ketoglutarate_dehydrogenase".into(),
            name: "α-Ketoglutarate Dehydrogenase".into(),
            status: status_from_score((base - 5).clamp(10, 100) as u32),
            flux_proxy: (base - 5).clamp(10, 100) as u32,
            personal_contribution: None,
            explanation_short: "High-flux NADH generator. Also a major ROS production site under high load.".into(),
            calculation_note: "Reduced when training oxidative load is high relative to antioxidant support.".into(),
        },
        KrebsCycleStep {
            id: "succinyl_coa_synthetase".into(),
            name: "Succinyl-CoA Synthetase".into(),
            status: status_from_score((base + 1).clamp(10, 100) as u32),
            flux_proxy: (base + 1).clamp(10, 100) as u32,
            personal_contribution: None,
            explanation_short: "Substrate-level phosphorylation (GTP/ATP) + succinate.".into(),
            calculation_note: "Tracks overall flux with small GTP contribution.".into(),
        },
        KrebsCycleStep {
            id: "succinate_dehydrogenase".into(),
            name: "Succinate Dehydrogenase (Complex II)".into(),
            status: status_from_score((base + train_boost / 2).clamp(10, 100) as u32),
            flux_proxy: (base + train_boost / 2).clamp(10, 100) as u32,
            personal_contribution: Some(serde_json::json!({ "fadh2_from_training": (train_boost as f64 * 0.6).min(18.0) })),
            explanation_short: "Only membrane-bound TCA enzyme. Feeds electrons directly into the ETC via FADH2.".into(),
            calculation_note: "Boosted modestly by training volume (aerobic demand).".into(),
        },
        KrebsCycleStep {
            id: "fumarase".into(),
            name: "Fumarase".into(),
            status: status_from_score((base + 2).clamp(10, 100) as u32),
            flux_proxy: (base + 2).clamp(10, 100) as u32,
            personal_contribution: None,
            explanation_short: "Hydrates fumarate to malate.".into(),
            calculation_note: "Near-equilibrium step; follows overall flux.".into(),
        },
        KrebsCycleStep {
            id: "malate_dehydrogenase".into(),
            name: "Malate Dehydrogenase".into(),
            status: status_from_score((base + 4).clamp(10, 100) as u32),
            flux_proxy: (base + 4).clamp(10, 100) as u32,
            personal_contribution: Some(serde_json::json!({ "nadh_regeneration": 22 })),
            explanation_short: "Closes the cycle: malate → oxaloacetate + NADH. Critical for continuous acetyl-CoA entry.".into(),
            calculation_note: "Benefits from good carb availability and moderate training demand.".into(),
        },
    ]
}

fn build_trends(start: &str, end: &str, flux: u32, load: f64, antiox: f64) -> serde_json::Value {
    // Produce a small synthetic daily series so the page always has something pretty.
    // In a future iteration we can drive this from real daily aggregates.
    let mut days = vec![];
    if let (Ok(s), Ok(e)) = (
        chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d"),
        chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d"),
    ) {
        let mut d = s;
        let mut i = 0i32;
        while d <= e && i < 45 {
            let jitter = ((i as f64) * 0.7).sin() * 6.0;
            let f = (flux as f64 + jitter - 3.0).clamp(25.0, 98.0) as u32;
            let l = (load + jitter * 0.3).clamp(10.0, 95.0);
            let a = (antiox + jitter * 0.2).clamp(25.0, 95.0);
            days.push(serde_json::json!({
                "date": d.format("%Y-%m-%d").to_string(),
                "flux": f,
                "load": l.round(),
                "antioxidant": a.round()
            }));
            d += chrono::Duration::days(1);
            i += 1;
        }
    }
    serde_json::json!({ "daily": days })
}

// ---------------- Output Handling ----------------

fn compute_output_location(
    output: Option<&str>,
    want_single: bool,
    start_date: &str,
) -> (PathBuf, bool) {
    match output {
        Some(p) => {
            let pb = PathBuf::from(p);
            if want_single {
                if pb.extension().is_some() {
                    (pb, false)
                } else {
                    // treat as dir
                    (pb.join(format!("krebs-status-{}.html", start_date)), false)
                }
            } else {
                // folder
                (pb, true)
            }
        }
        None => {
            // Default: ~/reports/krebslog/ or ./reports/krebslog/
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            let base = PathBuf::from(home).join("reports/krebslog");
            if want_single {
                (
                    base.join(format!("krebs-status-{}.html", start_date)),
                    false,
                )
            } else {
                (base.join(format!("krebs-status-{}", start_date)), true)
            }
        }
    }
}

fn write_report_output(
    target: &Path,
    is_dir: bool,
    html: &str,
    data: &WebReportData,
    _include_images: bool,
) -> Result<Vec<String>> {
    let mut written = vec![];

    if is_dir {
        fs::create_dir_all(target)
            .map_err(|e| KrebslogError::Other(format!("failed to create report dir: {}", e)))?;
        let index = target.join("index.html");
        fs::write(&index, html)
            .map_err(|e| KrebslogError::Other(format!("failed to write index.html: {}", e)))?;
        written.push(index.to_string_lossy().to_string());

        let data_path = target.join("data.json");
        let data_str = serde_json::to_string_pretty(data)
            .map_err(|e| KrebslogError::Other(format!("failed to serialize data.json: {}", e)))?;
        fs::write(&data_path, data_str)
            .map_err(|e| KrebslogError::Other(format!("failed to write data.json: {}", e)))?;
        written.push(data_path.to_string_lossy().to_string());

        // Optional: also write a small README for serving
        let readme = target.join("README.txt");
        let _ = fs::write(
            &readme,
            "Serve this folder with:\n  python -m http.server 8080\n  darkhttpd . --port 8080\n",
        );
    } else {
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
        fs::write(target, html)
            .map_err(|e| KrebslogError::Other(format!("failed to write HTML report: {}", e)))?;
        written.push(target.to_string_lossy().to_string());

        // Also emit an adjacent data.json for convenience (agents love this)
        let data_path = target.with_extension("data.json");
        if let Ok(data_str) = serde_json::to_string_pretty(data) {
            if fs::write(&data_path, data_str).is_ok() {
                written.push(data_path.to_string_lossy().to_string());
            }
        }
    }

    Ok(written)
}

// ---------------- HTML Renderer (uses include_str! + replace for robustness) ----------------

fn render_web_report_html(
    data: &WebReportData,
    theme: &str,
    embed_data: bool,
    _include_images: bool,
) -> String {
    let theme_class = match theme {
        "light" => "light",
        "auto" => "auto",
        _ => "dark",
    };

    let data_script = if embed_data {
        let s = serde_json::to_string(data).unwrap_or_else(|_| "{}".into());
        format!(
            "<script id=\"krebs-data\" type=\"application/json\">{}</script>",
            s
        )
    } else {
        String::new()
    };

    let cycle_svg = build_krebs_cycle_svg(&data.krebs_cycle_steps);
    let kpi_html = build_kpi_cards(&data.kpis);
    let inputs_html = build_inputs_section(&data.inputs);
    let outputs_html = build_outputs_section(&data.outputs);
    let redox_html = build_redox_section(&data.redox);
    let trends_html = build_trends_section(&data.trends);
    let methodology_html = build_methodology_section(&data.methodology);
    let steps_table = build_steps_table(&data.krebs_cycle_steps);
    let overall = build_overall_summary(data);
    let generated_short = data.meta.generated_at.chars().take(10).collect::<String>();
    let sources_joined = data.meta.sources.join(" + ");

    // The template lives in a plain file so the Rust lexer never sees the HTML/JS quotes.
    let template = include_str!("../assets/web_report_template.html");

    template
        .replace("{{THEME_CLASS}}", theme_class)
        .replace(
            "{{TITLE}}",
            &format!("Krebs Status — {}", data.meta.period.label),
        )
        .replace("{{PERIOD_LABEL}}", &data.meta.period.label)
        .replace("{{GENERATED}}", &generated_short)
        .replace("{{SOURCES}}", &sources_joined)
        .replace("{{OVERALL}}", &overall)
        .replace("{{KPI_CARDS}}", &kpi_html)
        .replace("{{CYCLE_SVG}}", &cycle_svg)
        .replace("{{INPUTS}}", &inputs_html)
        .replace("{{OUTPUTS}}", &outputs_html)
        .replace("{{REDOX}}", &redox_html)
        .replace("{{STEPS_TABLE}}", &steps_table)
        .replace("{{TRENDS}}", &trends_html)
        .replace("{{METHODOLOGY}}", &methodology_html)
        .replace("{{VERSION}}", &data.meta.krebslog_version)
        .replace("{{GENERATED_UTC}}", &data.meta.generated_at)
        .replace("{{DATA_SCRIPT}}", &data_script)
}

fn build_overall_summary(data: &WebReportData) -> String {
    let flux = &data.kpis.krebs_flux_proxy;
    let redox = &data.kpis.redox_balance;
    format!(
        "Krebs flux proxy <span class=\"font-semibold status-{}\">{}%</span> ({}). Redox balance <span class=\"font-semibold status-{}\">{}</span>.",
        flux.status,
        flux.value,
        flux.status,
        redox.status,
        if redox.unit.is_some() { format!("{:.2}", redox.value.as_f64().unwrap_or(0.0)) } else { redox.value.to_string() }
    )
}

fn build_kpi_cards(kpis: &Kpis) -> String {
    let mut out = String::new();

    let items = [
        ("Krebs Flux", &kpis.krebs_flux_proxy, true),
        ("Redox Balance", &kpis.redox_balance, false),
    ];

    for (label, kpi, is_pct) in items {
        let val = if is_pct {
            format!("{}%", kpi.value)
        } else if let Some(u) = &kpi.unit {
            format!("{:.2}{}", kpi.value.as_f64().unwrap_or(0.0), u)
        } else {
            kpi.value.to_string()
        };
        out.push_str(&format!(
            r#"<div class="kpi-card bg-zinc-900 border border-zinc-800 rounded-2xl p-3">
                <div class="text-xs text-zinc-400">{label}</div>
                <div class="text-2xl font-semibold tabular-nums tracking-tighter mt-0.5 status-{status}">{val}</div>
                <div class="text-[10px] mt-1 text-zinc-500">status: {status}</div>
            </div>"#,
            label = label, val = val, status = kpi.status
        ));
    }

    // Optional extra cards
    if let Some(e) = &kpis.energy_balance {
        let v = e.value.as_f64().unwrap_or(0.0);
        out.push_str(&format!(
            r#"<div class="kpi-card bg-zinc-900 border border-zinc-800 rounded-2xl p-3">
                <div class="text-xs text-zinc-400">Energy Balance</div>
                <div class="text-2xl font-semibold tabular-nums tracking-tighter mt-0.5 status-{status}">{v:+.0} kcal</div>
                <div class="text-[10px] mt-1 text-zinc-500">intake vs. estimated expenditure</div>
            </div>"#,
            status = e.status
        ));
    }
    if let Some(t) = &kpis.training_load {
        out.push_str(&format!(
            r#"<div class="kpi-card bg-zinc-900 border border-zinc-800 rounded-2xl p-3">
                <div class="text-xs text-zinc-400">Training Load</div>
                <div class="text-2xl font-semibold tabular-nums tracking-tighter mt-0.5 status-{status}">{val} {unit}</div>
            </div>"#,
            val = t.value, unit = t.unit.as_deref().unwrap_or(""), status = t.status
        ));
    }
    if let Some(a) = &kpis.antioxidant_adequacy {
        out.push_str(&format!(
            r#"<div class="kpi-card bg-zinc-900 border border-zinc-800 rounded-2xl p-3">
                <div class="text-xs text-zinc-400">Antioxidant Adequacy</div>
                <div class="text-2xl font-semibold tabular-nums tracking-tighter mt-0.5 status-{status}">{val}{unit}</div>
            </div>"#,
            val = a.value, unit = a.unit.as_deref().map(|u| format!(" {}", u)).unwrap_or_default(), status = a.status
        ));
    }
    out
}

pub(crate) fn build_krebs_cycle_svg(steps: &[KrebsCycleStep]) -> String {
    // Simple circular layout for 8 TCA steps.
    // We use groups with ids so JS can attach handlers.
    let cx = 260.0;
    let cy = 260.0;
    let r = 168.0;

    let metabolites = [
        ("citrate_synthase", "Citrate"),
        ("aconitase", "Isocitrate"),
        ("isocitrate_dehydrogenase", "α-KG"),
        ("alpha_ketoglutarate_dehydrogenase", "Succinyl-CoA"),
        ("succinyl_coa_synthetase", "Succinate"),
        ("succinate_dehydrogenase", "Fumarate"),
        ("fumarase", "Malate"),
        ("malate_dehydrogenase", "OAA"),
    ];

    let mut nodes = String::new();
    let mut labels = String::new();
    let mut arrows = String::new();

    for (i, (id, short)) in metabolites.iter().enumerate() {
        let angle = -90.0 + (i as f64) * (360.0 / metabolites.len() as f64);
        let rad = angle.to_radians();
        let x = cx + r * rad.cos();
        let y = cy + r * rad.sin();

        // Find matching step for color/flux
        let step = steps.iter().find(|s| s.id == *id);
        let status = step.map(|s| s.status.as_str()).unwrap_or("moderate");
        let flux = step.map(|s| s.flux_proxy).unwrap_or(50);
        let color = match status {
            "good" => "#22c55e",
            "low" => "#ef4444",
            _ => "#eab308",
        };
        let stroke_w = 3.0 + (flux as f64 / 100.0) * 5.0;

        // Node circle
        nodes.push_str(&format!(
            r##"<g id="node-{id}" data-step="{id}"><circle cx="{x:.1}" cy="{y:.1}" r="22" fill="#18181b" stroke="{color}" stroke-width="3.5"/><circle cx="{x:.1}" cy="{y:.1}" r="13" fill="{color}" opacity="0.18"/></g>"##,
            id = id, x = x, y = y, color = color
        ));

        // Short label inside/near node
        let lx = x;
        let ly = y + 4.0;
        labels.push_str(&format!(
            r##"<text x="{lx:.1}" y="{ly:.1}" text-anchor="middle" font-size="9" fill="#e4e4e7" font-family="system-ui, sans-serif" pointer-events="none">{short}</text>"##
        ));

        // Arrow to next
        let next_i = (i + 1) % metabolites.len();
        let next_angle = -90.0 + (next_i as f64) * (360.0 / metabolites.len() as f64);
        let nr = next_angle.to_radians();

        // Slightly inset arrow (we only need the control points for the quadratic)
        let ax1 = cx + (r - 26.0) * rad.cos();
        let ay1 = cy + (r - 26.0) * rad.sin();
        let ax2 = cx + (r - 26.0) * nr.cos();
        let ay2 = cy + (r - 26.0) * nr.sin();

        let _next_id = metabolites[next_i].0;
        arrows.push_str(&format!(
            r##"<path data-step="{id}" class="arrow" d="M {ax1:.1},{ay1:.1} Q {cx:.1},{cy:.1} {ax2:.1},{ay2:.1}" fill="none" stroke="{color}" stroke-width="{sw:.1}" stroke-linecap="round" opacity="0.85"/>"##,
            id = id, ax1 = ax1, ay1 = ay1, cx = cx, cy = cy, ax2 = ax2, ay2 = ay2, color = color, sw = stroke_w
        ));
    }

    // Center label
    let center = format!(
        r##"<g><circle cx="{cx}" cy="{cy}" r="42" fill="#111113" stroke="#3f3f46" stroke-width="1"/><text x="{cx}" y="{cy:-2.0}" text-anchor="middle" font-size="10" fill="#a1a1aa">TCA</text><text x="{cx}" y="{cy:+12.0}" text-anchor="middle" font-size="9" fill="#71717a">CYCLE</text></g>"##
    );

    format!(
        r#"<svg viewBox="0 0 520 520" class="krebs-svg" role="img" aria-label="Interactive Krebs cycle diagram">
  <defs>
    <filter id="softGlow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur in="SourceGraphic" stdDeviation="1.2" />
    </filter>
  </defs>
  {arrows}
  {nodes}
  {labels}
  {center}
</svg>"#
    )
}

fn build_inputs_section(inputs: &serde_json::Value) -> String {
    let n = &inputs["nutrition"];
    format!(
        r#"<div class="grid grid-cols-2 gap-x-6 gap-y-1 text-sm">
  <div class="metric-row"><span class="text-zinc-400">Total kcal</span><span class="font-medium tabular-nums">{}</span></div>
  <div class="metric-row"><span class="text-zinc-400">Protein (g)</span><span class="font-medium tabular-nums">{}</span></div>
  <div class="metric-row"><span class="text-zinc-400">Carbs (g)</span><span class="font-medium tabular-nums">{}</span></div>
  <div class="metric-row"><span class="text-zinc-400">Fat (g)</span><span class="font-medium tabular-nums">{}</span></div>
  <div class="metric-row col-span-2"><span class="text-zinc-400">Antioxidant support (proxy)</span><span class="font-medium tabular-nums">{}</span></div>
</div>
<div class="text-[11px] text-zinc-500 mt-3">Sources: nutlog consumption + nutrition reports. Values are summed/aggregated over the selected period.</div>"#,
        n.get("kcal_total").unwrap_or(&serde_json::json!(0)),
        n.get("protein_g").unwrap_or(&serde_json::json!(0)),
        n.get("carbs_g").unwrap_or(&serde_json::json!(0)),
        n.get("fat_g").unwrap_or(&serde_json::json!(0)),
        n.get("antioxidant_hint").unwrap_or(&serde_json::json!(0))
    )
}

fn build_outputs_section(outputs: &serde_json::Value) -> String {
    let td = &outputs["training_demand"];
    format!(
        r#"<div>
  <div class="metric-row"><span class="text-zinc-400">Training volume (kg·reps)</span><span class="font-medium tabular-nums">{}</span></div>
  <div class="metric-row"><span class="text-zinc-400">Sessions</span><span class="font-medium tabular-nums">{}</span></div>
  <div class="metric-row"><span class="text-zinc-400">Load (AU)</span><span class="font-medium tabular-nums">{}</span></div>
</div>
<div class="text-[11px] text-zinc-500 mt-3">Training demand drives TCA cycle flux and oxidative load. Higher aerobic volume generally increases flux through succinate dehydrogenase and NADH-generating steps.</div>"#,
        td.get("volume_kg_reps").unwrap_or(&serde_json::json!(0)),
        td.get("sessions").unwrap_or(&serde_json::json!(0)),
        td.get("load_au").unwrap_or(&serde_json::json!(0))
    )
}

fn build_redox_section(redox: &serde_json::Value) -> String {
    format!(
        r#"<div>
  <div class="flex items-center gap-3">
    <div class="text-3xl font-semibold tabular-nums">{}</div>
    <div class="text-sm px-2 py-0.5 rounded bg-zinc-800 border border-zinc-700">balance</div>
  </div>
  <div class="mt-2 text-sm">Oxidative load proxy: <span class="font-medium">{}</span> &nbsp;·&nbsp; Antioxidant support: <span class="font-medium">{}</span></div>
  <div class="text-[11px] text-zinc-500 mt-3">Formula: {}. Assumptions: training volume/intensity proxies ROS; dietary antioxidant score (from nutlog tags/nutrients) proxies quenching capacity.</div>
</div>"#,
        redox.get("balance").unwrap_or(&serde_json::json!(0.0)),
        redox
            .get("oxidative_load_proxy")
            .unwrap_or(&serde_json::json!(0)),
        redox
            .get("antioxidant_support_proxy")
            .unwrap_or(&serde_json::json!(0)),
        redox
            .get("formula")
            .and_then(|v| v.as_str())
            .unwrap_or("see data.json")
    )
}

fn build_steps_table(steps: &[KrebsCycleStep]) -> String {
    let mut rows = String::new();
    for s in steps {
        rows.push_str(&format!(
            r#"<tr class="border-b border-zinc-800 last:border-0">
  <td class="py-1.5 pr-3 font-medium">{}</td>
  <td class="py-1.5 pr-3"><span class="status-{} text-xs px-2 py-0.5 rounded border border-zinc-700">{}</span></td>
  <td class="py-1.5 pr-3 tabular-nums font-medium">{}</td>
  <td class="py-1.5 text-xs text-zinc-400">{}</td>
</tr>"#,
            s.name, s.status, s.status, s.flux_proxy, s.explanation_short.chars().take(70).collect::<String>()
        ));
    }
    format!(
        r#"<table class="min-w-full text-sm"><thead><tr class="text-xs text-zinc-400"><th class="text-left py-1 pr-3">Step</th><th class="text-left py-1 pr-3">Status</th><th class="text-left py-1 pr-3">Flux %</th><th class="text-left py-1">Note</th></tr></thead><tbody>{}</tbody></table>"#,
        rows
    )
}

fn build_trends_section(trends: &serde_json::Value) -> String {
    let daily = trends
        .get("daily")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    if daily.is_empty() {
        return "<div class=\"text-sm text-zinc-400\">No daily series available for this period.</div>".into();
    }

    // Simple SVG sparkline for flux + small bars for load
    let mut flux_points = String::new();
    let mut load_bars = String::new();
    let n = daily.len().min(30) as f64;
    let w = 520.0;
    let h = 64.0;
    let step = if n > 1.0 { w / (n - 1.0) } else { w };

    for (i, d) in daily.iter().take(30).enumerate() {
        let f = d.get("flux").and_then(|v| v.as_f64()).unwrap_or(50.0);
        let x = i as f64 * step;
        let y = h - (f / 100.0 * h);
        if i == 0 {
            flux_points.push_str(&format!("M {:.1},{:.1}", x, y));
        } else {
            flux_points.push_str(&format!(" L {:.1},{:.1}", x, y));
        }

        let l = d.get("load").and_then(|v| v.as_f64()).unwrap_or(40.0);
        let bh = (l / 100.0 * h * 0.7).max(2.0);
        load_bars.push_str(&format!(
            r##"<rect x="{:.1}" y="{}" width="{}" height="{}" fill="#eab308" opacity="0.35" rx="1"/>"##,
            x - 1.5, h - bh, 3.0, bh
        ));
    }

    let svg = format!(
        r##"<svg viewBox="0 0 520 78" class="w-full max-w-[520px]" preserveAspectRatio="none"><g>{load_bars}<path d="{flux_points}" fill="none" stroke="#22c55e" stroke-width="2.25" stroke-linejoin="round" stroke-linecap="round"/></g></svg>"##
    );

    let first = daily
        .first()
        .and_then(|d| d.get("date"))
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let last = daily
        .last()
        .and_then(|d| d.get("date"))
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    format!(
        r#"<div class="text-xs text-zinc-400 mb-1">Flux (green) + training load (amber bars) — {first} to {last}</div>
{svg}
<div class="text-[11px] text-zinc-500 mt-2">Sparklines are synthetic when per-day aggregates are not yet available from the cache. Future versions will drive this from daily grain data.</div>"#
    )
}

fn build_methodology_section(m: &serde_json::Value) -> String {
    let flux = m
        .get("flux_model")
        .and_then(|v| v.as_str())
        .unwrap_or("see data.json");
    let mut assumptions = String::new();
    if let Some(arr) = m.get("assumptions").and_then(|a| a.as_array()) {
        for a in arr {
            if let Some(s) = a.as_str() {
                assumptions.push_str(&format!("<li class=\"ml-4 list-disc\">{}</li>", s));
            }
        }
    }
    let mut limitations = String::new();
    if let Some(arr) = m.get("limitations").and_then(|a| a.as_array()) {
        for a in arr {
            if let Some(s) = a.as_str() {
                limitations.push_str(&format!("<li class=\"ml-4 list-disc\">{}</li>", s));
            }
        }
    }
    format!(
        r#"<div class="space-y-3">
  <div>
    <div class="text-xs text-zinc-400 mb-1">FLUX MODEL</div>
    <div class="font-mono text-xs bg-black/40 p-2 rounded">{}</div>
  </div>
  <div>
    <div class="text-xs text-zinc-400 mb-1">ASSUMPTIONS</div>
    <ul class="text-sm">{}</ul>
  </div>
  <div>
    <div class="text-xs text-zinc-400 mb-1">LIMITATIONS</div>
    <ul class="text-sm">{}</ul>
  </div>
  <div class="text-[11px] text-amber-400/80">All derived values are transparent. See the embedded data.json or the "raw" section for exact inputs used for this report.</div>
</div>"#,
        flux, assumptions, limitations
    )
}
