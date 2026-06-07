# Agent & JSON / Scripting Usage

`krebslog` was built from the ground up as a first-class tool for LLM agents (Hermes and similar) that need a unified metabolic picture.

The `--json` flag (global) is the primary contract. Human output is a convenience, not the interface agents should rely on.

## Core Contract for Agents

1. **Always pass `--json`** when you need structured data or reliable signaling.
2. **Check the exit code** — non-zero always means failure, even if JSON was emitted.
3. **Read stdout for the payload** on both success and error paths (errors are deliberately printed to stdout under `--json` so a single pipe capture works).
4. **Success envelope** (typical for high-level operations):

   ```json
   { "success": true, "id"?: number, "message": "...", ... }
   ```

5. **Error envelope** (stdout + non-zero exit):

   ```json
   { "success": false, "error": "..." }
   ```

6. Global flags that agents love:
   - `--db /explicit/path` — never rely on the default XDG location in automated flows.
   - `--no-cache` — force live calls to nutlog/repslog when freshness matters.
   - `--quiet` — reduce noise when you only care about the JSON.
   - `--nutlog-bin` / `--repslog-bin` (or the corresponding env vars) for hermetic testing.

## Recommended Invocation Pattern

```bash
DB="$HOME/.local/share/krebslog/krebslog.db"

krebslog --json --db "$DB" data pull --all --period "last 30 days"
krebslog --json --db "$DB" report daily --date today --include-image
krebslog --json --db "$DB" agent context --for hermes --since "last 30 days"
```

Capture stdout, `jq` or parse it, inspect `success`.

## Bootstrapping / Discovery

```bash
# What data sources are available right now?
krebslog --json data status --probe

# Pull exactly what you need for a context window
krebslog --json data pull --source nutlog --entity consumption --since "last 14 days"
krebslog --json data pull --source repslog --entity workout --since "last 14 days"
krebslog --json data pull --source repslog --entity "stats:summary" --since "last 14 days"
```

## Daily / Periodic Metabolic Snapshot

```bash
krebslog --json report daily --date today --include-image
krebslog --json report krebs-flux --period 14d
krebslog --json report redox-balance --since "last 7 days"
krebslog --json report energy-balance --since "last 14 days"
krebslog --json report correlations --x nutrition --y training-load --period 30d
```

Under `--json` these will (when fully implemented) include the raw series, the computed stats, and the exact formulas/assumptions that were used for any derived scores.

## Visuals & Telegram Bundles

krebslog is explicitly designed to produce publication-ready images and captions:

```bash
# Generate a Krebs cycle diagram with your personal flux overlays
krebslog image krebs-cycle --date-range "last 30 days" --theme dark --output ./reports/krebs-2026-06-07.png

# Full composite dashboard tuned for Telegram (1080x1920-ish or story-friendly)
krebslog image full-dashboard --date today --layout telegram --output ./reports/dashboard-today.png

# Ready-to-post payload
krebslog --json telegram daily --date today
krebslog --json telegram report --type redox --period 7d --post-ready
```

The `telegram` commands emit a MarkdownV2 caption (with bold, lists, emojis) plus the absolute or relative path to the image. Your bot or Hermes can post the pair directly.

## Rich Agent Context Bundles

The `agent context` command is one of the highest-value entry points:

```bash
krebslog --json agent context \
  --for hermes \
  --since "last 30 days" \
  --output /tmp/hermes-metabolic-context.json
```

Typical contents (when complete):
- Nutrition summary for the window
- Training load, volume, effective reps, zone distribution
- Krebs flux and redox balance scores with explanations
- Key correlations and trend signals
- References to the most recent images / reports

Agents can load this file into their system prompt or RAG store.

You can also ask krebslog to generate or refine prompt fragments:

```bash
krebslog agent tune --generate-prompts --focus "redox and recovery"
```

## Combining with nutlog and repslog Directly

krebslog is an *aggregator*, not a replacement. Sophisticated agents often do this:

1. Pull raw detailed data via nutlog/repslog when they need the full transaction log.
2. Pull the synthesized daily/period view + derived scores via krebslog.
3. Ask krebslog for images or Telegram bundles on demand.

Example mixed flow (pseudo):

```bash
# Detailed consumption for RAG
nutlog --json consumption list --since "last 7 days" > /tmp/consumption.json

# High-level metabolic picture + derived scores
krebslog --json report daily --date today > /tmp/daily.json

# Visual for the human user (or to include in a message)
krebslog image krebs-cycle --date-range "last 7 days" --output /tmp/krebs.png
```

## Date Handling Advice for Agents

- Prefer explicit `YYYY-MM-DD` when the agent's clock may differ from the user's machine.
- "last monday", "this week", etc. are evaluated on the machine running `krebslog`.
- Use `--period "last 30 days"` or explicit `--since`/`--until` for windows.

See [data-model.md](data-model.md) for the full date parser rules.

## Error Handling Pattern (Robust Agents)

```python
# pseudo
result = subprocess.run(
    ["krebslog", "--json", "--db", db, "report", "redox-balance", "--since", "last 7 days"],
    capture_output=True, text=True
)
data = json.loads(result.stdout)
if result.returncode != 0 or not data.get("success", True):
    raise RuntimeError(data.get("error") or "krebslog failed")
```

Always treat non-zero exit as authoritative failure.

## Configuration for Agents

Use the `config` group (or future config file) to set stable defaults for a given user/agent pair:

```bash
krebslog config set reports.dir ~/reports/krebslog
krebslog config set formulas.redox.training_weight 0.6
```

(Exact keys will be documented as the config system is implemented.)

## Skills / Tool Calling

See the top-level [AGENTS.md](../AGENTS.md) for the skill package export command:

```bash
krebslog agent skills export --to ~/.hermes/skills/krebslog
```

This copies curated command examples, JSON schemas, and guidance so an agent framework can natively "know" how to call krebslog.

## Summary of High-Value Commands for Agents

| Goal                              | Recommended Command                                      |
|-----------------------------------|----------------------------------------------------------|
| Discover available data           | `data status --probe`                                    |
| Refresh metabolic picture         | `data pull --all --period "last 30 days"`                |
| Daily briefing                    | `report daily --date today --include-image`              |
| Flux / energy state               | `report krebs-flux`, `report energy-balance`             |
| Recovery / antioxidant balance    | `report redox-balance`                                   |
| Visuals for user or RAG           | `image krebs-cycle`, `image full-dashboard`              |
| Telegram post content             | `telegram daily`, `telegram report`                      |
| Compact context bundle for prompt | `agent context --for hermes --since "last 30 days"`      |
| Tune prompts / few-shot examples  | `agent tune --generate-prompts --focus "..."`            |

See the full [Command Reference](command-reference.md) for every flag.

## See Also

- [Data Model](data-model.md) — exactly what the numbers mean and how they are derived.
- [AGENTS.md](../AGENTS.md) (top level) — development + usage guidelines for agents working on or with krebslog.
- Live `--json` output from the tool is always the most accurate schema reference.
