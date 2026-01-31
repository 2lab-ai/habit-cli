# habit-cli

Local-first habit/routine tracking CLI (single JSON file).

- No network / no telemetry
- Deterministic outputs (stable sorting + explicit `--today` / `HABITCLI_TODAY`)
- Fast to operate from a terminal or scripts

> Binary name: `habit`

---

## Install

### From source (recommended for now)

```bash
git clone <this-repo>
cd habit-cli

# install into ~/.cargo/bin
cargo install --path .

habit --version
habit --help
```

### Build only

```bash
cargo build --release
./target/release/habit --help
```

---

## Concepts (what the CEO needs to know)

### Habit model

Each habit has:

- **schedule**: when it is expected (e.g. `everyday`, `weekdays`, `mon,tue,...`)
- **period**: how the goal is measured
  - `day`: you must hit the target on each scheduled day
  - `week`: you must hit the target summed within the ISO week (Mon..Sun)
- **target**: required quantity (`--target N`)

### Habit selectors

Anywhere you see `<habit>`, you can pass:

- the habit id (e.g. `h0003`), or
- a **unique name prefix** (case-insensitive), e.g. `str` → `Stretch`

### Declaration gate (MVP)

By default, new habits are created with `needs_declaration=true`.

If a habit needs declarations:

- you can still record check-ins, but
- they are **not counted as “complete”** for that date unless a declaration exists for that date.

This is intended as a lightweight “commitment” mechanism.

---

## Deterministic runs (important for tests / backfills)

There are two ways to make output reproducible:

1) Pin the app’s logical “today”:

```bash
habit --today 2026-01-31 status

# or
export HABITCLI_TODAY=2026-01-31
habit status
```

2) When creating append-only events, always pass explicit timestamps:

- `declare` requires `--date` and `--ts` (RFC3339)
- `excuse` requires `--date` and `--ts` (RFC3339)
- `penalty tick/arm/resolve/void` require `--date` and `--ts` (RFC3339)

Example timestamp: `2026-01-31T10:00:00Z`

---

## Quickstart (5 minutes)

### 1) Add habits

```bash
habit add "스트레칭" --schedule weekdays --target 1 --period day
habit add "독서"     --schedule everyday  --target 1 --period day --notes "10쪽 이상"
habit add "달리기"   --schedule weekdays --target 3 --period week

habit list
```

### 2) Declare (if declaration gate is on)

```bash
habit declare 스트레칭 \
  --date 2026-01-31 \
  --ts 2026-01-31T09:00:00Z \
  --text "오늘 스트레칭 1회 한다"
```

### 3) Check in

```bash
# defaults to today
habit checkin 스트레칭

# backfill
habit checkin 달리기 --date 2026-01-27
habit checkin 달리기 --date 2026-01-29
```

Correction tools (same command):

```bash
# add qty (default)
habit checkin 독서 --qty 1

# set total qty for the date
habit checkin 독서 --set 1

# delete the record for the date
habit checkin 독서 --delete
```

### 4) Dashboard / status

```bash
habit status

# see a different day
habit status --date 2026-01-30

# choose which week to show
habit status --week-of 2026-01-27
```

---

## Core commands (what you’ll use daily)

### `habit status`

Shows:

- **Today**: scheduled habits for the day, completion (`done/target`)
- **This week**: progress summary

### `habit recap` (HelloHabit-style)

Recap computes completion percentages per habit over a range.

```bash
# past 30 days including today (default)
habit recap

# year-to-date / past week
habit recap --range ytd
habit recap --range week

# scripting
habit recap --format json
```

**Completion logic (important):**

- Daily habits: success on a scheduled day if counted_quantity ≥ target
- Weekly habits: success for a week if week_sum ≥ target
- If `needs_declaration=true`, a day is only eligible for success when a declaration exists for that date

### `habit stats`

Streak + success rate:

```bash
habit stats
habit stats 스트레칭 --from 2026-01-01 --to 2026-01-31
```

### `habit export`

```bash
# JSON to stdout
habit export --format json

# CSV files to a directory
mkdir -p /tmp/habit-export
habit export --format csv --out /tmp/habit-export
```

---

## DB location

DB path resolution (highest priority first):

1. `--db <path>`
2. `HABITCLI_DB_PATH`
3. `${XDG_DATA_HOME}/habit-cli/db.json`
4. `~/.local/share/habit-cli/db.json`

---

## Output + scripting

Most commands support:

- `--format table` (default)
- `--format json` (stable; good for scripts/tests)

Disable ANSI colors:

- `--no-color` or `NO_COLOR=1`

---

## Further docs

- `docs/MVP_SPEC.md`
- `docs/CLI_REFERENCE.md` (canonical contract)
- `docs/EXAMPLES.md`
