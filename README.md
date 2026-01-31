# habit-cli

Local, personal habit/routine tracking CLI.

- Local-first (single JSON file)
- No network / no telemetry
- Deterministic outputs (stable sorting + `--today` / `HABITCLI_TODAY`)

## Requirements

- Rust toolchain (stable)

## Install

### Build from source

```bash
cargo build --release

# binary:
./target/release/habit --help
```

### Install locally (dev)

```bash
cargo install --path .

habit --help
```

## Run (dev)

```bash
# runs the `habit` binary defined in Cargo.toml
cargo run -- --help

# example command
cargo run -- add "Stretch" --schedule weekdays --target 1 --period day
```

## Quickstart

For repeatable outputs (and tests), pin “today”:

```bash
export HABITCLI_TODAY=2026-01-31
```

Add some habits:

```bash
habit add "Stretch" --schedule weekdays --target 1 --period day
habit add "Read" --schedule everyday --target 1 --period day --notes "10 pages counts"
habit add "Run" --schedule weekdays --target 3 --period week
```

Check in (defaults to today):

```bash
habit checkin stretch
habit checkin run --date 2026-01-27
habit checkin run --date 2026-01-29
```

See status:

```bash
habit status
```

Stats:

```bash
habit stats
habit stats stretch --from 2026-01-01 --to 2026-01-31
```

Export:

```bash
# JSON to stdout
habit export --format json

# CSV to a directory
mkdir -p /tmp/habit-export
habit export --format csv --out /tmp/habit-export
```

## DB location

Default path (in priority order):

1. `--db <path>` (per invocation)
2. `HABITCLI_DB_PATH`
3. `${XDG_DATA_HOME}/habit-cli/db.json`
4. `~/.local/share/habit-cli/db.json`

## Output formats

Most commands support:

- `--format table` (default)
- `--format json` (stable; useful for scripting/tests)

`habit export` requires:

- `--format json` (writes JSON)
- `--format csv --out <dir>` (writes `habits.csv` and `checkins.csv`)

Disable ANSI color:

- `--no-color` or `NO_COLOR=1`

## Docs

See:

- `docs/MVP_SPEC.md`
- `docs/CLI_REFERENCE.md`
- `docs/EXAMPLES.md`
