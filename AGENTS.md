# agents.md — Guidelines for LLM Agents Working on krebslog

This document helps AI agents (and humans working with them) collaborate effectively on the `krebslog` project.

## Project Philosophy

`krebslog` is a **single-user, local-first, LLM-agent-first** CLI tool that turns data from `nutlog` + `repslog` + `bodylog` (optional but fully supported — see spec/03-bodylog.md) into rich metabolic reports centered on the Krebs (TCA) cycle, energy flux, training load, nutrition inputs, antioxidant/redox balance, and body adaptation outcomes.

Key principles:
- **Simplicity first** — Prefer boring, maintainable solutions over clever abstractions.
- **Agent-friendly by design** — The CLI is built to be easily used by LLM agents (Hermes and similar) via tools/skills. `--json` is the primary contract.
- **Predictability > magic** — Consistent command structure, clear output, and explicit behavior. All derived scores (Krebs flux proxies, redox balance, etc.) must expose the formulas/assumptions used.
- **Live CLI calls only** — All source data is obtained exclusively by spawning `nutlog --json ...` and `repslog --json ...`. **Zero direct database access** to the upstream tools' SQLite files. This is a hard architectural invariant.
- **Optional cache, fully bypassable** — krebslog may maintain its own lightweight rusqlite cache for speed, but `--no-cache` (or equivalent) must always produce fresh results from the child CLIs.

## How to Work as an Agent on This Project

### 1. Exploration
- Start by reading `CODING_PRACTICES.md`, `AGENTS.md`, the main `README.md`, and `spec/01-spec.md`.
- Use `cargo tree`, `cargo metadata --format-version 1`, and `rg` / `grep` for code exploration.
- Prefer reading source files directly rather than relying only on summaries.
- When in doubt about architecture, look at existing commands for patterns — especially the data pulling layer.
  - `src/cli.rs` — the complete command surface and flag design.
  - `src/commands/data.rs` + `src/utils.rs` — the critical "call child CLIs with --json, parse, forward" logic and the flexible date parser.
  - `src/error.rs` — the domain error types and how they become the consistent `{"success": false, "error": "..."}` shape.
  - `src/commands/report.rs`, `src/commands/image.rs`, etc. — how higher-level groups are implemented.
- Study the real `nutlog`, `repslog`, and `bodylog` CLIs (their `--help` and `--json` output shapes) because krebslog is a consumer of them. See spec/03-bodylog.md for the bodylog contract.

### 2. Making Changes
- Follow the command pattern: `krebslog <group> <action> [flags]`
  - Groups: `data`, `report`, `image`, `telegram`, `agent`, `config`, `migrate`, `cache`.
  - Examples: `data pull`, `report daily`, `report krebs-flux`, `image krebs-cycle`, `agent context`, `telegram daily`.
- Always support `--json` output for commands that return data.
- Use `clap` derive macros for all new commands.
- Add proper help text — this is the primary documentation for agents.
- Update `AGENTS.md` and `CODING_PRACTICES.md` when you introduce new patterns.
- When adding or changing derived metabolic calculations (flux, redox, energy balance, etc.), make the formulas/assumptions visible in the JSON output and document them.

### 3. Code Quality
- Run `cargo fmt` and `cargo clippy -- -D warnings` before proposing changes.
- Use the configurations in `rustfmt.toml` and `clippy.toml`.
- Prefer `thiserror` for domain errors (`KrebslogError`) and `anyhow` only at the binary boundary.
- Avoid `unwrap()`, `expect()`, and `panic!()` in normal execution paths.

### 4. CLI Design Rules (Critical for Agent Usability)
When adding or modifying commands:
- Keep the `krebslog <group> <action>` structure (higher-level groups than nutlog's entity model, but still predictable).
- Use `--json` as the standard way for agents to get structured data. Human-readable output (when not `--json`) should use `comfy-table` for clean tabular results.
- Date inputs must accept the rich flexible formats already supported (and extended beyond nutlog): `today`, `yesterday`, `last 7 days`, `last monday`, `this week`, `2026-06-07`, `N days ago`, etc. The parser lives in `utils.rs`.
- All derived values (Krebs flux proxy, redox/oxidative load, antioxidant adequacy, energy balance, etc.) must be transparent — include the inputs, weights, and assumptions in JSON responses.
- Error messages must be actionable and not assume human context only. Under `--json`, errors use the `{"success": false, "error": "..."}` envelope and the process exits non-zero.
- The "data comes only from live child `--json` calls" rule is sacred. Do not add direct SQLite reads of nutlog or repslog databases.

### 5. Database & Schema Changes
- krebslog's optional local cache (at `~/.local/share/krebslog/krebslog.db` or user `--db`) is the *only* database it owns.
- When the cache gains schema (daily aggregates, pull timestamps, config, etc.), use versioned migration files (similar to repslog's `migrations/`) and simple rusqlite patterns (matching nutlog's style).
- **Never** open or query the SQLite files belonging to nutlog, repslog, or bodylog. All freshness comes from live CLI subprocess calls (or previously cached results of those calls). Bodylog integration is specified in spec/03-bodylog.md.
- Timestamps are stored in UTC.
- The cache must remain completely optional and disableable (`--no-cache`).

### 6. Testing
- Add tests for new functionality (especially command parsing, JSON output shapes, date parsing, and the data-pulling layer).
- Use `assert_cmd` + `predicates` for CLI integration tests (see dev-dependencies in Cargo.toml).
- For data pull tests, prefer overriding the binary path (`--nutlog-bin`, `--repslog-bin`) or temporarily adjusting `PATH` to point at test doubles rather than relying on the real tools.
- Keep tests fast and focused. This is a local CLI tool.

### 7. Documentation
- Update command help text as the source of truth.
- Keep `CODING_PRACTICES.md` and this file up to date.
- Maintain the `docs/` directory (data-model.md, command-reference.md, agent-usage.md, image-generation.md, telegram-integration.md, etc.).
- When adding significant features (new reports, image types, agent context bundles), consider adding runnable examples.
- The `spec/01-spec.md` and roadmap should stay in sync with implemented behavior.

## Common Tasks for Agents

| Task                                      | Recommended Approach                                                                 |
|-------------------------------------------|--------------------------------------------------------------------------------------|
| Add a new entity to `data pull`           | Extend the source/entity mapping in `commands/data.rs`; handle args for the child CLI (includes bodylog since spec/03-bodylog.md); ensure `--json` passthrough or normalized output. |
| Add a new report (daily/weekly/flux/...)  | Add variant to `ReportAction` in `cli.rs`; implement in `commands/report.rs`; surface formulas in JSON. New integrated example: `report krebs-status` (body-validated flux + adaptation per spec/04). |
| Add `report web` interactive HTML        | New `Web` variant + full generator in `report.rs` (Tailwind CDN + vanilla JS + hand-crafted SVG cycle + bottom sheets). Single-file or folder output. See spec/02-web-report.md. |
| Add or extend an image generator          | Add to `ImageAction`; implement behind the images feature flag when heavy deps (plotters) are involved; support `--output` and theme flags. |
| Improve or extend date parsing            | Work in `utils.rs::parse_flexible_date`; add cases for new natural language forms; add tests; keep child-tool date formatting as `YYYY-MM-DD`. |
| Add Telegram-ready output for a report    | Implement in `commands/telegram.rs`; produce MarkdownV2 + image path; keep a clean `--json` block for bots. |
| Add or expand `agent context` / skills    | Work in `commands/agent.rs`; keep the output compact and up-to-date (nutrition + training + redox + insights); update docs/agent-usage.md and AGENTS.md examples. |
| Introduce or evolve the optional cache    | Add tables/migrations under rusqlite; expose `--no-cache`; update `data status` and `cache` commands; never bypass the live-CLI contract. |
| Change output format (human vs JSON)      | Support both modes for every new command; human uses comfy-table/colored when appropriate. |

## Things Agents Should Avoid

- Introducing any code that opens nutlog.db, repslog.db, or bodylog.db directly.
- Adding heavy crates (plotters, etc.) to `[dependencies]` instead of behind a feature flag (e.g. `images`).
- Breaking the `data pull` contract — all source data must come from child `--json` subprocess calls (or cache of prior such calls).
- Using unclear abbreviations in command or flag names.
- Making derived metabolic scores (flux, redox, etc.) opaque — formulas and assumptions must be visible under `--json`.
- Introducing async runtimes (tokio, sqlx async) in the core path unless there is a compelling reason and it is isolated; the current design follows nutlog's simpler synchronous style.
- Adding features that make the tool harder for other agents to use (e.g. interactive prompts, hidden side effects, inconsistent success shapes).

## Quick Reference

```bash
# Format & lint
cargo fmt
cargo clippy -- -D warnings

# Run tests
cargo test

# Typical development flow
cargo run -- --json data status --probe
cargo run -- --json data pull --all --period "last 30 days"
cargo run -- --json report daily --date today
cargo run -- --json report krebs-status --since "last 14 days"   # body-validated flux + adaptation
cargo run -- --json report web --period "last 30 days" --output ./reports/
cargo run -- --json agent context --for hermes --since "last 30 days"
```

---

**Goal**: Make `krebslog` a joy for both humans *and* LLM agents to use (at runtime) and to extend (as contributors).

Update this file whenever the project's agent-interaction patterns or development practices evolve. Keep the command examples and JSON contract notes accurate so that Hermes-style agents and other LLM users can rely on this document.
