# Reporting

The `report` group is the heart of krebslog. Reports synthesize nutrition (from nutlog) and training (from repslog) into metabolic insights with a strong emphasis on the Krebs cycle, energy flux, and redox balance.

All reports support the same flexible date language used everywhere else (`--since`, `--until`, `--period`, `--date`, `--days`, etc.).

Use `--json` to obtain complete machine-readable structures (raw series, computed stats, formulas, and assumptions) suitable for further agent processing or storage.

## Period Filtering (Common to All Reports)

- `--since DATE` / `--until DATE` — inclusive bounds (flexible syntax).
- `--period "14d"` or `--period "last 7 days"` — convenience shorthand (especially useful on `krebs-flux` and `energy-balance`).
- When a date filter is omitted, the report uses a sensible default window or all available data.

Dates are parsed with the rules described in [data-model.md](data-model.md).

## Daily Report

```bash
krebslog report daily --date today
krebslog --json report daily --date "2026-06-05" --include-image
```

Intended output (when fully implemented):

- Snapshot of nutrition totals for the day (kcal, macros, key micros, antioxidant score)
- Training load summary (volume, effective reps, session load, zones)
- Derived scores for that day: energy balance, krebs flux proxy, redox load
- Optional sidecar image path when `--include-image` is supplied

This is the "morning briefing" report most users and agents will request every day.

## Weekly / Multi-Day Aggregate

```bash
krebslog report weekly --since "last monday" --until today
krebslog --json report weekly --since "last 14 days"
```

Aggregates the daily grain over the window and shows trends + averages.

## Krebs Flux Report

```bash
krebslog report krebs-flux --period 14d
krebslog report krebs-flux --period 14d --output ./reports/krebs-flux.png
```

Focuses on the estimated flux through the TCA cycle:

- Contributions from carbohydrate (acetyl-CoA)
- Fat oxidation / mobilization
- Protein anaplerosis / cataplerosis
- Training-driven demand

The JSON output includes the exact weighting formula that was used so an agent can explain or question the number.

The optional `--output` (or the later `image krebs-cycle` command) can render a stylized cycle diagram with arrow thickness/color reflecting your personal flux for the period.

## Redox Balance Report

```bash
krebslog report redox-balance --since "last 7 days"
krebslog --json report redox-balance --since "last 30 days"
```

Compares training oxidative load (ROS proxy from volume, intensity, duration, HR zones) against antioxidant support coming from the diet (micronutrients, specific tags in nutlog products, etc.).

High value for recovery-focused agents and users.

## Energy Balance

```bash
krebslog report energy-balance --since "last 14 days"
krebslog --json report energy-balance --include-body-trends
```

Intake (nutrition) vs. estimated expenditure (training + optional resting metabolism hints).

`--include-body-trends` pulls weight/body-composition movement from bodylog (if the binary is present) and includes a rich `body` object in the JSON (stats + series for weight, body-fat, etc.). The weight delta serves as an observed outcome signal to help interpret the calculated energy balance. A `body_validation` block with qualitative label and discrepancy severity is also surfaced when body data is present. See spec/03-bodylog.md and spec/04-krebs-status.md.

## Krebs Status (Body-Validated)

```bash
krebslog report krebs-status --since "last 14 days"
krebslog --json report krebs-status --since "last 14 days" --include-raw
krebslog report krebs-status --date today
```

The primary integrated surface (per spec/04-krebs-status.md). Combines:

- Krebs flux proxy (0-10) with component breakdown (carb availability, fat mobilization, protein anaplerosis, training demand) and explicit formula.
- Redox balance (ROS vs. antioxidant proxies) + interpretation.
- Energy balance with `body_validation` (observed weight/muscle deltas turned into energy equivalents using 7700 kcal/kg fat / 5500 kcal/kg muscle, discrepancy severity, confidence).
- `body_adaptation` (latest measurement + trends for weight/muscle/fat + implication for Krebs function).
- Rule-based `insights[]` and a full `assumptions` + `caveats` block (measurement sparsity, short-term noise, normalization status).

Everything degrades gracefully when bodylog is absent or the window is sparse — flux/redox/energy are still produced with clear notes. `--include-raw` embeds the child payloads for agents.

Human output uses a compact table + narrative + bullets + explicit "Methodology & Caveats" section.

## Correlations

```bash
krebslog report correlations --x nutrition --y training-load --period 30d
krebslog --json report correlations --x carbs --y effective-reps
```

Simple exploratory views showing relationships between nutrition inputs and training outputs over a window. Useful for agents looking for "what actually moves the needle for this user."

## Web (Interactive HTML Report)

```bash
krebslog report web --period "last 30 days" --output ~/reports/krebslog/
krebslog --json report web --since today --output ./my-krebs.html
```

Produces a rich, self-contained, explorable single-file (or small folder) HTML dashboard per [spec/02-web-report.md](../spec/02-web-report.md):

- Dark theme, mobile-first, touch-friendly bottom sheets.
- Interactive Krebs cycle SVG (8 core steps) with status-colored nodes and flux-weighted arrows.
- KPI cards, inputs/outputs/redox, detailed step table, client-side trends, and complete methodology block.
- Embedded `data.json` (or adjacent file) with the full structured payload (kpis, steps, raw child data, formulas).
- Graceful when nutlog/repslog are missing or the window has little data — still renders a beautiful page with explanations.

Serve example:
```
python -m http.server 8080 --directory ~/reports/krebslog/krebs-status-2026-06-07/
# or just open the .html directly
```

This is the primary "deep dive" human + agent artifact. `--json` still gives the machine contract (paths + summary); the HTML is the deluxe view.

## Common Report Concepts (Future)

When the full aggregation engine lands:

- Every numeric report will include `count`, `min`, `max`, `avg`, `start`, `end`, `change`, and a simple `trend` direction.
- Raw daily series will be included under `--json` for agents that want to do their own statistics or charting.
- All derived scores will be accompanied by a small "assumptions" object listing the formula version, weights, and any config overrides that were active.

Human output uses `comfy-table` for clean tables plus a one-line summary. JSON is the rich form.

## See Also

- [Command Reference](command-reference.md) for every flag on the `report` subcommands.
- [Data Model](data-model.md) — exactly how the daily grain and derived scores are defined.
- [Image Generation](image-generation.md) — turning flux and balance numbers into visuals.
- [Agent & JSON Usage](agent-usage.md) — how agents typically consume report output.
