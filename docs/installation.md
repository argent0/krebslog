# Installation

## From Source (Cargo)

```bash
git clone https://github.com/argent0/krebslog.git
cd krebslog
cargo install --path .
```

The `krebslog` binary will be placed in `~/.cargo/bin/` (make sure this is on your `$PATH`).

For development / iteration:

```bash
cargo run -- --json data status --probe
cargo run -- --json data pull --all --period "last 14 days"
```

## Runtime Dependencies

`krebslog` has **no heavy runtime dependencies** of its own, but it is useless without its data sources:

- `nutlog` (required for nutrition/consumption data)
- `repslog` (required for training/workout data)

Both must be discoverable:

- In `$PATH`, or
- Via the `--nutlog-bin` / `--repslog-bin` flags, or
- Via the `NUTLOG_BIN` / `REPSLOG_BIN` environment variables.

```bash
which nutlog repslog
krebslog --nutlog-bin /usr/local/bin/nutlog --repslog-bin /usr/local/bin/repslog data status --probe
```

SQLite is statically bundled (via `rusqlite` with the `bundled` feature) so no system `libsqlite3` is required at runtime.

## Arch Linux / AUR (PKGBUILD)

A `PKGBUILD` is provided in the repository root, following the same pattern used by nutlog, repslog, and bodylog.

Typical workflow:

```bash
makepkg -si
```

The resulting package will:

- Install the release binary to `/usr/bin/krebslog`
- Install documentation (this `docs/` tree + top-level `README.md`, `AGENTS.md`, `CODING_PRACTICES.md`) to `/usr/share/doc/krebslog/`
- Depend on `nutlog` and `repslog` (or recommend them)

See the future `PKGBUILD` for `pkgver()` logic (git rev count + version for development snapshots) and exact installed files.

## Verifying Installation

```bash
krebslog --version
krebslog --help
krebslog data status --probe          # should find your nutlog + repslog
krebslog --json data pull --all --period "last 7 days"
```

On first use of any command that needs the cache, the XDG data directory is created automatically.

## Uninstallation

- Cargo: `cargo uninstall krebslog`
- Package manager: `pacman -R krebslog` (or equivalent)

The cache database (`krebslog.db`) lives in your XDG data directory and is **not** removed automatically:

```
~/.local/share/krebslog/krebslog.db
```

You can safely delete the directory if you want a complete purge.

## Documentation Location (Installed Packages)

When installed via a proper package (PKGBUILD or future distro packages):

- Top-level: `/usr/share/doc/krebslog/README.md`, `AGENTS.md`, `CODING_PRACTICES.md`
- Detailed docs: `/usr/share/doc/krebslog/docs/*.md`

Read them offline with any pager:

```bash
less /usr/share/doc/krebslog/docs/index.md
man -l /usr/share/doc/krebslog/docs/command-reference.md   # some systems
```

## Next Steps

See [Getting Started](getting-started.md) for your first data pull and report.
