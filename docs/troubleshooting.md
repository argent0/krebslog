# Troubleshooting

## "nutlog: command not found" or "repslog: command not found"

krebslog itself runs, but it cannot locate its data sources.

Solutions (in order):

1. `which nutlog repslog` — are they installed and on PATH?
2. Use the explicit overrides for this session:

   ```bash
   krebslog --nutlog-bin /path/to/nutlog --repslog-bin /path/to/repslog data status --probe
   ```

3. Set the environment variables persistently:

   ```bash
   export NUTLOG_BIN=/path/to/nutlog
   export REPSLOG_BIN=/path/to/repslog
   ```

4. After installing via package, make sure `/usr/bin` (or wherever the package put them) is on PATH.

Run `krebslog data status --probe` again after fixing the paths.

## "unrecognized date format"

The date parser supports a large but explicit set of forms (see [data-model.md](data-model.md)).

Recommended forms for reliability:

- `today`, `yesterday`
- `2026-06-07` (ISO-8601)
- `last 14 days`, `last monday`
- `3 days ago`

If you are an agent running on a different machine or in a container, prefer passing explicit `YYYY-MM-DD` rather than relative phrases.

## Data pull returns empty results or very old data

- The window you asked for may genuinely have no data in nutlog or repslog.
- Verify with the source tools directly:

  ```bash
  nutlog --json consumption list --since "last 30 days" | head
  repslog --json workout list --days 30
  ```

- You may be looking at a test database (`--db /tmp/...`) instead of your real one.
- Use `data status --probe` to confirm krebslog can still talk to the children.

## Reports or images previously showed "skeleton" notes (historical)

All core `report`, `image`, `telegram`, `agent`, and `config` surfaces now produce real output (using the shared gather/compute pipeline for nutrition+training+body, pure-Rust SVG for images, persisted config, and real Telegram/ agent bundles).

If you see an outdated note it is likely from an old binary or cached doc. Rebuild (`cargo build` or reinstall) and re-run. The `data pull` path has always been the most complete layer; reports and visuals are now fully wired on top of it.

You can (and should) exercise the full CLI + `--json` contract for agents.

## Cache / database problems

- "failed to open database" → the directory must be writable. krebslog creates the XDG directory on first use.
- Using `--db /some/path` — the parent directory must exist or be creatable.
- Want a completely clean slate? `krebslog cache clear --force` or simply delete the `.db` file (your source data in nutlog/repslog is unaffected).

## JSON from krebslog vs. child tools

When you do `krebslog --json data pull ...`, you will usually see the raw JSON array or object that the child tool (`nutlog` or `repslog`) produced.

High-level krebslog commands (`report`, `agent context`, `telegram`, etc.) wrap things in the `{"success": true, ...}` envelope or return richer synthesized objects.

On error paths under `--json`, krebslog still prints the error envelope to **stdout** (not stderr) and exits non-zero. Agents should read stdout and check the exit code.

## Timezone / "last monday" surprises

All relative dates are evaluated using the local calendar of the machine running `krebslog` at the moment the command executes.

- If you run krebslog on a server in UTC while you live in another zone, "today" will be the server's today.
- "last monday" is always "the most recent Monday before now on this machine's calendar".

For agents: when in doubt, pass explicit `YYYY-MM-DD`.

## Images not appearing or wrong size

Image generation is not fully implemented in the current skeleton. Once it lands:

- Make sure the output directory (`~/reports/krebslog` by default, or configured) is writable.
- Use `--output /absolute/path.png` for full control.
- Different `--layout` values (telegram, square, wide) affect aspect ratio.

## Rebuilding after source changes

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo install --path .
```

Or for a packaged build:

```bash
makepkg -fsi
```

## Debugging a specific command

```bash
# Use an isolated database
DB=/tmp/debug-krebs.db

# See exactly what child commands would run
krebslog --db $DB data pull --all --period "last 7 days" --dry-run

# Capture full JSON + see exit code
krebslog --json --db $DB report daily --date today 2>&1 | cat; echo "EXIT=$?"

# Inspect what krebslog itself would store (future)
krebslog --db $DB cache info
```

You can also run the child tools directly with the same arguments krebslog would use (visible in `--dry-run` output or by reading the source in `src/commands/data.rs`).

## Still Stuck?

1. Re-read the command's `--help`.
2. Read the relevant section in [command-reference.md](command-reference.md).
3. Try the operation with `--json --db /tmp/fresh.db` to get a clean, reproducible failure.
4. Verify the child tools (`nutlog`, `repslog`) work in isolation with `--json`.
5. Look at the source — the data pulling layer (`src/commands/data.rs` + `src/utils.rs`) is intentionally kept straightforward.

The design goal is that a human (or an agent reading the docs + `--help`) should be able to predict what will happen.

When reporting issues, include:

- `krebslog --version`
- The exact command + global flags
- The full stdout/stderr + exit code (especially under `--json`)
- Whether you used a custom `--db`
- Output of `krebslog data status --probe`
- OS / distribution
- Versions of nutlog and repslog (`nutlog --version`, `repslog --version`)
