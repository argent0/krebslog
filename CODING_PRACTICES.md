# Coding Practices for krebslog

This document defines the coding standards and best practices for the `krebslog` project.

## Goals

- Make the codebase pleasant for both humans and LLM agents to read and modify.
- Keep the CLI predictable, well-documented, and easy to script against (especially via `--json`).
- Follow idiomatic Rust while staying pragmatic for a single-user, local, aggregator-style CLI tool.
- Preserve the "live CLI calls only" contract: krebslog never reads nutlog/repslog/bodylog SQLite files directly.

## Core Principles

1. **LLM-Agent Friendly First**
   - Every command that returns data must support `--json` output.
   - Error messages should be clear, actionable, and structured under `--json` as `{ "success": false, "error": "..." }`.
   - Use consistent `group action` (and `group action sub-action`) subcommand pattern (e.g. `data pull`, `report krebs-flux`, `image krebs-cycle`, `agent context`).
   - Prefer explicit over clever.
   - All derived values (Krebs flux proxies, redox balance, energy balance, etc.) must expose the formulas/assumptions used in JSON output.

2. **Simplicity over Cleverness**
   - Single-user, local (optional) SQLite cache tool → avoid unnecessary abstractions.
   - Prefer `thiserror` for domain errors + `anyhow` only at the binary entry point.
   - Use `clap` derive API for commands.
   - Data ingestion happens exclusively via `std::process::Command` + `--json` against sibling tools (nutlog, repslog). Keep that layer simple and isolated (see `commands/data.rs` + `utils.rs`).

3. **Formatting & Style**
   - Run `cargo fmt` before committing (uses the project's `rustfmt.toml` — max width 100).
   - Use 4-space indentation.
   - Keep line lengths reasonable; let the formatter and clippy guide you.

4. **Linting**
   - Run `cargo clippy -- -D warnings` in CI / before proposing changes.
   - See `clippy.toml` for project-specific rules.
   - Avoid `unwrap()`, `expect()`, and `panic!()` in normal execution paths. When an "impossible" case is truly invariant, use `.expect("invariant: ...")` with a clear explanation.

5. **Error Handling**
   - Use `thiserror` for domain errors (`KrebslogError`).
   - Use `anyhow` only at the binary entry point (`main`) for top-level reporting.
   - Never silently ignore errors (no `let _ = ...` that swallows real failures).
   - Commands are responsible for emitting the JSON error shape before returning when `--json` is active.

6. **Database & Data (krebslog's own cache only)**
   - All timestamps stored in UTC.
   - krebslog owns an *optional* lightweight rusqlite cache (default `~/.local/share/krebslog/krebslog.db`, overridable with `--db`, fully disableable with `--no-cache`).
   - When adding cache schema, use simple migration files (rusqlite style, similar to nutlog) + versioned user_version or a migrations table.
   - **Never** open or query the DB files of nutlog, repslog, or bodylog. All source data freshness comes from live subprocess calls (or cache of prior calls).
   - Prefer explicit, simple queries. Keep the cache schema minimal (daily grain aggregates + pull metadata are the core).

7. **Testing**
   - Unit tests for pure logic (date parsing, formula helpers, etc.).
   - Integration tests for CLI commands using `assert_cmd` + `predicates` (already in dev-dependencies).
   - For data-pull paths, use `--nutlog-bin` / `--repslog-bin` (or PATH manipulation) with test doubles so tests don't depend on the real installed tools.
   - Keep tests fast — this is a local CLI, not a web service.

8. **Documentation**
   - Public items (and important internal functions) should have doc comments.
   - CLI help text (in clap definitions) is the primary user/ agent documentation.
   - Detailed docs live in the `docs/` directory (markdown). Keep `AGENTS.md`, `README.md`, `spec/01-spec.md`, and `docs/` in sync.
   - Update `CODING_PRACTICES.md` and `AGENTS.md` when patterns change.
   - Clear layer separation is intentional: data pulling, aggregation/processing, report generators, image renderers, output formatters (text/JSON/telegram). New code should respect these boundaries.

9. **Images & Optional Features**
   - Heavy dependencies for image generation (e.g. `plotters`) must live behind a feature flag (default-disabled or default-enabled per future decision in Cargo.toml). Core functionality must build and run without them.
   - Image output (PNG/SVG) and Telegram-ready bundles are first-class but implemented after the text/JSON report foundation.

## Recommended Tooling

| Tool       | Command                        | When to run          |
|------------|--------------------------------|----------------------|
| rustfmt    | `cargo fmt`                    | Before every commit  |
| Clippy     | `cargo clippy -- -D warnings`  | Before every commit  |
| Tests      | `cargo test`                   | Before every commit  |
| Audit      | `cargo audit`                  | Periodically         |

## Useful Commands for Development

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo build
cargo run -- --json data status --probe
```

## Layer / Architecture Notes (krebslog specific)

- `src/cli.rs` — clap definition of the entire surface. This is the contract.
- `src/main.rs` — thin dispatcher. Build a `Context` (or equivalent) once and pass it down.
- `src/commands/data.rs` + `src/utils.rs` — the data ingestion layer. Subprocess calls + date handling + JSON passthrough/normalization. This is the most critical "do not bypass" area.
- `src/error.rs` — domain errors + the `Result<T>` alias used by command handlers.
- Future: aggregation engine, report generators, image renderers, context builders for agents should live in appropriately named modules/files and stay decoupled from the CLI parsing and from direct source DBs.

## References

- Official Rust Style Guide: https://doc.rust-lang.org/style-guide/
- rustfmt configuration: https://rust-lang.github.io/rustfmt/
- Clippy documentation: https://rust-lang.github.io/rust-clippy/
- The project `AGENTS.md` (read this for agent/LLM collaboration rules).
- `spec/01-spec.md` for the original requirements and philosophy.

---

*This file is part of the krebslog coding practices. It is intentionally very similar to the nutlog/repslog standards because krebslog is meant to feel like a natural sibling in the same ecosystem.*
