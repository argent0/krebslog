use crate::cli::{AgentAction, AgentSkillsAction};
use crate::context::Context;
use crate::error::Result;

pub fn handle_agent(action: AgentAction, ctx: &Context) -> Result<()> {
    let json = ctx.json;
    let quiet = ctx.quiet;
    match action {
        AgentAction::Context {
            r#for,
            since,
            output,
        } => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "command": "agent context",
                        "for": r#for,
                        "since": since,
                        "output": output,
                        "note": "skeleton — will emit nutrition + training + redox + insights bundle"
                    })
                );
            } else if !quiet {
                println!("agent context --for {} --since {} (skeleton)", r#for, since);
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
