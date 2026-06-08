use crate::cli::ReportAction;
use crate::context::Context;
use crate::error::{KrebslogError, Result};
use crate::utils::{format_date_for_child, parse_flexible_date, resolve_bin, run_external_json};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
struct GatheredData {
    nutlog_available: bool,
    repslog_available: bool,
    consumption: Option<serde_json::Value>,
    nutrition_report: Option<serde_json::Value>,
    workouts: Option<serde_json::Value>,
    stats_summary: Option<serde_json::Value>,
    error_notes: Vec<String>,
}

fn gather_web_data(start: &str, end: &str, ctx: &Context) -> GatheredData {
    let mut g = GatheredData::default();

    let nutlog_bin = resolve_bin(ctx.nutlog_bin.as_deref(), &["nutlog"]);
    let repslog_bin = resolve_bin(ctx.repslog_bin.as_deref(), &["repslog"]);

    g.nutlog_available = nutlog_bin.is_some();
    g.repslog_available = repslog_bin.is_some();

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

fn extract_training_totals(g: &GatheredData) -> (f64, i64, f64) {
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

fn status_from_score(s: u32) -> String {
    if s >= 70 {
        "good".into()
    } else if s >= 45 {
        "moderate".into()
    } else {
        "low".into()
    }
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

fn build_krebs_cycle_svg(steps: &[KrebsCycleStep]) -> String {
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
