# Implementation Report: krebslog v0 skeleton (per spec/01-spec.md)

**Date:** 2026-06 (post initial implementation and GitHub publish)  
**Feature:** Complete core CLI skeleton for `krebslog` — Metabolic Reports, Krebs Cycle Visualizer & Redox Insights. CLI-first, local-only, LLM-agent primary interface.  
**Spec:** `spec/01-spec.md` (the full authoritative specification).  
**Author/Context:** Implemented via Grok 4.3 interactive sessions in `/home/aner/rust/krebs-dashboard`. Started from a near-empty directory containing only `spec/01-spec.md`.  
**Related:** Follows the exact patterns mandated by the project's own [AGENTS.md](../AGENTS.md) and the shared coding practices from sibling tools `nutlog`, `repslog`, and `bodylog`. Public GitHub repo: https://github.com/argent0/krebslog

## 1. Overview and Motivation

`krebslog` is a deliberately simple, local, CLI-first Rust tool that aggregates data from `nutlog` (nutrition) and `repslog` (workouts) — and optionally future `bodylog` — into rich metabolic reports focused on the Krebs (TCA) cycle, energy flux, training load, antioxidant/redox balance, and adaptation outcomes.

**Strict architectural invariant** (repeated throughout the spec and AGENTS.md):
- **Zero direct database access** to source tools. All data is pulled exclusively by calling the other CLIs with `--json` (via `std::process::Command`).
- Everything is local-first, offline, single-user, no cloud.
- `--json` is the primary machine/LLM-agent interface.
- Flexible natural-language dates.
- Optional lightweight local SQLite cache (fully disableable with `--no-cache`).
- First-class support for images (for Telegram/Hermes) and agent context bundles.

Prior state:
- Only the spec file existed in the workspace.
- No Cargo.toml, no source, no tests, no docs, no packaging.

The goal of the session was to deliver a working, tested, lint-clean foundation that:
- Implements the full command surface from the spec (data, report, image, telegram, agent, config, migrate, cache groups + globals).
- Delivers a **functional data ingestion layer** (the most critical part of Phase 1).
- Produces extensive documentation matching the style and depth of `bodylog/docs/`.
- Strictly obeys [AGENTS.md](../AGENTS.md) and [CODING_PRACTICES.md](../CODING_PRACTICES.md).
- Creates a public GitHub repository, commits, and pushes.

Roadmap alignment (from spec section 10):
- ✅ Phase 1: Core CLI skeleton + data pulling from nutlog + repslog + JSON output (primary focus)
- Stubs in place for later phases (reports, images, Telegram, agent context, cache wiring).

## 2. Design Decisions

### Runtime and Dependencies
- **rusqlite** (sync, like nutlog) for the optional cache. Rationale: "Simplicity first", no need for async (tokio/sqlx) in a local CLI. Cache is completely optional and bypassable.
- **clap v4 derive** for the entire command surface (global flags + nested groups).
- **chrono** for flexible date parsing (extended beyond nutlog to support "last 7 days", "last monday", "this week", etc.).
- **serde/serde_json**, **comfy-table**, **colored**, **anyhow** + **thiserror**, **directories** — minimal set matching the ecosystem.
- Image generation (`plotters` + custom SVG) is explicitly planned behind a feature flag (per spec).

No unnecessary abstractions. Clear separation of layers (as required by the spec): data pulling, (future) aggregation engine, report generators, image renderers, output formatters.

### Architecture Highlights
- `src/cli.rs`: Complete clap definition matching the spec's command structure exactly.
- `src/context.rs`: Small `Context` struct holding globals (`json`, `quiet`, `no_cache`, `db`, bin overrides). Eliminates repetitive 6-7 argument lists and follows "simplicity".
- `src/utils.rs`: Flexible date parser + `run_external_json` + `resolve_bin` (with PATH + common location fallbacks + overrides). No external "which" crate (minimal deps).
- `src/commands/data.rs`: Fully working `data pull` (single source or `--all`, specific entities or defaults, `--dry-run`, flexible periods) and `data status --probe`. Invokes real `nutlog --json` and `repslog --json` binaries and forwards their output.
- `src/error.rs`: `KrebslogError` with thiserror + consistent `{"success": false, "error": "..."}` shape.
- `src/main.rs`: Thin dispatcher using the Context.
- All other command groups (`report`, `image`, `telegram`, `agent`, etc.) have full clap surfaces + stub handlers that emit proper JSON success shapes (or human text) with "skeleton" notes. This allows agents to discover the full intended interface immediately.

### Error Handling & Coding Practices
- No bare `.unwrap()` or `.expect()` in normal paths (replaced with `.expect("invariant: ...")` with clear explanations where invariants hold).
- `thiserror` for domain errors, `anyhow` only at the binary boundary.
- `cargo fmt && cargo clippy -- -D warnings` clean.
- Unit tests added for pure logic (date parser) in `utils.rs`.
- Followed every rule in the adapted CODING_PRACTICES.md and AGENTS.md.

### Documentation
- Created **extensive `docs/`** exactly modeled on `bodylog/docs/` (and the expectations listed in the krebslog spec):
  - `index.md`, `getting-started.md`, `installation.md`, `command-reference.md` (detailed)
  - `data-model.md`, `pulling.md` (ingestion details)
  - `reporting.md`, `image-generation.md`, `telegram-integration.md`
  - `agent-usage.md`, `troubleshooting.md`
- All files are cross-linked, include tables, runnable examples (human + `--json`), agent notes, and "See Also" sections.
- Updated top-level `AGENTS.md` and `CODING_PRACTICES.md` to match the nutlog reference style while being krebslog-specific.
- `README.md` and `spec/` preserved.

### Packaging
- Added `PKGBUILD` (modeled exactly on nutlog/bodylog) that:
  - Uses dynamic `pkgver()` (Cargo version + git rev count + short SHA)
  - Builds with `cargo build --release --locked`
  - Installs binary + full docs set (including `docs/`) to `/usr/share/doc/krebslog/`
  - Declares practical `optdepends` on `nutlog` and `repslog`
- `cargo install --path .` also works (per spec).

### GitHub & Version Control
- Initialized clean git history.
- Created public repo `argent0/krebslog` via `gh repo create ... --public`.
- Committed 35 files (~5k insertions) with a detailed message.
- Pushed successfully. Remote is `origin` (ssh).

## 3. What Was Implemented vs. Stubbed

**Fully working (Phase 1 complete):**
- Global flags and command parser
- `data pull` (all forms, live child CLI calls, date handling, --dry-run, --probe)
- `data status`
- Error shapes and JSON contract
- Flexible date parser (richer than nutlog)
- Context passing and clean internal structure
- All documentation
- PKGBUILD + build hygiene

**Command surfaces complete, behavior stubbed (ready for next phases):**
- All `report` subcommands
- All `image` subcommands (with --output, themes, layouts)
- `telegram` commands
- `agent` commands (context, tune, skills export)
- `config`, `migrate`, `cache` groups

The stubs correctly emit the expected JSON success shapes so LLM agents can already discover and plan against the full intended interface.

**Not yet implemented (per roadmap):**
- Actual aggregation engine and daily grain storage in the cache
- Derived score calculations (Krebs flux proxy, redox, etc.) with transparent formulas in output
- Real image rendering (plotters + SVG)
- Full Telegram caption generation
- Agent context bundle generation
- Cache wiring for reports (currently data pull prints live results; cache is scaffolded but mostly dormant)

## 4. Adherence to Project Rules

- **AGENTS.md**: Followed "How to Work as an Agent" sections (exploration via rg/cargo, making changes with clap derive + --json, code quality, CLI design, DB rules, testing, docs). The document itself was updated.
- **CODING_PRACTICES.md**: All 9 principles respected (LLM-friendly first, simplicity, fmt/clippy, error handling, DB rules — especially "never touch source DBs", testing, docs, feature flags for images, layer separation).
- **Spec fidelity**: 100% on command structure, globals, data pulling philosophy, date handling, JSON shapes, packaging, docs expectations, and Phase 1 scope.
- No cloud, no direct DB reads of sources, minimal deps, etc.

## 5. Next Steps (Roadmap)

Per spec section 10:
1. (Current) Core skeleton + data pulling ✅
2. Basic daily/weekly text reports + simple charts
3. Full Krebs cycle + antioxidant/redox image generation
4. Telegram-ready formatting + agent context/tuning commands
5. Optional bodylog integration + advanced correlations
6. Polish, docs, packaging (PKGBUILD is already present)

Immediate priorities would be wiring the cache for daily aggregates, implementing the report handlers with real derivation logic (and exposing formulas), then the image module.

## 6. Verification

- `cargo fmt && cargo clippy -- -D warnings` clean
- `cargo test` (4 date parser tests + any others) pass
- Live smoke tests: `data status --probe` and `data pull` successfully invoke real system nutlog/repslog and surface their JSON.
- GitHub repo created and pushed: https://github.com/argent0/krebslog
- All sibling project patterns followed (docs style, PKGBUILD, reports/ folder + implementation report, AGENTS/CODING_PRACTICES, etc.)

This session produced a solid, agent-ready foundation that can be extended exactly as the spec envisions.

---

*Report written to `reports/implementation-report-initial.md` as part of establishing the same documentation and reporting conventions used by nutlog, repslog, and bodylog.*