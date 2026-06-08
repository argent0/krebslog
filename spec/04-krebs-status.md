# Specification: Body-Enhanced Krebs Cycle Status & Adaptation Insights

**Project:** krebslog  
**Feature:** Full exploitation of now-available bodylog data to ground Krebs (TCA) cycle flux estimates, redox balance, and energy partitioning in real-world body composition and adaptation outcomes.  
**Status:** Draft v0.1 (Planning Spec)  
**Date:** 2026-06-08  
**Author:** Grok  
**Goal:** Transform krebslog from "input + training proxy" metabolic reports into a system that delivers validated, personalized **Krebs cycle status** the user (and Hermes agent) can actually trust and act on — using bodylog as the critical outcome signal.  
**Related:** spec/01-spec.md, spec/03-bodylog.md (implemented), implementation-report-bodylog.md, AGENTS.md, docs/reporting.md, docs/data-model.md, docs/agent-usage.md

---

## 1. Purpose & Philosophy

The core promise of krebslog is a "unified metabolic operating picture centered on the Krebs cycle". Until bodylog integration, this picture was built from *upstream estimates* only:

- Nutrition → estimated acetyl-CoA supply to the cycle (nutlog)
- Training → energy demand + ROS load on mitochondria (repslog)

**Bodylog now supplies the missing downstream reality check**:

- Observed weight, fat, muscle, visceral fat, BMI trends
- Direct outcome of energy balance, nutrient partitioning, recovery, and mitochondrial adaptation

This spec defines how to make **full use** of that information so the user gains genuine insight into:

- Whether estimated Krebs flux is being realized (or bottlenecked)
- How body adaptation (recomp, fat loss, muscle gain) validates or challenges the flux/redox/energy models
- Personalized signals for metabolic flexibility, efficiency, and potential interventions (nutrition timing, antioxidant support, training adjustments, recovery)

All new derived status, scores, and insights **must remain fully transparent** — formulas, weights, assumptions, and body-data-specific caveats visible under `--json` and explained in human output. This preserves the "boring, predictable, agent-first" contract.

## 2. Why Body Data Transforms Krebs Cycle Understanding

The Krebs (TCA) cycle sits at the intersection of catabolism and anabolism:

- **Inputs/flux**: Acetyl-CoA (primarily carbs via PDH, fats via β-oxidation, some AA), anaplerotic replenishment (glutamine, etc.)
- **Outputs**: Reducing power (NADH/FADH₂) for ETC/ATP, GTP, CO₂, and biosynthetic intermediates (e.g. for heme, amino acids, nucleotides)
- **Regulation**: Redox state (NAD⁺/NADH), energy charge, Ca²⁺, allosteric effectors, mitochondrial health

Body composition changes are the integrated, real-world consequence of flux through this hub over days/weeks:

- Consistent fat loss + stable or increasing muscle in a mild deficit → efficient Krebs cycling + good mitochondrial function + adequate antioxidant/redox support for recovery and anaplerosis.
- Weight stability or gain despite estimated deficit + high training load → possible under-estimated expenditure, high TEF/NEAT, glycogen/water effects, or *inefficient* partitioning (Krebs intermediates diverted or bottlenecked).
- Muscle gain without fat gain in surplus → excellent nutrient partitioning into anabolism, supported by TCA intermediate pools and low chronic ROS.
- Stalled fat loss despite high estimated flux + high ROS proxy → possible redox bottleneck (glutathione/NADPH recycling insufficient) limiting β-oxidation or PDH activity.

**Bodylog turns abstract proxies into grounded status.** It allows krebslog to move from "here is what your inputs and training *suggest* about your Krebs cycle" to "here is what your body is *actually doing* in response — and what that tells us about cycle function, efficiency, and bottlenecks."

This is especially powerful for a single-user biohacker/athlete context (the target of the whole ecosystem).

## 3. High-Level Design Principles (Non-Negotiable)

1. **Live CLI only** — body data continues to come exclusively from `bodylog --json ...` calls. No direct DB access.
2. **Graceful & optional** — everything degrades cleanly if bodylog absent or window has no measurements (exactly as today).
3. **Transparency first** — every new score/insight includes `formula`, `inputs`, `assumptions`, `body_caveats`, `confidence` in JSON.
4. **Agent-first** — new surfaces and enriched `agent context` are immediately usable by Hermes-style agents.
5. **Progressive disclosure** — human output: crisp summary + narrative first; details, raw data, and methodology on demand or in `--json` / web report.
6. **Simple rules over magic** — start with documented weighted sums, deltas, and a small set of interpretable heuristics. Avoid opaque models.
7. **Date alignment** — body measurements are sparse; use nearest measurement, last-known, or explicit interpolation notes. Prefer 7d+ windows for trends.
8. **Update living docs** — command help, AGENTS.md, reporting.md, data-model.md, agent-usage.md, etc. stay in sync.

## 4. Proposed CLI Surface & Behavior

### 4.1 New Primary Command: `report krebs-status`

```bash
krebslog report krebs-status --since "last 14 days" [--until today] [--json] [--include-raw]
krebslog report krebs-status --date today          # single-day snapshot if measurement exists
```

**JSON shape (success case with body data):**
```json
{
  "success": true,
  "period": { "since": "2026-05-25", "until": "2026-06-07", "days": 14 },
  "sources": ["nutlog", "repslog", "bodylog"],
  "krebs_flux": {
    "proxy": 7.8,
    "scale": "0-10 (personal baseline pending history)",
    "components": {
      "carb_availability": 8.4,
      "fat_mobilization": 6.9,
      "protein_anaplerosis": 5.2,
      "training_demand": 8.7
    },
    "formula": "0.35*carb_norm + 0.25*fat_norm + 0.15*protein_anaplerosis + 0.25*training_load_norm (see assumptions)",
    "trend_7d": "up"
  },
  "redox_balance": {
    "score": 6.5,
    "interpretation": "adequate for current load but watch antioxidant intake during high-volume blocks",
    "ros_proxy": 7.2,
    "antioxidant_proxy": 6.1
  },
  "energy_balance": {
    "estimated_surplus_kcal": 180,
    "body_validation": {
      "weight_delta_kg": -0.4,
      "fat_delta_kg_est": -0.6,
      "muscle_delta_kg_est": +0.2,
      "interpretation": "Mild observed deficit signal despite small estimated surplus. Consistent with high Krebs flux + possible elevated NEAT or TEF. Short-term water/glycogen noise likely. Overall positive adaptation signal.",
      "discrepancy_severity": "low",
      "confidence": "medium (14d window, 4 measurements)"
    }
  },
  "body_adaptation": {
    "latest_measurement": { "date": "2026-06-07", "weight_kg": 82.1, "body_fat_pct": 21.1, "skeletal_muscle_pct": 37.6, ... },
    "trends": {
      "weight": { "change_kg": -0.6, "trend": "down" },
      "skeletal_muscle": { "change_pct": +0.3, "trend": "up" },
      "body_fat": { "change_pct": -0.8, "trend": "down" }
    },
    "implication_for_krebs": "Lean mass gain + fat loss in context of training load validates efficient TCA intermediate pools and good recovery capacity. Supports continued high flux without apparent mitochondrial/redox bottleneck."
  },
  "insights": [
    "Krebs flux strong and rising — driven by consistent peri-training carbs and high effective volume.",
    "Body comp moving in desired direction (recomp). Alignment with flux estimate is good → model credible.",
    "Redox adequate; consider targeted antioxidant support (e.g. via nutlog tags) if high-volume week planned.",
    "Measurement timing note: last 3 weights taken post-workout — possible transient glycogen effect on scale."
  ],
  "assumptions": {
    "energy_equiv_fat_kg": 7700,
    "energy_equiv_muscle_kg": 5500,
    "short_term_noise_factors_ignored": ["glycogen", "water", "gut_content", "scale_precision"],
    "personal_baseline": "not yet available (needs 30+ days history in cache)"
  },
  "caveats": [
    "Body data sparse (4 measurements in 14d). Trends use linear fit.",
    "No resting_metabolism logged in bodylog for this window — used profile-derived estimate."
  ],
  "raw": { ... }   // optional with --include-raw
}
```

**Human-readable output** (default):
Clean comfy-table summary of flux / redox / energy + body deltas, followed by 1-2 paragraph narrative explanation + 3-5 bullet insights + explicit "Methodology & Caveats" section (collapsible in web, always in `--json`).

If no bodylog data in window: still produce excellent flux/redox/energy output with `body_available: false` and note that body validation is unavailable.

### 4.2 Enhancements to Existing Reports

- **`report energy-balance --include-body-trends`** (or make body inclusion default when available):
  - Already partially wired. Extend the narrative to always include body_validation block when body data present.
  - Add qualitative label: "Energy estimate validated by body trend", "Possible under-estimate of expenditure", "High apparent efficiency", etc.

- **`report krebs-flux`** (existing per spec/01):
  - Extend output with `body_validation` and `adaptation_implication` sections (reuse logic from krebs-status).
  - Consider alias or future merge with `krebs-status` for the integrated view.

- **`report redox-balance`**:
  - Add correlation note or simple rule: e.g. if high training ROS proxy + stalled fat loss or slow muscle recovery → "possible redox constraint on β-oxidation / PDH flux".

- **`report daily --date today`** and **`report weekly`**:
  - If a body measurement exists on (or nearest to) the anchor date(s), surface key values + one-sentence Krebs context (e.g. "Body: 82.1 kg (-0.3 kg wk). Flux context: strong alignment with estimated high output from training block.").

- **`report correlations`**:
  - Pre-define and expose high-value pairs involving body data: training_load vs muscle_gain_7d, estimated_flux vs weight_stability, antioxidant_proxy vs fat_loss_rate, etc.
  - Allow user-specified x/y including any body metric.

- **`report web`**:
  - Add body-krebs quadrant/panel in the interactive HTML: flux line chart overlaid with weight/sparkline, discrepancy callout cards, adaptation efficiency gauge.
  - Include bodylog in `meta.sources` and methodology footer.

### 4.3 Image Generation

- **`image krebs-cycle`**:
  - Optional `--include-body-context` (or default): adds a bottom strip or side callout showing period body deltas + qualitative alignment verdict. Color-code flux arrows or metabolite nodes subtly based on whether body outcome supports high activity in that pathway (e.g. fat-loss signal → thicker β-ox → acetyl-CoA arrow).

- **`image full-dashboard`** and future composites:
  - Include a dedicated "Krebs Status + Body Outcome" card/panel with mini flux gauge, body trend sparklines, and top insight.

- **New image type (Phase 4+)**: `image metabolic-adaptation` or `body-krebs` — purpose-built composite for Telegram / reports: body comp waterfall or dual-axis chart + Krebs flux heatmap + redox status + 2-3 key narrative bullets.

All images remain pure-Rust (plotters + custom SVG) behind the existing feature flag.

### 4.4 Agent Context & Tuning

`krebslog agent context --for hermes --since "last 30 days"` must now include a top-level `krebs_status` object (compact, narrative-ready):

```json
"krebs_status": {
  "overall_rating": "strong_efficient_adapting",
  "one_sentence": "Krebs flux running strong (7.8/10) and body composition is moving in the right direction (recomp signal). Good validation of your current nutrition + training approach.",
  "key_flags": ["mild_energy_discrepancy_low_severity", "redox_adequate"],
  "body_validated_insights": [
    "Lean mass up while weight slightly down → excellent partitioning; TCA intermediates likely well supplied for anabolism and recovery.",
    "..."
  ],
  "recommendations": ["continue current carb timing around workouts", "monitor antioxidant tags in nutlog during next high-volume phase"]
}
```

`agent tune --focus "krebs cycle status interpretation"` can generate or refine few-shot examples and prompt fragments that teach the agent how to read these enriched bundles.

`agent skills export` updated to reflect new command.

### 4.5 Telegram

- `telegram report krebs-status` (new) or extend `telegram daily/weekly` to optionally include a krebs-status summary line + path to enhanced image.
- Caption remains concise MarkdownV2, optimized for posting.

## 5. Derived Calculations & Transparency Requirements

All new logic lives in `src/commands/report.rs` (or a new `krebs.rs` module if it grows) and must emit:

- Exact formula string
- List of input sources and raw values used
- Explicit assumptions (energy equivalents, normalization constants, personal baseline status)
- Body-specific caveats (measurement count, window length, timing relative to training/meals, profile vs logged BMR, interpolation method)
- Confidence / discrepancy severity

**Example starting formulas (to be refined in code, always overridable via future config):**

**Krebs Flux Proxy (0-10)**
```
carb_score     = normalize(carb_g_per_day_avg, personal_carb_range)
fat_score      = normalize(fat_g_per_day_avg * beta_ox_efficiency_hint, ...)
protein_score  = min(3.0, protein_g * anaplerosis_factor)
training_score = normalize(training_load_proxy, personal_load_range)
flux = 0.35*carb + 0.25*fat + 0.15*protein + 0.25*training
```

**Body Validation / Discrepancy**
```
est_expenditure = repslog_estimated + (bodylog_BMR or harris_benedict_from_profile)
est_surplus   = nutlog_intake_kcal - est_expenditure
observed_equiv = (weight_delta_kg * 7700) + (muscle_delta_kg * 5500)   # simplified, sign-aware
discrepancy   = est_surplus - observed_equiv
discrepancy_severity = clamp( |discrepancy| / (est_surplus + 100), 0, 1 )
```

**Simple Heuristic Insights (rule-based, transparent)**
- If discrepancy_severity low AND muscle_up AND fat_down → "Strong validation: efficient Krebs-supported recomp"
- If high training ROS + stalled fat loss + low antioxidant_proxy → "Watch for redox bottleneck; consider increasing tagged antioxidants or deload"
- If estimated surplus + weight stable/down + muscle stable → "Possible under-reported expenditure or high metabolic demand not captured in training log"

These heuristics power the `insights` array and narrative paragraphs. They are versioned and logged in output.

## 6. Data Model & Cache Implications (Future Phase)

When the optional daily aggregate cache is implemented (Phase 5 of original spec):

- Extend daily grain or add sparse `body_krebs_daily(date, weight_kg, body_fat_pct, skeletal_muscle_pct, visceral_fat_level, estimated_krebs_flux, redox_score, body_alignment_score, ...)`
- Or store body measurements separately and join.
- Pre-compute rolling deltas (7d, 14d, 30d) for fast trend access.
- `krebs_status` historical queries become possible (`report krebs-status --since 90d` can show trend of flux + alignment score).
- Migration handled via existing `migrate` command + schema version.

Body data remains **derived from prior live pulls**, never authoritative source.

## 7. Phased Implementation Plan

**Phase 0 — This Spec (complete when committed)**
- Finalize and commit `spec/04-krebs-status.md`
- Update AGENTS.md "Common Tasks" table with new report and context work
- Add placeholder sections or notes in docs/reporting.md and docs/data-model.md

**Phase 1 — Wire Body into Existing Reports (high confidence, quick value)**
- Extend `report energy-balance`, `krebs-flux`, `redox-balance`, `daily`, `weekly` to include body_validation / adaptation sections when data available.
- Update gather logic and human/JSON renderers in `report.rs`
- Add simple discrepancy calculation + narrative template
- Update docs and command help text

**Phase 2 — New `report krebs-status` + Agent Enrichment**
- Implement the new command variant + full integrated status builder
- Enrich `agent context` output with `krebs_status` object
- Add basic insights generator (rule-based)
- Full `--json` contract + human tables
- Update AGENTS.md examples and agent-usage.md

**Phase 3 — Visual & Telegram Polish**
- Enhance `image krebs-cycle` and `full-dashboard` with body context overlays/callouts
- Optional new image type for adaptation dashboard
- `telegram report krebs-status` support

**Phase 4 — Correlations + Web**
- Extend `report correlations` with body metrics and pre-canned high-value pairs
- Update web report template/JS for new panels (flux vs weight overlay, alignment gauge, insight cards)

**Phase 5 — Cache + History (when aggregation lands)**
- Persist new derived fields
- Enable historical krebs-status trends and long-term adaptation analysis
- Update `migrate`, `cache info`, `data status`

**Phase 6 — Docs, Tests, Polish**
- Complete updates to command-reference.md, reporting.md, data-model.md, getting-started.md, troubleshooting.md
- Add runnable examples and agent prompt fragments
- `cargo fmt && cargo clippy -- -D warnings && cargo test` + live verification with real data
- Mark roadmap item in spec/01-spec.md as delivered

## 8. Success Criteria

Before feature considered complete:

- `krebslog --json report krebs-status --since "last 14 days"` returns the rich shape above (or close) when body data exists in window, and graceful shape when it does not.
- `report energy-balance`, `krebs-flux`, etc. all surface body-informed interpretation without requiring extra flags (or with clear default behavior).
- `agent context` JSON contains a non-empty, useful `krebs_status` block that references body outcomes.
- Images generated with body context do not crash and visually communicate alignment.
- Every numeric derived value in `--json` is accompanied by its formula/assumptions/caveats.
- Human output for new surfaces is educational, concise, and actionable.
- All listed docs and AGENTS.md are updated and consistent.
- No new heavy dependencies; no violation of live-CLI-only rule.
- The feature feels like a natural, high-leverage extension of the existing bodylog integration — "exactly what body data was meant for in a Krebs-focused tool."

## 9. Risks, Edge Cases & Mitigations

- **Short-term body noise** (glycogen depletion supercompensation, water retention post high-carb, scale variance, timing vs workouts/meals): Always include explicit short-term caveat + prefer multi-day trends + document best practices (morning fasted consistent conditions).
- **Sparse measurements**: Note count and confidence; fall back to last-known or linear interpolation with flag.
- **Profile vs logged BMR**: Prefer bodylog `resting_metabolism` when present; otherwise note use of profile-derived estimate.
- **Over-confident interpretation**: Start conservative; insights are suggestions, not diagnoses. User (and agent) can always see raw + assumptions.
- **Formula evolution**: Keep formulas simple and versioned in output. Future cache history will allow personal baselines and better normalization.

## Appendix A: Example Invocations (Post-Implementation)

```bash
# Core new surface
krebslog --json report krebs-status --since "last 14 days"

# With body emphasis (future default behavior)
krebslog report energy-balance --since "last 30 days"

# Agent-ready bundle
krebslog --json agent context --for hermes --since "last 30 days"

# Visual with context
krebslog image krebs-cycle --since "last 14 days" --include-body-context --theme dark

# Correlations involving body outcomes
krebslog report correlations --x training_load --y muscle_gain
```

## Appendix B: Updates Required to Other Files

- `spec/01-spec.md` — update roadmap item 5 or add new item; refresh command examples and architecture text.
- `AGENTS.md` — add to Common Tasks table; update quick-reference examples; ensure "never open source DBs" still accurate.
- `docs/reporting.md` — new section on krebs-status report + body validation patterns.
- `docs/data-model.md` — extend Core Daily Grain and add body-krebs derived fields.
- `docs/command-reference.md` — document new command + flag changes.
- `docs/agent-usage.md` — examples of enriched context and krebs-status usage.
- `docs/image-generation.md` — body context overlays.
- `src/cli.rs`, `src/commands/report.rs`, `src/commands/agent.rs`, `src/commands/image.rs` — implementation targets.
- `README.md` (minor) if high-level description needs refresh.

---

**This specification is the authoritative plan for turning the now-available bodylog information into first-class Krebs cycle status intelligence.** Implementation should follow the phases, respect every constraint, and keep transparency and the live-CLI contract absolute. After delivery, mark the corresponding item in the main spec and close the loop with a new implementation report in `reports/`.

The end result: the user (and his LLM agent) will finally have a clear, validated picture of how his personal Krebs cycle is performing in the context of real body adaptation — exactly the deeper understanding this tool was built to deliver.