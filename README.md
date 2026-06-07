# krebslog

**Metabolic reports, Krebs cycle visualizer & redox insights.**

CLI-first • Local-only • LLM-agent primary interface • Image-capable reports for Telegram & Hermes agent tuning.

## Philosophy (exactly like nutlog)

- Local-first, offline, single-user, no cloud.
- CLI is the primary interface. `--json` is the reliable machine interface (designed for LLM agents first).
- Pull all input data exclusively by calling the other CLIs (`nutlog --json ...`, `repslog --json ...`). **Zero direct database access** to the source tools.
- Flexible natural-language dates (`today`, `last 7 days`, `last monday`, `2026-05-20`, ...).
- Minimal dependencies. Predictable behavior.
- Packaged for Arch (PKGBUILD) and `cargo install`.
- Built for LLM agents (rich `AGENTS.md`, skill packages, consistent JSON success/error shapes).

## Quick start

```bash
# See what data is available
krebslog data status --probe

# Pull recent nutrition + training data (live calls to nutlog + repslog)
krebslog --json data pull --all --period 30d

# Daily report (text + JSON)
krebslog --json report daily --date today

# Status of the optional krebslog cache
krebslog cache info
```

## Global flags

- `--json` — machine output (the default for agents and scripts)
- `--quiet`
- `--db /path/to/krebslog.db`
- `--no-cache` — disable local cache completely (always live)
- `--nutlog-bin /path`
- `--repslog-bin /path`

## Command groups (see `--help` for details)

- `data pull` / `data status`
- `report daily|weekly|krebs-flux|redox-balance|energy-balance|correlations`
- `image krebs-cycle|training-load-heatmap|antioxidant-shield|full-dashboard`
- `telegram daily|report`
- `agent context|tune|skills export`
- `config get|set|show|reset`
- `migrate`, `cache clear|info`

## Data model (internal)

See `docs/data-model.md` (to be expanded with caching).

krebslog maintains an optional lightweight local cache (SQLite) of daily aggregates derived from live CLI pulls:

- Date grain
- Nutrition totals + derived acetyl-CoA / antioxidant proxies (from nutlog consumption + product nutrition)
- Training load, volume, effective reps, HR zones, energy system hints (from repslog)
- Derived: krebs flux proxy, redox/oxidative balance, energy balance indicators

All derived scores are transparent in JSON output.

## Image generation

Powered by pure-Rust `plotters` (behind a future feature flag) + a custom SVG Krebs cycle renderer. Images go to `~/reports/krebslog/` (configurable).

## Development

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test

# Full verification (when docs examples exist)
cargo build && ./docs/verify_examples.sh || true
```

## License

MIT (matching the ecosystem of nutlog / repslog).
