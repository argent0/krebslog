# krebslog Documentation

`krebslog` is a simple, local-first, single-user CLI tool that turns data from your `nutlog` and `repslog` (and optionally future `bodylog`) tools into rich metabolic reports centered on the Krebs (TCA) cycle, energy flux, training load, nutrition inputs, antioxidant/redox balance, and adaptation outcomes.

It is designed to be **LLM-agent friendly** (via `--json`, consistent command groups, and predictable shapes) while remaining directly usable by humans. All source data is obtained exclusively by calling the other CLIs with `--json` — krebslog never reads their SQLite databases directly.

- **Project home**: see top-level [README.md](../README.md)
- **For agents and contributors**: [AGENTS.md](../AGENTS.md) and [CODING_PRACTICES.md](../CODING_PRACTICES.md)
- **Specification & roadmap**: [spec/01-spec.md](../spec/01-spec.md)

## Documentation Index

- [Installation](installation.md) — Cargo, packaging (PKGBUILD), dependencies on nutlog + repslog.
- [Getting Started](getting-started.md) — First steps, data pulling, basic reports, environment.
- [Command Reference](command-reference.md) — Exhaustive reference for every command, subcommand, flag, and return shape.
- [Data Model](data-model.md) — Daily aggregates, nutrition + training inputs, derived krebs flux / redox scores, optional SQLite cache.
- [Data Ingestion / Pulling](pulling.md) — How `data pull` and `data status` work by invoking sibling tools (the only supported ingestion path).
- [Reporting](reporting.md) — The core metabolic reports (daily, krebs-flux, redox-balance, energy-balance, correlations).
- [Image Generation](image-generation.md) — Krebs cycle diagrams, heatmaps, antioxidant shield, full dashboards (plotters + custom SVG).
- [Telegram Integration](telegram-integration.md) — Producing ready-to-post MarkdownV2 captions + image paths.
- [Agent & JSON Usage](agent-usage.md) — Primary interface for Hermes-style agents and scripts.
- [Troubleshooting](troubleshooting.md) — Common issues, date parsing, child tool problems, debugging.

## Quick Command Structure

```
krebslog [GLOBAL FLAGS] <GROUP> <ACTION> [options]
```

Global flags (apply to every command):

- `--json` — machine-readable JSON (the primary interface for agents and scripts)
- `--quiet` — minimal human output
- `--db /path/to/krebslog.db` — path to krebslog's own optional cache DB (XDG default: `~/.local/share/krebslog/krebslog.db`)
- `--no-cache` — disable the local cache completely (always call nutlog/repslog live)
- `--nutlog-bin PATH` (also `NUTLOG_BIN` env)
- `--repslog-bin PATH` (also `REPSLOG_BIN` env)
- `--help`, `--version`

Main groups:

- `data` — pull & inspect data from nutlog / repslog (the ingestion layer)
- `report` — metabolic reports and derived insights
- `image` — high-quality visual renders (Krebs cycle, heatmaps, dashboards)
- `telegram` — Telegram-optimized output (MarkdownV2 + image paths)
- `agent` — Hermes / LLM agent context bundles, prompt tuning, skill export
- `config` — persistent settings (binary paths, formulas, output dirs, themes)
- `migrate` — manage krebslog's optional cache schema
- `cache` — inspect / clear the local cache

Mutations and high-level operations return a consistent success envelope under `--json`:

```json
{ "success": true, "id"?: number, "message": "..." }
```

Errors (non-zero exit, printed to stdout for easy capture):

```json
{ "success": false, "error": "..." }
```

## Key Design Principles (from the spec)

- **Local-first, offline, single-user, no cloud.**
- **CLI is primary.** `--json` is the reliable contract for LLM agents.
- **Zero direct DB access** to source tools. Everything comes from live `nutlog --json ...` / `repslog --json ...` subprocess calls (or cached results of prior calls).
- Flexible natural-language dates everywhere (`today`, `last 7 days`, `last monday`, `2026-06-07`, ...).
- Minimal dependencies. Image generation (plotters + custom SVG) lives behind a feature flag.
- Transparent derived values: Krebs flux proxies, redox/oxidative load, energy balance, etc. always include the formulas/assumptions in JSON.
- First-class support for visual reports (Telegram, Hermes agent tuning).
- Clear separation of concerns: data pulling layer, aggregation engine, report generators, image renderers, output formatters.

## Version, Help, and Discovery

```bash
krebslog --version
krebslog --help
krebslog data --help
krebslog data pull --help
krebslog report --help
krebslog --json data status --probe
```

The `--help` text (powered by clap) is authoritative for exact syntax and current flag names.

## Next Steps

- [Getting Started](getting-started.md) — pull some data and run your first report today.
- [Command Reference](command-reference.md) — every detail.
- [Data Model](data-model.md) — what krebslog actually stores and derives.
- [Agent & JSON Usage](agent-usage.md) — how Hermes and other agents are expected to drive krebslog.

---

*This documentation is intended to be installed alongside the binary (e.g. under `/usr/share/doc/krebslog/docs/`) for offline reference, matching the pattern used by nutlog, repslog, and bodylog.*

