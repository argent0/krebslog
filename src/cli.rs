use clap::{Parser, Subcommand};

/// krebslog — Metabolic reports, Krebs cycle visualizer & redox insights.
///
/// CLI-first • Local-only • LLM-agent primary interface • Image-capable reports.
#[derive(Parser, Debug)]
#[command(name = "krebslog", version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Output structured JSON instead of human-readable text (primary interface for agents).
    #[arg(long, global = true)]
    pub json: bool,

    /// Override default SQLite cache location (XDG data dir ~/.local/share/krebslog/krebslog.db).
    #[arg(long, global = true, value_name = "PATH")]
    pub db: Option<String>,

    /// Minimal output (useful for scripting/LLMs).
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Disable the local SQLite cache entirely (always call source tools live).
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Path to the nutlog binary (default: search PATH + common locations).
    #[arg(long, global = true, value_name = "PATH", env = "NUTLOG_BIN")]
    pub nutlog_bin: Option<String>,

    /// Path to the repslog binary (default: search PATH + common locations).
    #[arg(long, global = true, value_name = "PATH", env = "REPSLOG_BIN")]
    pub repslog_bin: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pull data from source tools (nutlog, repslog) and optionally cache it.
    Data {
        #[command(subcommand)]
        action: DataAction,
    },

    /// Generate metabolic reports (daily, weekly, flux, redox, correlations, etc).
    Report {
        #[command(subcommand)]
        action: ReportAction,
    },

    /// Generate publication-quality images (Krebs cycle, heatmaps, dashboards).
    Image {
        #[command(subcommand)]
        action: ImageAction,
    },

    /// Produce Telegram-ready MarkdownV2 captions + image paths.
    Telegram {
        #[command(subcommand)]
        action: TelegramAction,
    },

    /// Hermes agent support: context bundles, prompt tuning, skill export.
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// Configuration (paths, formulas, output locations, themes).
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Database / cache maintenance.
    Migrate {
        /// Show current vs latest schema (no changes).
        #[arg(short, long)]
        status: bool,
        /// Show what would be applied (no changes).
        #[arg(short, long)]
        dry_run: bool,
        /// Force re-apply even if already at latest.
        #[arg(short, long)]
        force: bool,
    },

    /// Cache management utilities.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

// ---------------- Data ----------------

#[derive(Subcommand, Debug)]
pub enum DataAction {
    /// Pull / refresh data from a source tool via its --json interface.
    Pull {
        /// Source tool: nutlog or repslog
        #[arg(long, value_parser = ["nutlog", "repslog"])]
        source: Option<String>,

        /// Entity to pull (e.g. consumption, purchase, workout, set, stats).
        /// When omitted with --source, pulls a sensible default set for that source.
        #[arg(long)]
        entity: Option<String>,

        /// Start of period (flexible: today, yesterday, last 7 days, 2026-05-20, last monday, ...)
        #[arg(long, default_value = "last 30 days")]
        since: String,

        /// End of period (inclusive). Defaults to today.
        #[arg(long)]
        until: Option<String>,

        /// Convenient shortcut: pull a balanced set from all known sources for the period.
        #[arg(long)]
        all: bool,

        /// Period alias for --all (e.g. 90d, 30 days, last 14 days). Parsed into since/until.
        #[arg(long)]
        period: Option<String>,

        /// Show exactly what would be pulled (no writes, no cache updates).
        #[arg(long)]
        dry_run: bool,
    },

    /// Show last successful pull times and basic freshness per source/entity.
    Status {
        /// Also attempt a lightweight live probe of the source tools.
        #[arg(long)]
        probe: bool,
    },
}

// ---------------- Report ----------------

#[derive(Subcommand, Debug)]
pub enum ReportAction {
    /// Daily metabolic snapshot (nutrition + training load + derived krebs/redox proxies).
    Daily {
        /// Target date (flexible syntax).
        #[arg(long, default_value = "today")]
        date: String,

        /// Include a generated image path in output (PNG by default).
        #[arg(long)]
        include_image: bool,

        /// Output directory for any sidecar images (default: ~/reports/krebslog or XDG).
        #[arg(long)]
        output_dir: Option<String>,
    },

    /// Weekly or multi-day aggregate report.
    Weekly {
        #[arg(long, default_value = "last monday")]
        since: String,
        #[arg(long)]
        until: Option<String>,
    },

    /// Krebs (TCA) cycle flux proxy report for a period.
    #[command(name = "krebs-flux")]
    KrebsFlux {
        #[arg(long, default_value = "last 14 days")]
        period: String,
        /// Write an image instead of (or in addition to) text/JSON.
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Antioxidant vs. training oxidative load balance.
    #[command(name = "redox-balance")]
    RedoxBalance {
        #[arg(long, default_value = "last 7 days")]
        since: String,
        #[arg(long)]
        until: Option<String>,
    },

    /// Energy balance (intake vs. estimated expenditure) + optional body trends.
    #[command(name = "energy-balance")]
    EnergyBalance {
        #[arg(long, default_value = "last 14 days")]
        since: String,
        #[arg(long)]
        until: Option<String>,

        /// When bodylog is available, include weight/trend overlays.
        #[arg(long)]
        include_body_trends: bool,
    },

    /// Correlations between nutrition inputs and training outputs.
    Correlations {
        #[arg(long, default_value = "nutrition")]
        x: String,
        #[arg(long, default_value = "training-load")]
        y: String,
        #[arg(long, default_value = "last 30 days")]
        period: String,
    },
}

// ---------------- Image ----------------

#[derive(Subcommand, Debug)]
pub enum ImageAction {
    /// Render a stylized Krebs/TCA cycle diagram with flux overlays from your data.
    #[command(name = "krebs-cycle")]
    KrebsCycle {
        #[arg(long, default_value = "last 30 days")]
        date_range: String,
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
        #[arg(long, value_parser = ["light", "dark"], default_value = "dark")]
        theme: String,
    },

    /// Training load calendar-style heatmap.
    #[command(name = "training-load-heatmap")]
    TrainingLoadHeatmap {
        #[arg(long, default_value = "last 90 days")]
        period: String,
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Antioxidant / redox "shield" visualization.
    #[command(name = "antioxidant-shield")]
    AntioxidantShield {
        #[arg(long, value_parser = ["light", "dark"], default_value = "dark")]
        style: String,
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Composite dashboard (cards + cycle + trends) — Telegram friendly.
    #[command(name = "full-dashboard")]
    FullDashboard {
        #[arg(long, default_value = "today")]
        date: String,
        #[arg(long, value_parser = ["telegram", "square", "wide"], default_value = "telegram")]
        layout: String,
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },
}

// ---------------- Telegram ----------------

#[derive(Subcommand, Debug)]
pub enum TelegramAction {
    /// Daily post bundle: MarkdownV2 caption + image path (ready for bot).
    Daily {
        #[arg(long, default_value = "today")]
        date: String,
        /// Also emit a machine-readable JSON block after the caption.
        #[arg(long)]
        with_json: bool,
    },

    /// Generic report exporter tuned for Telegram (caption + image).
    Report {
        #[arg(long, value_parser = ["daily", "redox", "krebs", "energy"])]
        r#type: String,
        #[arg(long, default_value = "last 7 days")]
        period: String,
        /// Print only the final post-ready payload (no extra logs).
        #[arg(long)]
        post_ready: bool,
    },
}

// ---------------- Agent (Hermes) ----------------

#[derive(Subcommand, Debug)]
pub enum AgentAction {
    /// Emit a compact, up-to-date context bundle suitable for injection into an agent prompt or RAG.
    Context {
        #[arg(long, default_value = "hermes")]
        r#for: String,
        #[arg(long, default_value = "last 30 days")]
        since: String,
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Generate or refine prompt templates / few-shot examples focused on metabolic interpretation.
    Tune {
        #[arg(long)]
        generate_prompts: bool,
        #[arg(long)]
        focus: Option<String>,
    },

    /// Export skill packages (AGENTS.md fragments, command examples) for the Hermes agent.
    #[command(name = "skills")]
    Skills {
        #[command(subcommand)]
        action: AgentSkillsAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum AgentSkillsAction {
    /// Copy skill definition files into the agent's skills directory.
    Export {
        #[arg(long, value_name = "DIR")]
        to: Option<String>,
        /// Overwrite existing files.
        #[arg(long)]
        force: bool,
    },
}

// ---------------- Config ----------------

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Get or list configuration values.
    Get {
        /// Key (e.g. nutlog.path, reports.dir, formulas.redox.weight_training)
        key: Option<String>,
    },
    /// Set a configuration value (persisted in the krebslog DB or config file).
    Set { key: String, value: String },
    /// Show the resolved effective configuration.
    Show,
    /// Reset to built-in defaults (does not delete your data).
    Reset {
        #[arg(long)]
        force: bool,
    },
}

// ---------------- Cache ----------------

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Clear all cached data (does not affect source tool databases).
    Clear {
        /// Confirm without interactive prompt.
        #[arg(long)]
        force: bool,
    },
    /// Show cache location, size, and last write times.
    Info,
}
