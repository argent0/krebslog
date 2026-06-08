/// Runtime context derived from global CLI flags.
/// Passed down to command handlers so we don't repeat the same 6–7 parameters everywhere.
/// This also makes future injection (e.g. a cache handle, config overrides) easier
/// while keeping the surface small.
#[derive(Clone, Debug, Default)]
pub struct Context {
    pub json: bool,
    pub quiet: bool,
    #[allow(dead_code)]
    pub no_cache: bool,
    pub db: Option<String>,
    pub nutlog_bin: Option<String>,
    pub repslog_bin: Option<String>,
    pub bodylog_bin: Option<String>,
}

impl Context {
    /// Build a context from the parsed top-level CLI (after clap).
    pub fn from_cli(cli: &crate::cli::Cli) -> Self {
        Self {
            json: cli.json,
            quiet: cli.quiet,
            no_cache: cli.no_cache,
            db: cli.db.clone(),
            nutlog_bin: cli.nutlog_bin.clone(),
            repslog_bin: cli.repslog_bin.clone(),
            bodylog_bin: cli.bodylog_bin.clone(),
        }
    }
}
