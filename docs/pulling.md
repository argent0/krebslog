# Data Ingestion / Pulling

The `data` command group is the ingestion layer of krebslog. It is the **only** supported way for krebslog to obtain nutrition and training information.

## Philosophy

- krebslog never opens nutlog.db, repslog.db, or bodylog.db directly.
- All data freshness ultimately comes from executing the sibling CLIs with `--json`.
- The optional local cache exists only for speed and offline convenience; it can be bypassed at any time with `--no-cache`.

This design keeps the source of truth in the tools where the user actually logs data, while giving krebslog a clean, auditable view.

## `data pull`

### Basic Forms

```bash
# Pull a single entity from one source
krebslog data pull --source nutlog --entity consumption --since "last 30 days"

# Pull from repslog
krebslog data pull --source repslog --entity workout --since "2026-05-01"

# Pull from bodylog (weight / body comp)
krebslog data pull --source bodylog --entity measurement --since "last 14 days"
krebslog data pull --source bodylog --entity report:summary --since "last 7 days"

# The convenient "I want everything recent" form
krebslog data pull --all --period 90d
krebslog --json data pull --all --period "last 14 days"
```

`--all` currently pulls a curated default set:
- From nutlog: `consumption` list + `report nutrition`
- From repslog: `workout` list + `stats summary` (and volume when useful)
- From bodylog: `measurement` list + `report summary` (weight/body comp trends)

### Flexible Windows

All the usual date syntax works for `--since`, `--until`, and `--period`:

- `today`, `last 7 days`, `last monday`
- `2026-06-01`
- `30d`, `last 30 days`

`--period` is especially convenient with `--all`.

### Dry Run & Debugging

```bash
krebslog data pull --all --period "last 7 days" --dry-run
```

Shows exactly which child commands would be executed without actually calling them or touching the cache.

### What Actually Happens (Current Implementation)

1. krebslog resolves the effective `--since` / `--until` using its flexible date parser.
2. It locates the `nutlog` and/or `repslog` binaries (PATH + fallbacks + overrides).
3. It spawns the child with `--json` plus the appropriate subcommand and date flags.
4. It captures stdout, prints it (so you get the raw child data under `--json`), and (in the future) stores normalized daily aggregates + derived scores into its own cache.
5. Non-zero exits or unparseable JSON from a child are turned into clear krebslog errors.

Because the child's JSON is forwarded (or lightly normalized), agents that want the raw transaction-level detail can still get it by calling nutlog/repslog directly or by looking at what `data pull` surfaced.

## `data status`

```bash
krebslog data status
krebslog data status --probe
krebslog --json data status --probe
```

- Shows the last known successful pull time per source/entity (when caching is active).
- `--probe` actually executes a lightweight live call (`nutlog consumption list --since today` and `repslog stats summary`) to verify that the binaries are present, respond to `--json`, and are returning plausible data.

This is the first command most people and agents should run after installing krebslog.

## Caching Behavior (Future)

Once the cache is wired:

- Successful pulls will upsert into daily aggregate rows.
- Subsequent `report` commands can serve from the cache when the requested window is fully covered and `--no-cache` was not passed.
- `data status` will show real timestamps.
- `cache clear` and `cache info` become useful.
- `migrate` will manage schema versions for the cache.

Even with the cache enabled, any command can force a fresh pull by using a recent enough `--since` window or by combining with explicit `data pull` beforehand.

## Error Cases Handled Gracefully

- Child binary not found → clear message suggesting installation or `--*-bin` override.
- Child returns non-zero → the stderr (or a summary) is surfaced.
- Child produces invalid JSON → krebslog reports a parse error with context.
- Date range contains no data → the child usually returns an empty array; krebslog surfaces it cleanly.

## Relationship to Other Commands

- `report *` and `image *` will (when implemented) internally trigger or reuse pulls when the requested data is not present in cache.
- Agents often do an explicit `data pull --all ...` at the start of a session or on a schedule, then request multiple reports and context bundles from the now-warm cache.
- `telegram` and `agent context` commands ultimately depend on the same ingestion layer.

## See Also

- [Command Reference](command-reference.md) — every flag on `data pull` and `data status`.
- [Data Model](data-model.md) — what gets stored after a pull succeeds.
- [Getting Started](getting-started.md) — the practical first-pull workflow.
- `src/commands/data.rs` and `src/utils.rs` in the source tree (the actual child invocation and date handling logic).
