**krebslog** — Full SpecificatioMetabolic Reports, Krebs Cycle Visualizer & Redox Insightsts*CLI-first • Local-only • LLM-agent primary interface • Image-capable reports for Telegram & Hermes agent tuningng**

### 1. Purpose & Philosophy (Exactly like nutlog)
krebslog is a simple, local, CLI-first Rust tool that turns data from your existing nutlog and repslog (and optionally bodylog) tools into rich, visual metabolic reports centered on Krebs (TCA) cyclele**, energy flux, training load, nutrition inpuantioxidant/redox balancece**, and body adaptation outcomeCore Philosophy (identical to nutlog)g)**:
- Local-first, offline, single-user, no cloud, no accountsCLI is the primary interfacece**. Everything is a command--json is the reliable machine interfacece** (designed for LLM agents first).
- Puall input data exclusively by calling the other CLIsIs** (nutlog --json ... and repslog --json ...). **Zero direct database access**.
- Flexible natural-language dates (today, yesterday, last 7 days, 2026-05-20).
- Minimal dependencies, predictable behavior, extensive inline + docs/ documentation.
- Packaged for Arch (PKGBUILD) and easy cargo install.
- BuLLM agentsagents** (rich AGENTS.md, skill packages, consistent JSON success/error shapes).

It exists to give you (and your Hermes aunified metabolic operating pictureicture** without duplicating the logging tools.

### 2. High-Level Architecture
krebslog
├── Calls (subprocess + --json)
│   ├── nutlog (nutrition, consumption, products, reports)
│   └── repslog (workouts, sets, stats, exercises)
│   └── (future: bodylog measurements)
├── Internal (optional local SQLite cache for speed — fully disableable)
├── Processing (daily/period aggregates, flux proxies, redox balance, correlations)
├── Output
│   ├── JSON (for agents & scripting)
│   ├── Human tables / Markdown
│   └── Images (PNG/SVG) for Telegram posts & visual reports
└── Special modes
    ├── Telegram-ready formatting
    └── Hermes agent context / prompt tuning bundles
**No direct SQLite reads** of the source tools. All data freshness comes from live CLI calls (or cached previous calls).

### 3. Command Structure (nutlog-style)
Global flags (every command):
- --json — machine output (default for agents)
- --quiet
- --db /path/to/krebslog.db (its own optional cache)
- --nutlog-bin /usr/bin/nutlog (override binary)
- --repslog-bin /usr/bin/repslog
- --date-format etc.

**Main command groups**:

```bash
# Data ingestion (pull & cache)
krebslog data pull --source nutlog --entity consumption --since "last 30 days"
krebslog data pull --source repslog --entity workout --since "2026-05-01"
krebslog data pull --all --period 90d          # convenient shortcut
krebslog data status                          # shows last pull times per source

# Reports (core value)
krebslog report daily --date today --include-image
krebslog report weekly --since "last monday" --until today
krebslog report krebs-flux --period 14d --output ./reports/krebs-flux.png
krebslog report redox-balance --since "last 7 days"   # antioxidant vs training load
krebslog report energy-balance --include-body-trends  # when bodylog added
krebslog report correlations --x nutrition --y training-load

# Visual generation (first-class)
krebslog image krebs-cycle --date-range "last 30 days" --output krebs-cycle.png
krebslog image training-load-heatmap --period 90d
krebslog image antioxidant-shield --style dark --output redox.png
krebslog image full-dashboard --date today --layout telegram

# Telegram-optimized output
krebslog telegram daily --date today          # prints MarkdownV2 caption + image path
krebslog telegram report --type redox --period 7d --post-ready
```

# Hermes agent support & tuning
krebslog agent context --for hermes --since "last 30 days" --output hermes-context.json
krebslog agent tune --generate-prompts --focus "redox and recovery"
krebslog agent skills export                  # copies skill files for Hermes

# Utilities
krebslog config set nutlog.path ...
krebslog migrate
krebslog cache clear

### 4. Data Model (Internal Aggregates)
`krebslog` maintains its own lightweight local state (optional SQLite at `~/.local/share/krebslog/krebslog.db` or user-specified).

**Core daily grain** (one row per local calendar day):
- Date (YYYY-MM-DD)
- Nutrition (from nutlog): total kcal, protein_g, carbs_g, fat_g, estimated acetyl-CoA precursors, antioxidant_score (custom or derived from tags/nutrients)
- Training (from repslog): total volume_kg_reps, effective_reps, cardio_calories, time_in_zones (Z1–Z5), session_load (RPE × duration), energy_system_mix
- Derived:
  - Estimated energy balance
  - Krebs flux proxy (simple weighted model: carb availability + fat mobilization + protein anaplerosis + training demand)
  - Redox / Oxidative load score (training ROS proxy – antioxidant support)
  - Balance indicators (e.g., antioxidant_adequacy vs load)

All derived values are transparent — the JSON output includes the formulas/assumptions used.

### 5. Image Generation Capabilities
`krebslog` can generate publication-quality images for reports:

- **Standard charts**: Training load over time, macro trends, energy balance, HR zone distribution, antioxidant intake vs load — powered by the `plotters` crate (pure Rust, no Python dependency).
- **Custom Krebs Cycle diagram**: Built-in SVG generator with:
  - Core cycle metabolites
  - Overlay arrows whose thickness/color reflect your personal flux proxies (nutrition inputs, training demand)
  - “Antioxidant Shield” layer (glutathione/NADPH recycling, ROS quenching indicators)
  - Data callouts for the selected period
- **Full dashboard composites**: Multiple panels in one image (overview cards + cycle viz + trends) — ideal for Telegram.
- Output formats: PNG (default), SVG.
- Styling: Light/dark themes, configurable colors, Telegram-optimized aspect ratios (e.g., 1080×1920 or square for stories).
- All images saved to `~/reports/krebslog/` (or configurable) with consistent naming.

Example:
```bash
krebslog image krebs-cycle --since "last 14 days" --theme dark --output ./reports/krebs-2026-06-07.png
```

### 6. Telegram Consumption
Reports are designed to be posted directly to Telegram (channel, group, or saved messages):

- `krebslog telegram daily` outputs:
  - Ready-to-use MarkdownV2 caption (with bold, lists, emojis)
  - Path to the generated PNG
  - Optional JSON block for bots that want structured data
- A separate Telegram bot (or your Hermes agent) can watch the `reports/` directory or call the CLI and post the image + caption.
- No Telegram API library in `krebslog` itself (keeps philosophy minimal and local). It just produces perfect content for a bot/agent to consume.

### 7. Hermes Agent Integration & Tuning
Because everything is `--json` + well-documented, `krebslog` is a first-class tool for your Hermes agent. Built-in support:
- `AGENTS.md` with exact command examples and JSON schemas.
- `krebslog agent context` generates a compact, up-to-date context bundle (nutrition summary + training load + redox balance + key insights) that the agent can load into its prompt or RAG.
- `krebslog agent tune` can generate or refine prompt templates / few-shot examples focused on metabolic interpretation, recovery advice, or flux analysis.
- Skill packages (similar to repslog) can be installed so the Hermes agent natively “knows” how to call `krebslog`.

This allows the agent to reason over the combined data and even request new images/reports on demand.

### 8. Tech Stack & Implementation Notes
- Language: Rust (matching nutlog/repslog/bodylog)
- CLI: clap v4
- Data pulling: std::process::Command to invoke the other binaries with --json
- Caching (optional): rusqlite + simple schema (can be completely disabled with --no-cache)
- Image generation: plotters crate + custom SVG module (no heavy external deps)
- Date handling: chrono with the same flexible parsing as nutlog
- JSON: serde + consistent { "success": true, ... } / error shapes
- Docs: Extensive docs/ (data-model.md, command-reference.md, agent-usage.md, image-generation.md, telegram-integration.md)
- Packaging: PKGBUILD for Arch, easy cargo install --path .

Dependencies kept minimal — image generation is behind a default-enabled feature flag if desired.

### 9. Development & Contribution
Same standards as nutlog:
- cargo fmt && cargo clippy -- -D warnings && cargo test
- Clear separation: data pulling layer, aggregation engine, report generators, image renderers, output formatters.
- All formulas/assumptions for flux/redox scores are documented and overridable via config.

### 10. Roadmap / Phased Delivery (Suggested)
1. Core CLI skeleton + data pulling from nutlog + repslog + JSON output
2. Basic daily/weekly text reports + simple charts
3. Full Krebs cycle + antioxidant/redox image generation
4. Telegram-ready formatting + agent context/tuning commands
5. Optional bodylog integration + advanced correlations
6. Polish, docs, packaging

---

This spec stays 100% faithful to the nutlog philosophy while adding exactly the capabilities you asked for: live CLI-based input from nutlog + repslog, first-class image creation for reports, Telegram-optimized output, and strong support for tuning/feeding your Hermes agent.