use crate::cli::DataAction;
use crate::context::Context;
use crate::db::{open_db, record_pull, store_body_measurements};
use crate::error::{KrebslogError, Result};
use crate::utils::{format_date_for_child, parse_flexible_date, resolve_bin, run_external_json};
use chrono::{Duration, Utc};
use colored::*;
use comfy_table::{presets, Cell, Table};
use serde_json::Value;

/// Handle the `krebslog data ...` command group.
pub fn handle_data(action: DataAction, ctx: &Context) -> Result<()> {
    match action {
        DataAction::Pull {
            source,
            entity,
            since,
            until,
            all,
            period,
            dry_run,
        } => handle_pull(source, entity, since, until, all, period, dry_run, ctx),
        DataAction::Status { probe } => handle_status(probe, ctx),
    }
}

fn handle_status(probe: bool, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    let nutlog_override = ctx.nutlog_bin.as_deref();
    let repslog_override = ctx.repslog_bin.as_deref();
    let bodylog_override = ctx.bodylog_bin.as_deref();
    let db_override = ctx.db.as_deref();

    let nutlog_bin = resolve_bin(nutlog_override, &["nutlog"]);
    let repslog_bin = resolve_bin(repslog_override, &["repslog"]);
    let bodylog_bin = resolve_bin(bodylog_override, &["bodylog"]);

    // Real last_pull from krebslog's optional cache (populated on successful pulls).
    // We show history even under --no-cache (the flag only forces live *data* calls).
    let last_nut = get_cached_last_pull(db_override, "nutlog");
    let last_rep = get_cached_last_pull(db_override, "repslog");
    let last_body = get_cached_last_pull(db_override, "bodylog");

    let mut sources: Vec<Value> = vec![];

    // nutlog
    let nut = if let Some(b) = &nutlog_bin {
        let mut info = serde_json::json!({
            "source": "nutlog",
            "bin": b,
            "available": true,
            "last_pull": last_nut,
            "entities": ["consumption", "purchase", "report:nutrition"]
        });
        if probe {
            // Do a trivial call to confirm it responds to --json
            match run_external_json(
                b,
                &[
                    "consumption".into(),
                    "list".into(),
                    "--since".into(),
                    "today".into(),
                ],
            ) {
                Ok((out, _)) => {
                    let count = if let Ok(v) = serde_json::from_str::<Value>(&out) {
                        if v.is_array() {
                            v.as_array().map(|a| a.len()).unwrap_or(0)
                        } else {
                            0
                        }
                    } else {
                        0
                    };
                    info["probe"] = serde_json::json!({ "ok": true, "sample_count": count });
                }
                Err(e) => {
                    info["probe"] = serde_json::json!({ "ok": false, "error": e.to_string() });
                }
            }
        }
        info
    } else {
        serde_json::json!({
            "source": "nutlog",
            "available": false,
            "error": "binary not found"
        })
    };
    sources.push(nut);

    // repslog
    let rep = if let Some(b) = &repslog_bin {
        let mut info = serde_json::json!({
            "source": "repslog",
            "bin": b,
            "available": true,
            "last_pull": last_rep,
            "entities": ["workout", "stats:summary", "stats:volume"]
        });
        if probe {
            match run_external_json(b, &["stats".into(), "summary".into()]) {
                Ok((out, _)) => {
                    info["probe"] = serde_json::json!({ "ok": true, "sample": out.lines().next().unwrap_or("").chars().take(80).collect::<String>() });
                }
                Err(e) => {
                    info["probe"] = serde_json::json!({ "ok": false, "error": e.to_string() });
                }
            }
        }
        info
    } else {
        serde_json::json!({
            "source": "repslog",
            "available": false,
            "error": "binary not found"
        })
    };
    sources.push(rep);

    // bodylog
    let body = if let Some(b) = &bodylog_bin {
        let mut info = serde_json::json!({
            "source": "bodylog",
            "bin": b,
            "available": true,
            "last_pull": last_body,
            "entities": ["measurement", "report:summary", "report:weight", "config"]
        });
        if probe {
            // Lightweight probe: config is always fast and requires no dates
            match run_external_json(b, &["config".into(), "show".into()]) {
                Ok((out, _)) => {
                    info["probe"] = serde_json::json!({ "ok": true, "sample": out.lines().next().unwrap_or("").chars().take(80).collect::<String>() });
                }
                Err(e) => {
                    info["probe"] = serde_json::json!({ "ok": false, "error": e.to_string() });
                }
            }
        }
        info
    } else {
        serde_json::json!({
            "source": "bodylog",
            "available": false,
            "error": "binary not found"
        })
    };
    sources.push(body);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "sources": sources,
                "derived": {
                    "krebs_status": "computed on demand from live pulls (body-validated flux/redox/energy/adaptation per spec/04)",
                    "cache_note": "full persistence of krebs + body aggregates when daily grain lands (Phase 5)"
                }
            }))
            .expect("serializing a freshly constructed in-memory Value cannot fail")
        );
    } else if !quiet {
        let mut table = Table::new();
        table.load_preset(presets::UTF8_FULL_CONDENSED);
        table.set_header(vec!["Source", "Available", "Binary", "Probe"]);
        for s in &sources {
            let src = s["source"].as_str().unwrap_or("?");
            let avail = s["available"].as_bool().unwrap_or(false);
            let bin = s["bin"].as_str().unwrap_or("-");
            let probe_str = if let Some(p) = s.get("probe") {
                if p["ok"].as_bool().unwrap_or(false) {
                    "OK".green().to_string()
                } else {
                    "FAIL".red().to_string()
                }
            } else {
                "-".to_string()
            };
            table.add_row(vec![
                Cell::new(src),
                Cell::new(if avail { "yes" } else { "no" }),
                Cell::new(bin),
                Cell::new(probe_str),
            ]);
        }
        println!("{}", table);
        if !probe {
            println!("(use --probe to attempt live connectivity checks)");
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_pull(
    source: Option<String>,
    entity: Option<String>,
    since: String,
    until: Option<String>,
    all: bool,
    period: Option<String>,
    dry_run: bool,
    ctx: &Context,
) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    let nutlog_override = ctx.nutlog_bin.as_deref();
    let repslog_override = ctx.repslog_bin.as_deref();
    let bodylog_override = ctx.bodylog_bin.as_deref();
    let no_cache = ctx.no_cache;
    let db_override = ctx.db.as_deref();

    // Resolve effective since/until
    let (since_eff, until_eff) = resolve_period(&since, until.as_deref(), period.as_deref())?;

    let since_str = format_date_for_child(since_eff);
    let until_str = until_eff.map(format_date_for_child);

    if all || source.is_none() {
        // Pull a curated set from all known tools
        if !quiet && !json {
            println!(
                "Pulling from nutlog + repslog + bodylog for {}..{}",
                since_str,
                until_str.as_deref().unwrap_or("now")
            );
        }
        pull_nutlog_default(
            &since_str,
            until_str.as_deref(),
            dry_run,
            json,
            quiet,
            nutlog_override,
            !no_cache,
            db_override,
        )?;
        pull_repslog_default(
            &since_str,
            until_str.as_deref(),
            dry_run,
            json,
            quiet,
            repslog_override,
            !no_cache,
            db_override,
        )?;
        pull_bodylog_default(
            &since_str,
            until_str.as_deref(),
            dry_run,
            json,
            quiet,
            bodylog_override,
            !no_cache,
            db_override,
        )?;
        if json {
            // Already emitted per-source success objects or arrays; emit a top level ack if nothing was printed.
            // In practice the per-source calls print JSON when json=true.
        }
        return Ok(());
    }

    let src = source
        .as_ref()
        .expect("source is Some: the all/none case was handled above")
        .to_lowercase();
    match src.as_str() {
        "nutlog" => {
            let ent = entity.as_deref().unwrap_or("consumption");
            pull_nutlog_entity(
                ent,
                &since_str,
                until_str.as_deref(),
                dry_run,
                json,
                quiet,
                nutlog_override,
                !no_cache,
                db_override,
            )
        }
        "repslog" => {
            let ent = entity.as_deref().unwrap_or("workout");
            pull_repslog_entity(
                ent,
                &since_str,
                until_str.as_deref(),
                dry_run,
                json,
                quiet,
                repslog_override,
                !no_cache,
                db_override,
            )
        }
        "bodylog" => {
            let ent = entity.as_deref().unwrap_or("measurement");
            pull_bodylog_entity(
                ent,
                &since_str,
                until_str.as_deref(),
                dry_run,
                json,
                quiet,
                bodylog_override,
                !no_cache,
                db_override,
            )
        }
        other => Err(KrebslogError::UnknownSource(other.to_string())),
    }
}

fn resolve_period(
    since: &str,
    until: Option<&str>,
    period: Option<&str>,
) -> Result<(chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>)> {
    // If --period given with --all, interpret period as "since = today - N, until = today"
    let since_dt = parse_flexible_date(since)?;

    let until_dt = if let Some(u) = until {
        Some(parse_flexible_date(u)?)
    } else if let Some(p) = period {
        // Support simple "90d", "30 days", "last 14 days" etc for convenience.
        // For now reuse the date parser; if it looks like "NNd" treat as N days.
        let p_lower = p.trim().to_lowercase();
        if let Some(days) = p_lower
            .strip_suffix('d')
            .and_then(|s| s.parse::<i64>().ok())
        {
            Some(since_dt + Duration::days(days))
        } else {
            Some(parse_flexible_date(p)?)
        }
    } else {
        None
    };

    Ok((since_dt, until_dt))
}

#[allow(clippy::too_many_arguments)]
fn pull_nutlog_default(
    since: &str,
    until: Option<&str>,
    dry_run: bool,
    json: bool,
    quiet: bool,
    override_bin: Option<&str>,
    cache_enabled: bool,
    db_override: Option<&str>,
) -> Result<()> {
    // Default entities we care about for metabolic picture:
    // - consumption (primary nutrition intake)
    // - (future) could also pull product nutrition or reports, but consumption is the grain we need
    pull_nutlog_entity(
        "consumption",
        since,
        until,
        dry_run,
        json,
        quiet,
        override_bin,
        cache_enabled,
        db_override,
    )?;
    // Optionally also surface a nutrition report for the window (very useful aggregate)
    pull_nutlog_entity(
        "report:nutrition",
        since,
        until,
        dry_run,
        json,
        quiet,
        override_bin,
        cache_enabled,
        db_override,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn pull_nutlog_entity(
    entity: &str,
    since: &str,
    until: Option<&str>,
    dry_run: bool,
    json: bool,
    quiet: bool,
    override_bin: Option<&str>,
    cache_enabled: bool,
    db_override: Option<&str>,
) -> Result<()> {
    let bin =
        resolve_bin(override_bin, &["nutlog"]).ok_or_else(|| KrebslogError::ExternalTool {
            bin: "nutlog".into(),
            reason: "not found in PATH or common locations".into(),
        })?;

    if dry_run {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "success": true,
                    "dry_run": true,
                    "source": "nutlog",
                    "entity": entity,
                    "since": since,
                    "until": until
                })
            );
        } else if !quiet {
            println!("(dry-run) would call: {} --json ... for {}", bin, entity);
        }
        return Ok(());
    }

    // Build args for the source tool.
    // We map our "entity" names to nutlog subcommands.
    let mut args: Vec<String> = vec![];
    let mut display_entity = entity.to_string();

    match entity {
        "consumption" | "consumptions" => {
            args.push("consumption".into());
            args.push("list".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "report:nutrition" | "nutrition-report" => {
            display_entity = "report nutrition".to_string();
            args.push("report".into());
            args.push("nutrition".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "purchase" | "purchases" => {
            args.push("purchase".into());
            args.push("list".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        other => {
            // Pass through — user can do advanced pulls; this keeps the tool open.
            // Example: entity "product list" is not a thing, but we let the user say --entity "product" "list" via future extension.
            // For skeleton we support the main ones and fall back to treating the entity as the subcommand path.
            // Simpler: error for unknown in v1 skeleton.
            return Err(KrebslogError::UnknownEntity {
                src: "nutlog".into(),
                entity: other.to_string(),
            });
        }
    }

    if !quiet && !json {
        println!(
            "Pulling {} from nutlog ({}..{})",
            display_entity,
            since,
            until.unwrap_or("today")
        );
    }

    let (stdout, _stderr) = run_external_json(&bin, &args)?;

    // Record pull for freshness in data status (bodylog also stores rows).
    if cache_enabled {
        if let Ok(v) = serde_json::from_str::<Value>(&stdout) {
            let count = v.as_array().map(|a| a.len() as i64);
            if let Ok(conn) = open_db(db_override) {
                let _ = record_pull(&conn, "nutlog", entity, since, until, count);
            }
        }
    }

    // Pass the JSON (or text) through. For agents this is the raw material.
    if stdout.trim().is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({ "success": true, "source": "nutlog", "entity": display_entity, "rows": 0 })
            );
        } else if !quiet {
            println!("(no data)");
        }
    } else {
        // Emit exactly what the child produced (already --json from child when we asked).
        println!("{}", stdout.trim_end());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn pull_repslog_default(
    since: &str,
    until: Option<&str>,
    dry_run: bool,
    json: bool,
    quiet: bool,
    override_bin: Option<&str>,
    cache_enabled: bool,
    db_override: Option<&str>,
) -> Result<()> {
    pull_repslog_entity(
        "workout",
        since,
        until,
        dry_run,
        json,
        quiet,
        override_bin,
        cache_enabled,
        db_override,
    )?;
    // Also pull a stats summary for the window (good aggregate)
    pull_repslog_entity(
        "stats:summary",
        since,
        until,
        dry_run,
        json,
        quiet,
        override_bin,
        cache_enabled,
        db_override,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn pull_repslog_entity(
    entity: &str,
    since: &str,
    until: Option<&str>,
    dry_run: bool,
    json: bool,
    quiet: bool,
    override_bin: Option<&str>,
    cache_enabled: bool,
    db_override: Option<&str>,
) -> Result<()> {
    let bin =
        resolve_bin(override_bin, &["repslog"]).ok_or_else(|| KrebslogError::ExternalTool {
            bin: "repslog".into(),
            reason: "not found in PATH or common locations".into(),
        })?;

    if dry_run {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "success": true,
                    "dry_run": true,
                    "source": "repslog",
                    "entity": entity,
                    "since": since,
                    "until": until
                })
            );
        } else if !quiet {
            println!("(dry-run) would call: {} --json ... for {}", bin, entity);
        }
        return Ok(());
    }

    let mut args: Vec<String> = vec![];
    let mut display_entity = entity.to_string();

    match entity {
        "workout" | "workouts" | "session" | "sessions" => {
            args.push("workout".into());
            args.push("list".into());
            // repslog uses --days for relative; for absolute dates we can use since/until if supported,
            // or fall back to computing days. For skeleton compute a conservative --days from since.
            let days = days_from_since(since);
            if let Some(d) = days {
                args.push("--days".into());
                args.push(d.to_string());
            }
            // Note: repslog workout list currently doesn't take calendar --since like nutlog.
            // We approximate with --days. More precise filtering can be done client-side later.
        }
        "stats:summary" | "summary" => {
            display_entity = "stats summary".to_string();
            args.push("stats".into());
            args.push("summary".into());
            let days = days_from_since(since).unwrap_or(30);
            args.push("--days".into());
            args.push(days.to_string());
        }
        "stats:volume" | "volume" => {
            display_entity = "stats volume".to_string();
            args.push("stats".into());
            args.push("volume".into());
            args.push("--period".into());
            // repslog volume accepts e.g. "30d"
            let p = if let Some(d) = days_from_since(since) {
                format!("{}d", d)
            } else {
                "30d".into()
            };
            args.push(p);
        }
        other => {
            return Err(KrebslogError::UnknownEntity {
                src: "repslog".into(),
                entity: other.to_string(),
            });
        }
    }

    if !quiet && !json {
        println!(
            "Pulling {} from repslog (approx {}..{})",
            display_entity,
            since,
            until.unwrap_or("recent")
        );
    }

    let (stdout, _stderr) = run_external_json(&bin, &args)?;

    if cache_enabled {
        if let Ok(v) = serde_json::from_str::<Value>(&stdout) {
            let count = v.as_array().map(|a| a.len() as i64);
            if let Ok(conn) = open_db(db_override) {
                let _ = record_pull(&conn, "repslog", entity, since, until, count);
            }
        }
    }

    if stdout.trim().is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({ "success": true, "source": "repslog", "entity": display_entity, "rows": 0 })
            );
        } else if !quiet {
            println!("(no data)");
        }
    } else {
        println!("{}", stdout.trim_end());
    }

    Ok(())
}

// ---------------- bodylog support ----------------

#[allow(clippy::too_many_arguments)]
fn pull_bodylog_default(
    since: &str,
    until: Option<&str>,
    dry_run: bool,
    json: bool,
    quiet: bool,
    override_bin: Option<&str>,
    cache_enabled: bool,
    db_override: Option<&str>,
) -> Result<()> {
    // Default entities for bodylog in a metabolic context:
    // - measurement list (raw daily measurements)
    // - report summary (convenient aggregates + trends for the window)
    pull_bodylog_entity(
        "measurement",
        since,
        until,
        dry_run,
        json,
        quiet,
        override_bin,
        cache_enabled,
        db_override,
    )?;
    pull_bodylog_entity(
        "report:summary",
        since,
        until,
        dry_run,
        json,
        quiet,
        override_bin,
        cache_enabled,
        db_override,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn pull_bodylog_entity(
    entity: &str,
    since: &str,
    until: Option<&str>,
    dry_run: bool,
    json: bool,
    quiet: bool,
    override_bin: Option<&str>,
    cache_enabled: bool,
    db_override: Option<&str>,
) -> Result<()> {
    let bin =
        resolve_bin(override_bin, &["bodylog"]).ok_or_else(|| KrebslogError::ExternalTool {
            bin: "bodylog".into(),
            reason: "not found in PATH or common locations".into(),
        })?;

    if dry_run {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "success": true,
                    "dry_run": true,
                    "source": "bodylog",
                    "entity": entity,
                    "since": since,
                    "until": until
                })
            );
        } else if !quiet {
            println!("(dry-run) would call: {} --json ... for {}", bin, entity);
        }
        return Ok(());
    }

    let mut args: Vec<String> = vec![];
    let mut display_entity = entity.to_string();

    match entity {
        "measurement" | "measurements" => {
            args.push("measurement".into());
            args.push("list".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "report:summary" | "summary" => {
            display_entity = "report summary".to_string();
            args.push("report".into());
            args.push("summary".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "report:weight" | "weight" => {
            display_entity = "report weight".to_string();
            args.push("report".into());
            args.push("weight".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "report:body-fat" | "body-fat" | "body_fat" => {
            display_entity = "report body-fat".to_string();
            args.push("report".into());
            args.push("body-fat".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "report:muscle" | "muscle" | "skeletal-muscle" => {
            display_entity = "report muscle".to_string();
            args.push("report".into());
            args.push("muscle".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "report:visceral-fat" | "visceral-fat" | "visceral_fat" => {
            display_entity = "report visceral-fat".to_string();
            args.push("report".into());
            args.push("visceral-fat".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "report:bmi" | "bmi" => {
            display_entity = "report bmi".to_string();
            args.push("report".into());
            args.push("bmi".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "report:resting-metabolism" | "resting-metabolism" | "resting_metabolism" => {
            display_entity = "report resting-metabolism".to_string();
            args.push("report".into());
            args.push("resting-metabolism".into());
            args.push("--since".into());
            args.push(since.into());
            if let Some(u) = until {
                args.push("--until".into());
                args.push(u.into());
            }
        }
        "config" | "profile" => {
            display_entity = "config".to_string();
            args.push("config".into());
            args.push("show".into());
            // config show takes no date args
        }
        other => {
            return Err(KrebslogError::UnknownEntity {
                src: "bodylog".into(),
                entity: other.to_string(),
            });
        }
    }

    if !quiet && !json {
        println!(
            "Pulling {} from bodylog ({}..{})",
            display_entity,
            since,
            until.unwrap_or("recent")
        );
    }

    let (stdout, _stderr) = run_external_json(&bin, &args)?;

    // Cache the result (if enabled) for body data and freshness.
    // We do this for measurement lists and for reports that can supply per-day series.
    if cache_enabled {
        if let Ok(v) = serde_json::from_str::<Value>(&stdout) {
            // record the pull
            record_pull_for_body(db_override, entity, since, until, &v);

            // Try to extract and store actual measurement rows for later use by reports/agent.
            let records = extract_body_records(&v);
            if !records.is_empty() {
                if let Ok(conn) = open_db(db_override) {
                    let _ = store_body_measurements(&conn, &records);
                }
            }
            // Also cache profile if this was a config pull
            if entity == "config" || entity == "profile" {
                if let Ok(conn) = open_db(db_override) {
                    let _ = crate::db::store_body_profile(&conn, &v);
                }
            }
        }
    }

    if stdout.trim().is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({ "success": true, "source": "bodylog", "entity": display_entity, "rows": 0 })
            );
        } else if !quiet {
            println!("(no data)");
        }
    } else {
        println!("{}", stdout.trim_end());
    }

    Ok(())
}

/// Record pull_log entry for bodylog pulls (and also attempt to count rows when obvious).
fn record_pull_for_body(
    db_override: Option<&str>,
    entity: &str,
    since: &str,
    until: Option<&str>,
    parsed: &Value,
) {
    let count = if let Some(arr) = parsed.as_array() {
        Some(arr.len() as i64)
    } else if let Some(series) = parsed.get("series").and_then(|s| s.as_array()) {
        Some(series.len() as i64)
    } else if let Some(w) = parsed.get("weight").or_else(|| parsed.get("body_fat")) {
        w.get("count").and_then(|c| c.as_i64())
    } else {
        None
    };
    if let Ok(conn) = open_db(db_override) {
        let _ = record_pull(&conn, "bodylog", entity, since, until, count);
    }
}

/// Extract an array of measurement-like records from either:
/// - a direct array (bodylog measurement list)
/// - a report object containing a "series" array
/// - a summary that has nested per-metric objects (best effort, may be sparse)
fn extract_body_records(v: &Value) -> Vec<Value> {
    if let Some(arr) = v.as_array() {
        return arr.clone();
    }
    if let Some(series) = v.get("series").and_then(|s| s.as_array()) {
        return series.clone();
    }
    // For summary shapes, we don't have per-day here; the caller will have also pulled "measurement".
    // Still, surface the top-level weight etc stats as a single pseudo-record for last_pull only.
    vec![]
}

fn days_from_since(since: &str) -> Option<i64> {
    // Best effort: if since is a concrete date, compute days back from today.
    if let Ok(dt) = parse_flexible_date(since) {
        let now = Utc::now();
        let delta = now.signed_duration_since(dt);
        let days = (delta.num_days() + 1).max(1);
        return Some(days);
    }
    // crude parse of "last N days"
    if let Some(rest) = since
        .strip_prefix("last ")
        .and_then(|s| s.strip_suffix(" days"))
    {
        if let Ok(n) = rest.trim().parse::<i64>() {
            return Some(n);
        }
    }
    None
}

/// Best-effort last successful pull timestamp for a source from the optional cache.
/// Returns RFC3339 string or null (as JSON null via Option in caller sites).
fn get_cached_last_pull(db_override: Option<&str>, source: &str) -> Option<String> {
    // We deliberately ignore ctx.no_cache here: last_pull is historical metadata.
    match open_db(db_override) {
        Ok(conn) => crate::db::get_last_pull_for_source(&conn, source).unwrap_or(None),
        Err(_) => None,
    }
}
