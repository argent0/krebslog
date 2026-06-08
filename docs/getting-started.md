# Getting Started

This guide walks through a minimal productive session with `krebslog`.

It assumes you already have working installations of `nutlog` and `repslog` (the only data sources krebslog understands).

## 1. Verify Your Data Sources

`krebslog` does **not** store nutrition or training data itself. It pulls everything live (or from its own small cache) by invoking the sibling tools.

```bash
which nutlog repslog
nutlog --json consumption list --since today
repslog --json stats summary --days 7
```

If either is missing, install them first (see their own documentation).

Then ask krebslog to discover them:

```bash
krebslog data status --probe
```

You should see both sources listed as "available" with a successful probe.

## 2. Your First Data Pull

Pull recent data from both tools (the convenient bulk form):

```bash
krebslog --json data pull --all --period "last 30 days"
```

Or pull specific entities with full control:

```bash
krebslog --json data pull --source nutlog --entity consumption --since "last 14 days"
krebslog --json data pull --source repslog --entity workout --since "last 14 days"
krebslog --json data pull --source repslog --entity "stats:summary" --since "last 14 days"
```

`--period` accepts the same flexible date language as `--since` (e.g. `90d`, `last 30 days`).

`--dry-run` lets you see exactly what would be called without executing anything.

## 3. Database / Cache Location

krebslog maintains an *optional* lightweight SQLite cache of daily aggregates (nutrition totals, training load, derived krebs flux and redox scores, etc.).

Default location (XDG):

```
$XDG_DATA_HOME/krebslog/krebslog.db
# usually ~/.local/share/krebslog/krebslog.db
```

Override for a session or script:

```bash
krebslog --db /tmp/demo-krebs.db --json data pull --all --period "last 7 days"
krebslog --db /tmp/demo-krebs.db --json report daily --date today
```

Completely disable the cache (always hit the live child CLIs):

```bash
krebslog --no-cache --json report redox-balance --since "last 7 days"
```

> **Agent tip**: Pass `--db` explicitly in automated flows for reproducibility. Use `--no-cache` when you need guaranteed fresh data from nutlog/repslog.

The parent directory is created automatically on first use.

## 4. Run Your First Report

Even in the current skeleton you can exercise the command surface:

```bash
krebslog --json report daily --date today
krebslog --json report krebs-flux --period 14d
krebslog --json report redox-balance --since "last 7 days"
```

Human-readable (non-JSON) output is also supported for quick inspection.

Full reports (with real aggregation, derived scores, and formulas in the JSON) will appear as the aggregation engine is implemented.

## 5. Generate Visuals (Planned)

Image commands are wired and documented but the actual rendering (Krebs cycle diagram with flux overlays, training heatmaps, antioxidant shield, composite dashboards) is a later milestone.

```bash
krebslog image krebs-cycle --date-range "last 30 days" --theme dark --output ./reports/krebs-cycle.png
krebslog image full-dashboard --date today --layout telegram
```

Images are designed for direct use in Telegram posts or as context for Hermes.

## 6. Telegram-Ready Output

```bash
krebslog --json telegram daily --date today
krebslog --json telegram report --type redox --period 7d --post-ready
```

These commands emit a ready-to-use MarkdownV2 caption plus the path to a generated image. A separate bot or your Hermes agent can watch the output directory or call krebslog on demand and post the pair.

## 7. Agent / Hermes Context Bundles

One of krebslog's primary purposes is feeding rich, up-to-date metabolic context to LLM agents:

```bash
krebslog --json agent context --for hermes --since "last 30 days" --output /tmp/hermes-krebs-context.json
```

See [agent-usage.md](agent-usage.md) for the full contract, recommended invocation patterns, and how to combine krebslog with nutlog/repslog calls inside an agent loop.

## 8. Minimal Human Workflow

1. `krebslog data status --probe` (confirm sources)
2. `krebslog --json data pull --all --period "last 30 days"`
3. `krebslog report daily --date today`
4. `krebslog --json report redox-balance --since "last 7 days"`
5. (later) `krebslog image krebs-cycle --date-range "last 14 days"`

## 9. Minimal Agent / Scripting Workflow

Always use `--json`, prefer explicit `--db` for isolation, and treat non-zero exit + `{"success": false, ...}` as failure.

```bash
DB="$HOME/.local/share/krebslog/krebslog.db"

krebslog --json --db "$DB" data pull --all --period "last 30 days"
krebslog --json --db "$DB" report daily --date today --include-image
krebslog --json --db "$DB" agent context --for hermes --since "last 30 days"
```

Parse stdout as JSON. Capture image paths from report/telegram/image commands when `--include-image` or equivalent is used.

## Environment & Overrides

- `NUTLOG_BIN` / `--nutlog-bin`
- `REPSLOG_BIN` / `--repslog-bin`
- `BODYLOG_BIN` / `--bodylog-bin`

Useful when the tools are installed in non-standard locations or you want to test against a development build of nutlog/repslog.

## Next

- Full syntax for every command: [Command Reference](command-reference.md)
- What krebslog actually stores and derives: [Data Model](data-model.md)
- How data actually enters the system: [Data Ingestion / Pulling](pulling.md) (to be added)
- Using from LLM agents: [Agent & JSON Usage](agent-usage.md)
- Visuals and Telegram: [Image Generation](image-generation.md) and [Telegram Integration](telegram-integration.md)
- Common problems: [Troubleshooting](troubleshooting.md)
