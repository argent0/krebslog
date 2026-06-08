use crate::cli::{AgentAction, AgentSkillsAction};
use crate::commands::report::build_compact_krebs_status_for_agent;
use crate::context::Context;
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
            let mut body_latest: Option<serde_json::Value> = None;

            if body_available {
                if let Some(bin) = &bodylog_bin {
                    // Best-effort latest measurement or weight summary for the agent bundle
                    if let Ok((out, _)) = run_external_json(
                        bin,
                        &[
                            "measurement".into(),
                            "list".into(),
                            "--since".into(),
                            "today".into(),
                        ],
                    ) {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out) {
                            if let Some(arr) = v.as_array() {
                                if let Some(first) = arr.first() {
                                    body_latest = Some(first.clone());
                                }
                            }
                        }
                    }
                }
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
                if let Some(b) = body_latest {
                    payload["body"] = serde_json::json!({ "latest": b });
                } else if body_available {
                    payload["body"] = serde_json::json!({ "available": true, "note": "no recent measurement for 'today'" });
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
