mod cli;
mod commands;
mod context;
mod error;
mod utils;

use clap::Parser;
use colored::*;

use crate::cli::{Cli as AppCli, Commands as AppCommands};
use crate::context::Context;
use anyhow::Result as AnyhowResult;

fn main() {
    let cli = AppCli::parse();
    if let Err(e) = run(cli) {
        // Always surface errors to stderr. For --json callers the individual command
        // handlers are responsible for emitting the {"success":false,...} shape before
        // returning the error (so we can still exit 1).
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run(cli: AppCli) -> AnyhowResult<()> {
    let ctx = Context::from_cli(&cli);

    match cli.command {
        AppCommands::Data { action } => {
            commands::data::handle_data(action, &ctx)?;
        }
        AppCommands::Report { action } => {
            commands::report::handle_report(action, &ctx)?;
        }
        AppCommands::Image { action } => {
            commands::image::handle_image(action, &ctx)?;
        }
        AppCommands::Telegram { action } => {
            commands::telegram::handle_telegram(action, &ctx)?;
        }
        AppCommands::Agent { action } => {
            commands::agent::handle_agent(action, &ctx)?;
        }
        AppCommands::Config { action } => {
            commands::config::handle_config(action, &ctx)?;
        }
        AppCommands::Migrate {
            status,
            dry_run,
            force,
        } => {
            commands::migrate::handle_migrate(status, dry_run, force, &ctx)?;
        }
        AppCommands::Cache { action } => {
            commands::cache::handle_cache(action, &ctx)?;
        }
    }
    Ok(())
}
