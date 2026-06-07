# Image Generation

`krebslog` treats high-quality, publication-ready images as a first-class output, on equal footing with text and JSON reports.

Images are intended for:

- Direct posting to Telegram channels or saved messages
- Inclusion in Hermes agent context / RAG (as image descriptions or paths)
- Human review and motivation

## Output Formats & Styling

- PNG (default for sharing)
- SVG (for editing or high-resolution use)
- Light / dark themes (`--theme light|dark`)
- Configurable output directory (default `~/reports/krebslog` or value from `config`)

Aspect ratios suitable for Telegram stories, posts, and square dashboards are supported via the `full-dashboard` layout options.

## The Krebs Cycle Diagram

```bash
krebslog image krebs-cycle --date-range "last 30 days" --theme dark --output krebs-cycle.png
```

This is the signature visualization:

- Core TCA cycle metabolites (citrate, isocitrate, α-ketoglutarate, succinyl-CoA, succinate, fumarate, malate, oxaloacetate)
- Overlay arrows whose thickness and/or color reflect your personal flux proxies for the selected period (carb-driven acetyl-CoA, fat mobilization, protein contribution, training demand)
- "Antioxidant Shield" layer (glutathione / NADPH recycling indicators, ROS quenching)
- Data callouts showing the key nutrition and training drivers for the window
- Title and date range annotation

The diagram is generated with a combination of custom SVG construction + (when the feature is enabled) plotters for any supporting charts embedded in the composite.

## Training Load Heatmap

```bash
krebslog image training-load-heatmap --period 90d --output load-heatmap.png
```

Calendar-style or grid heatmap showing daily or weekly training load (volume, effective reps, session load, or a composite). Great for spotting training density, deloads, and progression patterns over multi-month windows.

## Antioxidant / Redox Shield

```bash
krebslog image antioxidant-shield --style dark --output redox.png
```

A more abstract / illustrative view emphasizing the balance (or imbalance) between training oxidative stress and dietary antioxidant support. Useful companion to the `report redox-balance` numbers.

## Full Dashboard Composites

```bash
krebslog image full-dashboard --date today --layout telegram
krebslog image full-dashboard --date-range "last 7 days" --layout square
```

Multi-panel images that combine:

- Overview cards (key numbers for the period)
- Mini Krebs cycle or flux arrows
- Trend sparklines or small heatmaps
- Redox / energy balance indicators

The `telegram` layout is tuned for vertical phone-friendly aspect ratios. `square` and `wide` options exist for other uses.

## Image Output in Other Commands

Several report and telegram commands accept `--include-image` or `--output` so you can get the visual as a side effect of asking for the textual/JSON report:

```bash
krebslog report krebs-flux --period 14d --output ./reports/flux-14d.png
krebslog --json telegram daily --date today   # includes image path in the payload
```

## Implementation Notes (Current Status)

- Command surface is fully defined in `src/cli.rs` (`ImageAction` variants).
- Actual rendering logic (SVG generation + optional plotters integration) is a planned milestone after the text/JSON reporting and data model are solid.
- Heavy dependencies (`plotters`) will live behind a Cargo feature flag so the core tool remains lightweight.
- All images are written to a user-configurable reports directory with consistent, sortable filenames.

## Styling & Theming

- Global or per-command `--theme light|dark`
- Future config keys for accent colors, label fonts, etc.
- Telegram-optimized output tries to stay legible even after compression and dark-mode clients.

## See Also

- [Command Reference](command-reference.md) — exact flags for every image subcommand.
- [Reporting](reporting.md) — the numeric data that feeds the visuals.
- [Telegram Integration](telegram-integration.md) — how images + captions are meant to be posted.
- [spec/01-spec.md](../spec/01-spec.md) section 5 — original vision for the Krebs cycle renderer and composite dashboards.
