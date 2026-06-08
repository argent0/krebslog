# Specification: Bodylog Data Integration for krebslog

**Project:** krebslog  
**Feature:** Bodylog support (third data source for body composition & trends)  
**Status:** Draft v0.1 (Planning Spec)  
**Date:** 2026-06-08  
**Author:** Grok (planning from real bodylog binary inspection + krebslog patterns)  
**Goal:** Optional, graceful, live-CLI-only integration that activates existing placeholders (`--include-body-trends`) and fulfills the "body adaptation outcomes" part of the project vision.

---

## 1. Purpose & Goals

krebslog's mission is a unified metabolic operating picture centered on the Krebs cycle, energy flux, training load, nutrition, redox balance, **and body adaptation outcomes**.

Bodylog (the sibling tool for daily body composition) supplies the missing "outcome" and "context" signals:

- Weight trends (as a real-world validator of energy balance claims)
- Body composition (fat %, skeletal muscle %, visceral fat, BMI)
- Profile data (height, date of birth) useful for BMR/REE context
- Resting metabolism estimates (when logged or derived by bodylog)
- Measurement-level fidelity (one record per local calendar day)

### Primary Goals
- Make `--include-body-trends` on `report energy-balance` (and future reports) actually work.
- Allow `data pull --all` and explicit `--source bodylog` to ingest body data exactly like nutlog/repslog.
- Surface body data in daily/weekly snapshots, web reports, images, Telegram bundles, and Hermes agent context when available.
- Improve interpretation of energy balance and "adaptation" (e.g. "weight down 0.6 kg while reporting surplus → expenditure or water/measurement effects").
- Preserve 100% optionality: krebslog remains fully functional (and tests pass) with zero bodylog binary present.

### Secondary Goals
- Provide clean extension points for future derived signals (e.g. fat-free mass trends, phase angle if bodylog adds it, long-term recomposition rates).
- Keep formulas/assumptions visible under `--json` (e.g. how weight delta qualifies an energy balance number, whether resting metabolism from bodylog was used).
- Educate agents and humans about the difference between *intake/expenditure estimates* and *observed body outcomes*.

---

## 2. Constraints & Design Principles (Non-Negotiable)

These are sacred and mirror the rules for nutlog + repslog:

1. **Live CLI calls only.** All body data comes from spawning `bodylog --json ...`. **krebslog must never open** `~/.local/share/bodylog/bodylog.db` (or any user `--db` for bodylog).
2. **Optional & graceful.** Bodylog is the third source. Absence of the binary or empty data must degrade cleanly (same as missing nutlog today).
3. **Symmetric flags & env.** `--bodylog-bin PATH` + `BODYLOG_BIN` env var, exactly parallel to the other two.
4. **Date handling.** krebslog's rich flexible parser (`utils.rs`) produces `YYYY-MM-DD` via `format_date_for_child`. Pass normalized calendar dates to bodylog. Bodylog's own date syntax is stricter (no "last 30 days" shorthand); krebslog translates.
5. **Passthrough + minimal normalization.** For `data pull`, forward bodylog's JSON largely as-is (agents want the real shapes). Higher-level reports may lightly reshape for convenience while including a `raw` or `source_data` block.
6. **Transparency of derivations.** Any use of body data in flux, redox, energy balance, or "adaptation" scores must emit the inputs, weights, and caveats in the `--json` output.
7. **Cache is krebslog's only DB.** Future daily aggregates or body snapshots live only in krebslog's optional cache. The cache remains fully bypassable with `--no-cache`.
8. **CLI contract.** `krebslog <group> <action>` structure. Every data-returning command supports `--json`. Help text is primary documentation.
9. **Agent first.** All new surfaces must be immediately usable by Hermes-style agents via `--json` and documented in AGENTS.md + agent-usage.md.
10. **No new heavy dependencies** for the core integration. Image enhancements that use body data may live behind the existing images feature flag if needed.

---

## 3. Observed Bodylog Interface (The Contract krebslog Will Consume)

(Inspected from the real `/usr/bin/bodylog` on 2026-06-08. Shapes are the ground truth for v1 integration.)

### Global
- Flags: `--json`, `--db <PATH>`, `--quiet`
- Subcommands: `measurement`, `report`, `config`

### `bodylog measurement list --since DATE [--until DATE]`
Returns a JSON **array** (newest first) of measurement records.

**Example record:**
```json
{
  "id": 3,
  "date": "2026-06-07",
  "weight_kg": 82.1,
  "body_fat_pct": 21.1,
  "skeletal_muscle_pct": 37.6,
  "visceral_fat_level": 10,
  "bmi": 26.8,
  "created_at": { "utc": "2026-06-07T19:27:51Z", "local": "2026-06-07T19:27:51Z" },
  "updated_at": { "utc": "2026-06-07T19:27:51Z", "local": "2026-06-07T19:27:51Z" }
}
```

**Notes:**
- `resting_metabolism_kcal` is accepted on create but often absent or null in list responses if not recorded.
- Dates are local calendar days (`YYYY-MM-DD`).
- Bodylog accepts flexible dates on its side: `today`, `yesterday`, `2026-06-05`, `last monday`, `3 days ago`, etc. krebslog will prefer passing `YYYY-MM-DD` for determinism.

### `bodylog report <metric> --since DATE [--until ...]`
Supported metrics (from `--help`): `weight`, `body-fat`, `muscle`, `visceral-fat`, `bmi`, `resting-metabolism`, `summary`.

**`report summary` shape (most useful aggregate):**
```json
{
  "period": { "since": "2026-06-01", "until": null },
  "weight": { "count": 3, "min": 82.1, "max": 82.7, "avg": 82.5, "start": 82.7, "end": 82.1, "change": -0.6, "trend": "down" },
  "body_fat": { ... similar ... },
  "skeletal_muscle": { ... },
  "visceral_fat": { ... },
  "bmi": { ... },
  "resting_metabolism": null,   // frequently null if not logged
  "measurement_count": 3
}
```

Individual reports (e.g. `report weight`) add a `series` array with denormalized daily rows containing the other fields for convenience.

### `bodylog config show`
```json
{
  "height_cm": 175.0,
  "date_of_birth": "1983-07-21",
  "updated_at": { "utc": "...", "local": "..." }
}
```

This is valuable for any future REE/BMR estimation inside krebslog when bodylog itself does not supply `resting_metabolism`.

### Date Syntax Notes for Callers
Bodylog does **not** accept krebslog's "last 30 days" shorthand in all contexts. krebslog's `data pull` and internal report gatherers **must** resolve to concrete `YYYY-MM-DD` before invoking bodylog (already the pattern used for nutlog in many places).

---

## 4. CLI Surface Changes

### 4.1 Global Flags (cli.rs)

Add symmetrically after the repslog flag:

```rust
/// Path to the bodylog binary (default: search PATH + common locations).
#[arg(long, global = true, value_name = "PATH", env = "BODYLOG_BIN")]
pub bodylog_bin: Option<String>,
```

Update `Context` (context.rs) with:
```rust
pub bodylog_bin: Option<String>,
```
and the `from_cli` mapping.

Update all documentation that lists global flags (command-reference.md, getting-started.md, etc.).

### 4.2 data pull

**Source enum extension (cli.rs + data.rs):**
- Change `value_parser = ["nutlog", "repslog"]` → include `"bodylog"`.
- Update help text: "Source tool: nutlog, repslog, or bodylog".

**New entities for bodylog (suggested mapping):**
- `measurement` (or `measurements`) → `bodylog measurement list --since ... --until ...`
- `report:summary` (or `summary`) → `bodylog report summary --since ...`
- `report:weight`, `report:body-fat`, `report:muscle`, `report:visceral-fat`, `report:bmi`, `report:resting-metabolism`
- `config` (or `profile`) → `bodylog config show`

**Default pull for `--all`:**
In addition to current nutlog + repslog defaults, also pull:
- `measurement list` for the window (primary raw source)
- `report summary` (convenient aggregates + trends)

**Status probe (`data status --probe`):**
- Resolve bodylog bin.
- Perform a lightweight call, e.g. `bodylog --json config show` or `bodylog --json measurement list --since today`.
- Include in the sources table/JSON array exactly like the other two (with `entities` list).

**Error updates:**
- `KrebslogError::UnknownSource` message: `Supported: nutlog, repslog, bodylog`

### 4.3 report energy-balance

Activate the existing placeholder:

```bash
krebslog report energy-balance --since "last 14 days" --include-body-trends
```

When `--include-body-trends` is passed (or always in the future once body data is first-class):
- If bodylog binary is available, fetch `report weight` (or `report summary` + `measurement list`) for the period.
- Include in JSON output a `body` block:
  ```json
  "body": {
    "weight": { "start": 82.7, "end": 82.1, "delta_kg": -0.6, "trend": "down", "avg": 82.5, "count": 3, ... },
    "body_fat_pct": { ... optional ... },
    "skeletal_muscle_pct": { ... },
    "sources": ["bodylog"],
    "notes": "Weight delta used as outcome signal for energy balance interpretation."
  }
  ```
- Human output should render a small summary or table section (comfy-table).
- If bodylog is unavailable or has no data for the window, emit a clear note under `--json` (`"body": {"available": false, "reason": "..."}`) and degrade gracefully.

The flag name `--include-body-trends` remains the user-facing spelling even if implementation also pulls raw measurements.

### 4.4 Other Reports & Images

- `report daily`: If a measurement exists for the exact `--date`, surface the latest known weight / comp values (best-effort join on date).
- `report weekly` / multi-day: Include start/end/avg weight and simple body comp deltas when bodylog data is present.
- `report web`: Add `"bodylog"` to the `sources` array when body data was successfully gathered. Include body KPIs/trends in the data payload and page (progressive disclosure; weight trend sparkline is high value).
- `image full-dashboard` and future composites: Optional body trend strip or callout cards when data available.
- `agent context`: Enrich the bundle with a `body` section (latest measurement + period trends + profile context if present).

### 4.5 telegram

`telegram daily` and `telegram report` should be able to incorporate body delta / latest weight when `--include-body-trends` style logic or future defaults pull it. Keep captions concise.

---

## 5. Data Pulling Layer Implementation Notes

Follow the exact patterns in `src/commands/data.rs` + `src/utils.rs`:

- Add `pull_bodylog_default(...)` called from the `--all` path.
- Add `pull_bodylog_entity(entity, since, until, ...)` modeled on the nutlog/repslog versions.
- Use `resolve_bin(ctx.bodylog_bin.as_deref(), &["bodylog"])`.
- For measurement list: build args `["measurement", "list", "--since", since_str, ... "--until", until_str]`.
- For reports: `["report", "summary", "--since", since_str]` (bodylog reports appear to accept `--since` with the same date forms it accepts elsewhere; pass YYYY-MM-DD).
- For config: `["config", "show"]` (no date args needed).
- Always forward the child's stdout under `--json` (or wrap lightly with success + source metadata for consistency with current nutlog/repslog pulls).
- Dry-run support must show the exact `bodylog --json ...` command that would be run.
- Update the "Pulling from X + Y + bodylog" progress message when `--all`.

Handle the fact that bodylog date parsing is less permissive than krebslog's: the `resolve_period` + `format_date_for_child` path already gives us safe `YYYY-MM-DD` strings.

Add a small helper if needed (e.g. `pull_bodylog_measurement_list` or keep it inside the entity matcher).

---

## 6. Report Data Gathering & Web Report Changes

Current `GatheredData` in report.rs is private and web-focused. For bodylog:

- Extend it (or create a parallel `BodyGathered` / add fields):
  ```rust
  bodylog_available: bool,
  body_measurements: Option<Value>,   // raw list or normalized
  body_summary: Option<Value>,
  body_profile: Option<Value>,        // height + dob
  ```
- Add a `gather_body_data(start, end, ctx)` function (or extend the existing gather_web_data).
- Call it unconditionally for web report (graceful if missing) and conditionally for energy-balance when the flag is set.
- In `build_web_report_data` (and future daily/weekly builders), include bodylog in the `sources` vector.
- Update the HTML template footer and methodology sections to mention bodylog when present (the template currently hardcodes "nutlog + repslog").

For energy-balance (once the real implementation replaces the skeleton), the body data should participate in any "apparent vs. observed" narrative.

---

## 7. Cache & Schema Implications (Future, When Aggregation Lands)

Current state: data pull mostly passthroughs; full daily grain + derived scores are still skeleton.

When caching is implemented:

- Daily aggregates should be able to store (or reference) body measurement values for that exact local date:
  - `weight_kg REAL NULL`
  - `body_fat_pct REAL NULL`
  - `skeletal_muscle_pct REAL NULL`
  - `visceral_fat_level REAL NULL`
  - `bmi REAL NULL`
  - `resting_metabolism_kcal INTEGER NULL`
- Because bodylog measurements are sparse (user may not weigh every day), prefer storing the actual measurement date rows rather than forcing a value on every nutrition/training day.
- Alternative: a lightweight `body_daily(date TEXT PRIMARY KEY, ...)` table in krebslog's cache, populated only on successful body pulls.
- Pull metadata / freshness tracking must now include a bodylog row (source + entity + last_pull timestamp).
- `migrate` will need a new schema version bump when these columns/tables are added. Use simple rusqlite `PRAGMA user_version` or a migrations table (consistent with project guidance).
- `--no-cache` must still force live bodylog calls even if cached body rows exist.

Body data in the cache is **derived from prior live calls** — never the source of truth.

---

## 8. Derived Calculations & Transparency

Body data affects (or provides context for):

- **Energy balance**: Intake (nutlog) vs. estimated expenditure (repslog training + optional resting metabolism from bodylog or profile-derived). Weight delta is an *outcome signal*, not an input to the arithmetic, but should be shown alongside for interpretation.
- **Adaptation / recomposition**: Longer-term trends in muscle vs. fat (useful for `report weekly`, correlations, agent insights).
- **BMR / TDEE hints**: If bodylog supplies resting_metabolism in reports, prefer it. Otherwise fall back to age/height/weight formulas only if the user has opted in via config (future).

Every report JSON that incorporates body data **must** include an `assumptions` or `body_notes` object, e.g.:

```json
"body": {
  "weight_delta_kg": -0.6,
  "period_days": 3,
  "resting_metabolism_source": "bodylog.report (null for this window)",
  "interpretation_note": "Observed weight trend used only for qualitative cross-check of energy balance estimate. Water weight, glycogen, and measurement timing introduce noise."
}
```

---

## 9. Agent & JSON Contract

- `data status --probe` JSON grows a third source entry.
- `data pull` JSON for bodylog entities is the direct child output (or a small `{ "success": true, "source": "bodylog", "entity": "...", "data": <child json> }` wrapper for consistency).
- Report JSON (especially energy-balance with body trends) includes the body block described above + full raw child payloads under a `raw` key where practical (following the web report `raw` pattern).
- `agent context` output must advertise body availability and include a compact body snapshot:
  ```json
  "body": {
    "latest": { "date": "2026-06-07", "weight_kg": 82.1, ... },
    "trends_14d": { "weight_delta_kg": -0.6, "trend": "down", ... },
    "profile": { "height_cm": 175.0, ... }
  }
  ```
- Update AGENTS.md quick-reference examples and the "Common Tasks" table.
- Update agent-usage.md with bodylog pull + report examples.

---

## 10. Documentation & Help Text Updates (Required)

- `cli.rs` clap help text (source list, entity examples, energy-balance flag description).
- `docs/command-reference.md` — global flags table, data pull, report energy-balance, data status.
- `docs/pulling.md` — add bodylog examples and entity list.
- `docs/reporting.md` — flesh out the energy-balance section and note body data in daily/weekly/web.
- `docs/data-model.md` — extend the "Core Daily Grain" section with body fields; document profile usage.
- `docs/getting-started.md`, `docs/agent-usage.md` — add BODYLOG_BIN and body examples.
- `docs/index.md` and others that mention the three tools.
- `AGENTS.md` — already mentions bodylog in several places; ensure the "never open DB" list and data sources description stay accurate. Add bodylog pull examples to the quick reference if space allows.
- `CODING_PRACTICES.md` — the "never open" sentence already covers bodylog; no major change needed unless new patterns emerge.
- `spec/01-spec.md` — update architecture diagram text, command examples, and roadmap item 5 ("Optional bodylog integration...") to "Done / in progress" with pointer to this spec.
- Web report template (minor): update the hardcoded "nutlog + repslog" footer when bodylog contributed data.
- README.md if it duplicates high-level source list.

---

## 11. Error Handling & Availability

- Binary not found for bodylog → treat like the others (clear message suggesting install or `--bodylog-bin`).
- Child non-zero or bad JSON → `ExternalToolFailed` / `JsonParse` (existing variants are sufficient; they already carry the bin name).
- No measurements in window → empty array or `{ "count": 0 }` from child; krebslog surfaces cleanly.
- `include_body_trends` requested but bodylog unavailable → success with explanatory note in the body block (do **not** error the whole report).

All errors under `--json` continue to use the `{ "success": false, "error": "..." }` envelope + non-zero exit.

---

## 12. Phased Implementation Plan (Recommended Order)

**Phase 0 — Planning (this spec)**
- Write and review spec/03-bodylog.md.
- Update AGENTS.md / CODING_PRACTICES if any new patterns are locked in during review.

**Phase 1 — Plumbing (small, high confidence)**
- Add `--bodylog-bin` / `BODYLOG_BIN` to Cli + Context.
- Update `resolve_bin` call sites and `data status` (probe + table/JSON).
- Update `UnknownSource` error and DataAction source parser.
- Add bodylog row to config show (skeleton) output.
- `cargo fmt && cargo clippy -- -D warnings && cargo test`.

**Phase 2 — Data Pull (core contract)**
- Implement `pull_bodylog_default` + `pull_bodylog_entity`.
- Wire into `--all` path and explicit `--source bodylog`.
- Support the main entities (measurement, report:*, config).
- Dry-run, quiet, json passthrough.
- Update `data status` last_pull (when cache lands) and probe to actually call bodylog.
- Add basic integration test coverage using `--bodylog-bin` override (or PATH temp hack with a fake binary that prints known JSON).

**Phase 3 — Activate Body Trends in Reports (user value)**
- Make `report energy-balance --include-body-trends` call bodylog and include structured body data in JSON.
- Human-readable table / notes for the flag.
- Extend `gather_web_data` (or new gather) so web reports see bodylog in sources.
- Light enrichment of daily + weekly skeletons (latest measurement on date, simple deltas).

**Phase 4 — Agent & Broader Surfaces**
- Enrich `agent context` output with body section.
- Update telegram daily/report to optionally surface weight delta.
- Image dashboard callouts (low priority).

**Phase 5 — Cache + Full Aggregation**
- Schema migration for body columns or dedicated body table in krebslog cache.
- Store/reuse body data on pull when `--no-cache` is not used.
- Use cached body values inside report builders when the window is covered.
- Update `migrate`, `cache info`, `data status`.

**Phase 6 — Polish, Docs, Tests**
- Fill every doc listed in section 10.
- Add examples to AGENTS.md and agent-usage.md.
- More CLI integration tests (assert_cmd) for the new source and the energy-balance flag.
- Update spec/01-spec.md status.
- End-to-end manual verification: status probe, pull --all, energy-balance with trends, web report, agent context, absence of bodylog binary.

---

## 13. Success Criteria

Before considering the feature complete:

- `krebslog --json data status --probe` lists bodylog with `available: true` (when installed) and performs a successful probe.
- `krebslog --json data pull --all --period "last 14 days"` produces bodylog measurement + summary JSON in the stream.
- Explicit `krebslog --json data pull --source bodylog --entity measurement --since "last 7 days"` works and returns the real array shape.
- `krebslog --json report energy-balance --include-body-trends --since "last 14 days"` contains a `body` object with weight stats derived from bodylog (or a clear `available: false`).
- The same command without bodylog binary present still succeeds and notes the absence.
- Web report generation includes bodylog in `meta.sources` when data was pulled.
- `krebslog --json agent context --since "last 30 days"` contains a `body` key (even if empty object or `{"available": false}`).
- All new code passes `cargo fmt`, `cargo clippy -- -D warnings`, and existing + new tests.
- No code path ever constructs a direct rusqlite connection to a bodylog database file.
- AGENTS.md, command-reference.md, pulling.md, reporting.md, and data-model.md are updated and consistent with the implemented behavior.
- The feature feels boring and predictable — exactly like adding a third sibling source should.

---

## Appendix A: Example Invocations (After Implementation)

```bash
# Discovery
krebslog --json data status --probe

# Pull everything recent (now includes body)
krebslog --json data pull --all --period "last 30 days"

# Explicit body pulls
krebslog --json data pull --source bodylog --entity measurement --since "last 14 days"
krebslog --json data pull --source bodylog --entity report:summary --since "2026-06-01"

# Energy balance with outcomes
krebslog --json report energy-balance --since "last 14 days" --include-body-trends

# Agent bundle (will contain body trends when present)
krebslog --json agent context --for hermes --since "last 30 days"
```

---

## Appendix B: Raw Bodylog Examples (Captured During Planning)

See tool output in the planning session for the exact arrays and report objects used to derive the shapes above. These are checked into the repo only as illustrative data; the live binary is the contract.

---

**This specification is the authoritative plan for bodylog integration.** Implementation should follow the phases, respect every constraint in section 2, and keep the "live child CLI only" rule absolute. After each phase, update the living docs (especially command help text, AGENTS.md, and the data-model/reporting/pulling documents) so agents and humans always have accurate ground truth.

When the work is complete, mark the corresponding item in spec/01-spec.md roadmap as delivered and reference this document.