# Command Reference

This is the detailed reference for every command, subcommand, flag, and return shape in `krebslog`.

**The live `--help` output is the single source of truth for exact current syntax.** This document adds explanations, examples (human + `--json`), success/error shapes, agent notes, and cross-references.

All commands accept the global flags listed below.

## Global Flags

| Flag              | Description                                                                 | Agent Notes                                      |
|-------------------|-----------------------------------------------------------------------------|--------------------------------------------------|
| `--json`          | Emit structured, pretty-printed JSON instead of human tables/text           | Use this for almost everything                   |
| `--quiet`         | Suppress non-essential progress / success messages                          | Good when you only want the JSON payload         |
| `--db PATH`       | Full path to krebslog's own optional cache DB                               | Highly recommended for scripts and isolation     |
| `--no-cache`      | Disable the local cache; always call nutlog/repslog live                    | Use when you need guaranteed fresh data          |
| `--nutlog-bin`    | Path to the nutlog binary (overrides PATH + fallbacks)                      | Or set `NUTLOG_BIN` env var                      |
| `--repslog-bin`   | Path to the repslog binary                                                  | Or set `REPSLOG_BIN` env var                     |
| `--bodylog-bin`   | Path to the bodylog binary (overrides PATH + fallbacks)                     | Or set `BODYLOG_BIN` env var                     |
| `--help` / `-h`   | Show help for the command or subcommand                                     | -                                                |
| `--version` / `-V`| Show version                                                                | -                                                |

## Command Groups Overview

| Group     | Purpose                                                              | Key Sub-actions                                      |
|-----------|----------------------------------------------------------------------|------------------------------------------------------|
| `data`    | Ingest data by calling nutlog/repslog via `--json`                   | `pull`, `status`                                     |
| `report`  | Metabolic reports and derived scores                                 | `daily`, `weekly`, `krebs-flux`, `redox-balance`, `energy-balance`, `correlations` |
| `image`   | Publication-quality visuals (Krebs cycle, heatmaps, dashboards)      | `krebs-cycle`, `training-load-heatmap`, `antioxidant-shield`, `full-dashboard` |
| `telegram`| Produce ready-to-post MarkdownV2 captions + image paths              | `daily`, `report`                                    |
| `agent`   | Context bundles, prompt tuning, and skill export for LLM agents      | `context`, `tune`, `skills export`                   |
| `config`  | Persistent settings (paths, formulas, output locations, themes)      | `get`, `set`, `show`, `reset`                        |
| `migrate` | Manage krebslog's optional cache schema                              | (flags: `--status`, `--dry-run`, `--force`)          |
| `cache`   | Inspect or clear the local cache                                     | `clear`, `info`                                      |

## data

### data pull

```bash
krebslog data pull --source nutlog --entity consumption --since "last 30 days" [--until DATE] [--dry-run]
krebslog data pull --source repslog --entity workout --since "last 14 days"
krebslog data pull --all --period 90d [--dry-run]
```

- `--source` — `nutlog`, `repslog`, or `bodylog`
- `--entity` — what to pull from that source (e.g. `consumption`, `workout`, `stats:summary`, `report:nutrition`). When omitted with a single `--source`, a sensible default for that tool is used.
- `--all` — pull a curated default set from both tools (most common form for agents).
- `--period` — convenience window (works with `--all`).
- `--dry-run` — print what would be executed without calling the children or writing cache.

Under `--json` the raw output from the child tool(s) is printed (or lightly wrapped). This gives agents both the high-level pull acknowledgment and the detailed child data in one go.

See [pulling.md](pulling.md) for the full ingestion story.

### data status

```bash
krebslog data status
krebslog data status --probe
krebslog --json data status --probe
```

Shows last successful pull times per source/entity (when the cache is populated) and basic availability.

`--probe` executes lightweight live calls to verify that the binaries can be found and respond to `--json`.

## report

(See the dedicated [reporting.md](reporting.md) for concepts and examples. The command surface is defined as follows.)

### report daily

```bash
krebslog report daily --date today [--include-image] [--output-dir DIR]
krebslog --json report daily --date "2026-06-05" --include-image
```

Daily nutrition + training + derived krebs/redox/energy snapshot.

### report weekly

```bash
krebslog report weekly --since "last monday" [--until today]
```

Multi-day aggregate + trends.

### report krebs-flux

```bash
krebslog report krebs-flux --period 14d [--output ./reports/flux.png]
```

Focus on estimated TCA cycle flux with formula transparency in JSON.

### report redox-balance

```bash
krebslog report redox-balance --since "last 7 days" [--until ...]
```

Training oxidative load vs. dietary antioxidant support.

### report energy-balance

```bash
krebslog report energy-balance --since "last 14 days" [--include-body-trends]
```

Intake vs. expenditure. With `--include-body-trends`, krebslog pulls weight / body comp from bodylog (if available) and includes structured `body` stats (start/end/delta/trend/series) in the JSON output plus a `body_validation` block with qualitative label and discrepancy severity. See spec/03-bodylog.md and spec/04-krebs-status.md.

### report krebs-status

```bash
krebslog report krebs-status --since "last 14 days" [--until DATE] [--include-raw]
krebslog report krebs-status --date today
```

Body-enhanced Krebs cycle status report. Flux (with components + formula), redox, energy + body_validation, body_adaptation (latest + trends + implication), insights, assumptions, and caveats. All derived values are transparent. `--include-raw` embeds the underlying nutlog/repslog/bodylog payloads. Single-day snapshots via `--date`. See spec/04-krebs-status.md for the full JSON contract and human layout.

### report correlations

```bash
krebslog report correlations --x nutrition --y training-load --period 30d
```

Exploratory relationships between inputs and outputs.

### report web

```bash
krebslog report web --period "last 30 days" --output ~/reports/krebslog/
krebslog report web --since "last 7 days" --single-file --output ./krebs-today.html --theme dark
```

Generates a fully self-contained, mobile-first interactive HTML report (Tailwind via CDN, zero-build vanilla JS).

- Hero element: clickable SVG Krebs cycle diagram with color + thickness encoding of your personal flux per step.
- Bottom sheet on tap with per-step contribution, explanation, and calculation notes.
- KPI cards, Inputs/Outputs/Redox sections, sortable metrics table, trends sparkline, full methodology + assumptions.
- `--json` returns success metadata + paths (the HTML body is never printed under JSON).
- Output: single-file `.html` (with embedded data + adjacent `.data.json`) by default, or `--folder` for `index.html` + `data.json`.
- Works offline after generation (open directly or serve with `python -m http.server` / darkhttpd).
- All derived values (flux proxy, redox, etc.) link back to transparent formulas in the page and in `data.json`.

## image

Image commands are fully declared in the CLI (see [image-generation.md](image-generation.md) for vision and status).

```bash
krebslog image krebs-cycle --date-range "last 30 days" --theme dark --output krebs-cycle.png
krebslog image training-load-heatmap --period 90d
krebslog image antioxidant-shield --style dark --output redox.png
krebslog image full-dashboard --date today --layout telegram --output dashboard.png
```

## telegram

See [telegram-integration.md](telegram-integration.md).

```bash
krebslog telegram daily --date today [--with-json]
krebslog telegram report --type redox --period 7d --post-ready
```

## agent

See [agent-usage.md](agent-usage.md) and the top-level [AGENTS.md](../AGENTS.md).

```bash
krebslog agent context --for hermes --since "last 30 days" [--output path.json]
krebslog agent tune --generate-prompts --focus "redox and recovery"
krebslog agent skills export [--to DIR] [--force]
```

## config

```bash
krebslog config show
krebslog config get reports.dir
krebslog config set nutlog.path /custom/nutlog
krebslog config reset --force
```

(Implementation is still minimal in the skeleton; the surface is defined for future use.)

## migrate

```bash
krebslog migrate --status
krebslog migrate --dry-run
krebslog migrate --force
```

Manages schema for krebslog's own optional cache (not the source tools). Currently a no-op stub that follows the same pattern as nutlog/repslog.

## cache

```bash
krebslog cache info
krebslog cache clear --force
```

Inspect size/location or wipe the local krebslog cache. Source data in nutlog/repslog is never touched.

## Success & Error Shapes (Applies to Most Commands)

**Success (mutations / high-level ops):**

```json
{ "success": true, "id"?: number, "message": "..." }
```

**Data responses** (lists, reports, status, context, etc.) are usually the natural rich object or array for that command.

**Error (stdout + non-zero exit):**

```json
{ "success": false, "error": "..." }
```

## See Also

- [Getting Started](getting-started.md) — practical examples
- The other topical docs in this directory for deeper explanations
- Live `krebslog <group> --help` and `krebslog <group> <action> --help`
- [spec/01-spec.md](../spec/01-spec.md) for the original requirements

