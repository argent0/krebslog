# Data Model

This document describes the internal aggregates krebslog maintains, date handling, derived metabolic scores (Krebs flux, redox balance, etc.), and the optional local cache. Both humans and agents benefit from understanding these details.

## High-Level Architecture

`krebslog` does **not** duplicate the logging work done by nutlog and repslog.

- All raw data comes from live calls (or cached results of prior calls) to:
  - `nutlog --json consumption list ...`
  - `nutlog --json report nutrition ...`
  - `repslog --json workout list ...`
  - `repslog --json stats summary ...` / `stats volume ...`
- krebslog's job is **aggregation + derivation + presentation**:
  - Daily grain summaries
  - Krebs (TCA) cycle flux proxies
  - Redox / oxidative load balance
  - Energy balance
  - Correlations
  - Beautiful images and Telegram-ready bundles
  - Compact context for Hermes-style agents

## Core Daily Grain (Intended)

When the aggregation engine is complete, krebslog will maintain (in its optional cache) one conceptual row per local calendar day containing:

### Nutrition Inputs (sourced from nutlog)

- `total_kcal`
- `protein_g`, `carbs_g`, `fat_g`
- `estimated_acetyl_coa_precursors` (derived from carb + fat + protein intake)
- `antioxidant_score` (custom or derived from tags/nutrients in products)

### Training Inputs (sourced from repslog)

- `total_volume_kg_reps`
- `effective_reps`
- `cardio_calories`
- `time_in_zones` (Z1–Z5 heart rate seconds)
- `session_load` (RPE × duration or similar)
- `energy_system_mix` hints

### Derived Values (computed by krebslog)

- `energy_balance` (intake vs. estimated expenditure)
- `krebs_flux_proxy` — weighted model combining:
  - Carb availability (acetyl-CoA from carbs)
  - Fat mobilization
  - Protein anaplerosis
  - Training metabolic demand
- `redox_oxidative_load_score` — training ROS proxy
- `antioxidant_adequacy` vs. load balance indicators
- Other balance / adaptation signals

All derived values are **transparent**. Under `--json` you will receive the raw inputs, the weights/formula used, and the final number, plus any config overrides that were active.

## Optional Local Cache

Location (override with `--db`):

```
~/.local/share/krebslog/krebslog.db   (or $XDG_DATA_HOME/krebslog/krebslog.db)
```

The cache is:

- **Completely optional** — `--no-cache` makes every operation hit the live child CLIs.
- **Lightweight** — only daily aggregates + pull metadata + a small amount of config. It is not a second copy of all your nutlog/repslog rows.
- **Disableable at the binary level** in the future if desired.

Current status (skeleton phase): the `data pull` commands execute the child calls and surface their JSON, but full storage into daily aggregates + derived score computation is still being implemented. `data status` currently shows `last_pull: null`.

When implemented, schema changes will use simple rusqlite migrations (versioned via `PRAGMA user_version` or a dedicated migrations table), matching the style used in nutlog and the spirit of repslog.

## Date Handling

### Input Dates (`--date`, `--since`, `--until`, `--period`, `--date-range`)

krebslog uses an extended flexible date parser (in `src/utils.rs`) that accepts everything nutlog/repslog accept plus more:

- `today`, `yesterday`, `tomorrow`
- `YYYY-MM-DD` (recommended for agents and reproducibility)
- `N days ago` / `N day ago`
- `last week`, `last month`
- `last 7 days`, `last 30 days` (treated as window starts)
- Weekday forms: `last monday`, `last tue`, `this week`
- `this month`
- Common variants and full RFC3339 (date portion extracted)

Parsing is performed relative to the **local calendar** of the machine running `krebslog` (`chrono::Local`).

When dates are passed down to nutlog or repslog, they are normalized to `YYYY-MM-DD`.

**Agent advice**:
- If your agent runs on a different host/timezone than the "user", prefer passing explicit `YYYY-MM-DD` strings.
- "last monday" is always evaluated on the machine where the `krebslog` process executes.

### Internal Date Concepts

- The **logical day** for a daily aggregate is the user's local calendar day (`YYYY-MM-DD`).
- All `created_at` / `updated_at` (in the cache and in any future pull metadata) are full RFC3339 UTC strings.
- JSON output for timestamps follows the same shape used by nutlog:

  ```json
  "created_at": { "utc": "2026-06-07T17:47:58Z", "local": "2026-06-07T19:47:58+02:00" }
  ```

Human output converts timestamps to local zone of the displaying process.

## Configuration & Overridable Formulas

The `config` command group (and future config file / DB storage) will allow overriding:

- Paths to nutlog/repslog (already supported via flags/env)
- Weights and assumptions inside the Krebs flux and redox calculations
- Default report periods, image themes, output directories (`~/reports/krebslog` by default)
- Telegram aspect ratios / layouts

All active assumptions for a given report or image will be included in the `--json` output.

## JSON Shapes (Contract for Agents)

- Lists and raw child data are typically returned as top-level arrays or the exact shape produced by the child tool.
- High-level operations (pull acknowledgments, reports, context bundles) use the `{"success": true, ...}` envelope.
- Errors under `--json` are still printed to stdout as `{"success": false, "error": "..."}` and the process exits non-zero.
- Image and telegram commands include `path` fields pointing at generated artifacts.

See [agent-usage.md](agent-usage.md) and the live `--json` output for the current exact shapes.

## Why This Model?

- **Source of truth stays in the leaf loggers.** nutlog and repslog remain the only places you log nutrition or workouts.
- **krebslog is a view + derivation layer.** This matches the "unified metabolic operating picture" goal without duplication.
- **Optional cache for speed + offline use**, but never at the cost of freshness when the user (or agent) asks for it.
- **Transparent derivations** so agents and humans can reason about *why* a flux or redox number is high or low.
- Dates are deliberately split into "user's local day" (for aggregates) vs. UTC instants (for auditing).

## See Also

- [Command Reference](command-reference.md) — how the model is exposed in the CLI
- [Agent & JSON Usage](agent-usage.md) — how to consume and combine the data
- [Data Ingestion](pulling.md) — the live call layer (future dedicated doc)
- Source: `src/utils.rs` (date parser + child invocation), `src/commands/data.rs`, future aggregation/report modules, `spec/01-spec.md` section 4.
