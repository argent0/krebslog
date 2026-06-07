# Telegram Integration

`krebslog` produces content that is *ready to post* to Telegram (channels, groups, or saved messages). It does **not** contain any Telegram API client or bot code — that is deliberately left to a separate bot, script, or your Hermes agent.

## The Two Main Telegram Commands

### `telegram daily`

```bash
krebslog telegram daily --date today
krebslog --json telegram daily --date today --with-json
```

Intended behavior:

- Prints a nicely formatted MarkdownV2 caption (bold metrics, emoji, bullet lists, short insights)
- Includes (or references) the path to a generated image (daily dashboard or krebs-flux mini)
- With `--with-json`, also emits a machine-readable block that a bot can use for further processing or alt text

### `telegram report`

```bash
krebslog telegram report --type redox --period 7d --post-ready
krebslog --json telegram report --type krebs --period 14d
```

`--type` can be `daily`, `redox`, `krebs`, `energy`, etc.

`--post-ready` suppresses extra logging so a bot can capture stdout cleanly.

## Typical Bot / Agent Flow

1. Call `krebslog --json telegram daily --date today`
2. Parse the JSON (or the structured text) to extract:
   - `caption` (MarkdownV2)
   - `image_path`
   - Optional extra `data` for logging or decision making
3. Use your Telegram library to send the photo + caption (parse_mode = MarkdownV2)
4. Optionally archive the image + caption to a "posted" directory

Because the heavy lifting (data pull, aggregation, image rendering, caption composition) lives in krebslog, the Telegram-side code can stay tiny and reliable.

## MarkdownV2 Notes

Krebslog takes care to produce valid MarkdownV2:

- Bold with `*bold*` or `**bold**` (depending on the exact rules the library expects)
- Lists with `- ` or `• `
- Emoji used for scannability
- Escaping of characters that have special meaning in MarkdownV2 when they appear in free text (numbers, parentheses in formulas, etc.)

If you ever see parsing errors on the Telegram side, the `--json` output of the `telegram` commands contains the raw caption so you can debug escaping.

## Image + Caption Pairing

Images generated for Telegram use layouts and aspect ratios that look good in the Telegram mobile and desktop clients (including stories / vertical posts).

Recommended workflow for a daily post:

```bash
krebslog --json telegram daily --date today
# or the explicit image + caption route
krebslog image full-dashboard --date today --layout telegram --output /tmp/daily.png
# then construct or retrieve the caption via report/telegram daily --with-json
```

## Directory Watching Pattern

Many users run a small watcher (inotify, systemd path unit, or a cron job) that:

- Watches `~/reports/krebslog/`
- When a new `*.png` appears that matches the "ready for Telegram" naming convention, calls `krebslog telegram ...` (or reads a sidecar `.caption.md` file) and posts it.

This keeps the actual posting logic completely decoupled from krebslog.

## Agent-Driven Posting (Hermes)

A Hermes-style agent can decide on its own when to generate and post content:

```bash
# Agent decides a good redox summary is worth sharing
krebslog --json telegram report --type redox --period 7d --post-ready | \
  your-telegram-poster --channel @your-metabolic-log
```

Or the agent can first ask for the context bundle, reason about it, then request specific images and the matching caption.

## Configuration

Future `config` keys relevant to Telegram:

- `telegram.parse_mode` (MarkdownV2 is the default and strongly recommended)
- `reports.dir` (where images are written)
- Per-layout aspect ratio / sizing hints

See `krebslog config --help` once the config system is populated.

## See Also

- [Image Generation](image-generation.md) — the visuals that accompany the captions.
- [Telegram commands in the Command Reference](command-reference.md)
- Top-level [AGENTS.md](../AGENTS.md) — examples of Hermes requesting krebslog output on demand.
- The original vision in [spec/01-spec.md](../spec/01-spec.md) section 6.
