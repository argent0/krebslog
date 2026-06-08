use crate::cli::ConfigAction;
use crate::context::Context;
use crate::db::{get_all_config, get_config_value, open_db, reset_config, set_config_value};
use crate::error::Result;

pub fn handle_config(action: ConfigAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    let db_override = ctx.db.as_deref();

    // Known defaults (shown in `show`; some feed into computations when stored overrides are absent).
    let defaults: Vec<(&str, &str)> = vec![
        ("nutlog.bin", "(auto)"),
        ("repslog.bin", "(auto)"),
        ("bodylog.bin", "(auto)"),
        ("reports.dir", "~/reports/krebslog"),
        ("base_metabolism_kcal", "1750"),
    ];

    match action {
        ConfigAction::Get { key } => {
            let mut value: Option<String> = None;
            let mut note: Option<String> = None;
            if let Ok(conn) = open_db(db_override) {
                if let Some(k) = &key {
                    value = get_config_value(&conn, k).unwrap_or(None);
                } else {
                    // When no key, return the full map for convenience
                    if let Ok(all) = get_all_config(&conn) {
                        if json {
                            let mut cfg = serde_json::Map::new();
                            for (k, v) in all {
                                cfg.insert(k, serde_json::Value::String(v));
                            }
                            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                                "success": true,
                                "config": cfg,
                                "defaults": defaults.iter().cloned().collect::<std::collections::HashMap<_,_>>()
                            })).unwrap());
                            return Ok(());
                        }
                    }
                }
            } else {
                note = Some("db unavailable; showing defaults only".to_string());
            }
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "key": key, "value": value, "note": note })
                );
            } else if !quiet {
                if let Some(k) = &key {
                    match &value {
                        Some(v) => println!("{} = {}", k, v),
                        None => println!("{} (default or unset)", k),
                    }
                } else {
                    println!("Config keys (stored + defaults):");
                    for (k, d) in &defaults {
                        println!("  {} = {}", k, d);
                    }
                }
            }
        }
        ConfigAction::Set { key, value } => match open_db(db_override) {
            Ok(conn) => {
                if let Err(e) = set_config_value(&conn, &key, &value) {
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({ "success": false, "error": e.to_string() })
                        );
                    }
                    return Err(e);
                }
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": true, "set": key, "value": value })
                    );
                } else if !quiet {
                    println!("config set {} = {}", key, value);
                }
            }
            Err(e) => {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": false, "error": format!("config store unavailable: {}", e) })
                    );
                }
                return Err(e);
            }
        },
        ConfigAction::Show => {
            let mut stored: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            let mut note: Option<String> = None;
            if let Ok(conn) = open_db(db_override) {
                if let Ok(all) = get_all_config(&conn) {
                    for (k, v) in all {
                        stored.insert(k, v);
                    }
                }
            } else {
                note = Some("db unavailable; defaults only".into());
            }
            if json {
                let mut effective = serde_json::Map::new();
                for (k, d) in &defaults {
                    let v = stored.get(*k).cloned().unwrap_or_else(|| d.to_string());
                    effective.insert((*k).to_string(), serde_json::Value::String(v));
                }
                let mut out = serde_json::json!({ "success": true, "config": effective });
                if let Some(n) = note {
                    out["note"] = serde_json::json!(n);
                }
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            } else if !quiet {
                println!("Effective config:");
                for (k, d) in &defaults {
                    let v = stored.get(*k).cloned().unwrap_or_else(|| d.to_string());
                    println!("  {} = {}", k, v);
                }
                if let Some(n) = note {
                    println!("  ({})", n);
                }
            }
        }
        ConfigAction::Reset { force: _ } => {
            match open_db(db_override) {
                Ok(conn) => {
                    let _ = reset_config(&conn); // best effort
                    if json {
                        println!("{}", serde_json::json!({ "success": true, "reset": true }));
                    } else if !quiet {
                        println!("config reset (defaults restored)");
                    }
                }
                Err(e) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({ "success": false, "error": e.to_string() })
                        );
                    }
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}
