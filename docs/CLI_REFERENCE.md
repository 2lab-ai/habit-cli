# Habit CLI — CLI Reference (v0)

This document is the canonical interface contract for the MVP.

> Command name is written as `habit`.

---

## 1) Global behavior

### 1.1 Habit selectors
Where a command accepts `<habit>`, it should accept:
- an exact habit ID (e.g. `h0003`), or
- a unique name prefix (case-insensitive), e.g. `str` → `Stretch`

If ambiguous, error deterministically:
- print candidates sorted by name
- exit code `4`

### 1.2 Global options
These options may appear before or after subcommands.

- `--db <path>`
  - Overrides the DB path for this invocation.
- `--today <YYYY-MM-DD>`
  - Overrides “today” for deterministic output/testing.
  - Takes precedence over `HABITCLI_TODAY`.
- `--format table|json|csv`
  - Default: `table`.
  - Most commands support `table`/`json`.
  - `csv` is primarily for `export` (other commands may ignore or reject it; keep scripts on `json` for stability).
  - `json` must be stable (sorted arrays, fixed keys).
- `--no-color`
  - Disables ANSI color output.
- `--help`

### 1.3 Exit codes
- `0` success
- `2` usage error / validation error
- `3` not found (no habit matches)
- `4` ambiguous selector (name prefix matches multiple habits)
- `5` IO error / DB corruption

---

## 2) Environment variables

- `HABITCLI_DB_PATH`
  - Default DB location override (explained in the MVP spec).
- `HABITCLI_TODAY`
  - Default “today” override (YYYY-MM-DD). Useful for tests.
- `NO_COLOR`
  - If set (to any value), implies `--no-color`.

---

## 3) Commands

## 3.1 `habit add`
Create a new habit.

**Usage**
```bash
habit add <name> [options]
```

**Options**
- `--schedule <pattern>`
  - One of: `everyday`, `weekdays`, `weekends`, `mon,tue,...,sun`
  - Default: `everyday`
- `--period day|week`
  - Default: `day`
- `--target <N>`
  - Integer ≥ 1
  - Default: `1`
- `--notes <text>`
- `--needs-declaration <true|false>`
  - Default: `true`
  - If true, completion is only recognized when a declaration exists for that date.
  - Semantics: check-ins may still be recorded, but are not counted toward completion until a declaration exists.
- `--excuse-quota-per-week <N>`
  - Default: `2`
  - Maximum number of **allowed** excuses per ISO week (Mon..Sun) for this habit.

**Output (table)**
- prints created habit: id, name, schedule, target

---

## 3.2 `habit list`
List habits.

**Usage**
```bash
habit list [--all] [--format table|json]
```

**Options**
- `--all`
  - Include archived habits.

**Notes**
- Sorted by name (then id).

---

## 3.3 `habit show`
Show full details for one habit.

**Usage**
```bash
habit show <habit> [--format table|json]
```

---

## 3.4 `habit archive`
Archive a habit (soft delete).

**Usage**
```bash
habit archive <habit>
```

**Notes**
- Archived habits are hidden from `status` by default.
- Existing check-ins remain.

---

## 3.5 `habit unarchive`
Unarchive a habit.

**Usage**
```bash
habit unarchive <habit>
```

---

## 3.6 `habit checkin`
Add progress for a habit on a date.

**Usage**
```bash
habit checkin <habit> [--date YYYY-MM-DD] [--qty N] [--set N] [--delete]
```

**Options**
- `--date <YYYY-MM-DD>`
  - Default: today
- `--qty <N>`
  - Integer ≥ 1
  - Default: `1`
- `--set <N>`
  - Integer ≥ 0
  - Sets the aggregate quantity for that date (corrections).
- `--delete`
  - Deletes the check-in record for that date (equivalent to quantity = 0).

**Semantics**
- Default behavior: if a record exists for (habit_id, date), `--qty` adds to the existing quantity.
- `--set` overwrites the aggregate quantity for that date.
- If both `--qty` and `--set` are provided: validation error.
- If `--delete` is provided with `--qty` or `--set`: validation error.

---

## 3.7 `habit declare`
Record a declaration for a habit on a date (append-only).

**Usage**
```bash
habit declare <habit> --date YYYY-MM-DD --ts RFC3339 --text <string>
```

**Options**
- `--date <YYYY-MM-DD>` (required)
- `--ts <RFC3339>` (required)
- `--text <string>` (required)

**Semantics**
- Declarations are append-only.
- If a habit has `needs_declaration=true`, completion for that date is only recognized when a declaration exists for that date.

---

## 3.8 `habit excuse`
Record an exception (excuse) for a habit on a date (append-only).

**Usage**
```bash
habit excuse <habit> --date YYYY-MM-DD --ts RFC3339 --reason <string> [--kind allowed|denied]
```

**Options**
- `--date <YYYY-MM-DD>` (required)
- `--ts <RFC3339>` (required)
- `--reason <string>` (required)
- `--kind allowed|denied`
  - Default: `allowed`

**Quota policy (deterministic)**
- Each habit has `excuse_quota_per_week` (default 2).
- If an excuse is requested with `--kind allowed` but the weekly quota is exhausted, the record is stored as `denied`.

---

## 3.9 `habit penalty`
Penalty/trap engine.

### 3.9.1 `habit penalty arm`
Register (or update) a penalty rule for a habit.

**Usage**
```bash
habit penalty arm <habit> --multiplier 2 --cap 8 --deadline-days 1 --date YYYY-MM-DD --ts RFC3339
```

### 3.9.2 `habit penalty tick`
Evaluate missed obligations for a date and create penalty debt for the next day.

**Usage**
```bash
habit penalty tick --date YYYY-MM-DD --ts RFC3339 [--idempotency-key <string>]
```

**Notes**
- Tick is idempotent: running it multiple times for the same date does not create duplicate debt.

### 3.9.3 `habit penalty status` / `habit penalty list`
List outstanding penalty debts as of a date.

**Usage**
```bash
habit penalty status [--date YYYY-MM-DD] [--format table|json]
habit penalty list   [--date YYYY-MM-DD] [--format table|json]
```

### 3.9.4 `habit penalty resolve` / `habit penalty void`
Close a penalty debt.

**Usage**
```bash
habit penalty resolve <debt_id> --date YYYY-MM-DD --ts RFC3339 --reason <string>
habit penalty void    <debt_id> --date YYYY-MM-DD --ts RFC3339 --reason <string>
```

---

## 3.10 `habit status`
Dashboard view for today and the current week.

**Usage**
```bash
habit status [--date YYYY-MM-DD] [--week-of YYYY-MM-DD] [--include-archived] [--format table|json]
```

**Options**
- `--date <YYYY-MM-DD>`
  - The “today” shown in the Today section.
- `--week-of <YYYY-MM-DD>`
  - Choose which week to show (defaults to week containing today).
- `--include-archived`

**Table output requirements**
- Today section:
  - show each scheduled habit for that date
  - show completion like `done/target` and checkmark state
- This week section:
  - daily-target habits: show `X/Y scheduled days done`
  - weekly-target habits: show `sum/target`

---

## 3.11 `habit stats`
Compute streaks and success rates.

**Usage**
```bash
habit stats [<habit>] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--format table|json]
```

**Options**
- `--from <YYYY-MM-DD>`
- `--to <YYYY-MM-DD>`

**Defaults**
- If `<habit>` omitted: stats for all non-archived habits.
- Default window if not provided:
  - day-period habits: last 30 days ending today
  - week-period habits: last 12 ISO weeks ending this week

**Required metrics**
- Current streak
- Longest streak
- Success rate in the requested window

**Notes**
- Dates before a habit’s `created_date` do not count toward streak/success-rate calculations.

---

## 3.12 `habit recap`
HelloHabit-style recap: completion percentages per habit over a time range.

**Usage**
```bash
habit recap [--range ytd|month|week] [--include-archived] [--format table|json]
```

**Options**
- `--range <ytd|month|week>`
  - Default: `month`
  - `ytd`: Year-to-date (Jan 1 through today)
  - `month`: Past 30 days including today
  - `week`: Past 7 days including today
- `--include-archived`
  - Include archived habits in the recap.

**Output (table)**
Displays a HelloHabit-style list with:
- Habit name
- Target label (e.g., "8/day", "3/week")
- Completion percentage
- Visual progress bar
- Success ratio (successes/eligible)

Habits are sorted by completion percentage (descending).

**Output (JSON)**
```json
{
  "recap": [
    {
      "habit_id": "h0001",
      "name": "Water",
      "period": "day",
      "target_label": "8/day",
      "target": 8,
      "successes": 25,
      "eligible": 30,
      "rate": 0.833,
      "percent": 83,
      "range": {
        "kind": "month",
        "from": "2026-01-02",
        "to": "2026-01-31"
      }
    }
  ]
}
```

**Completion calculation**
- **Daily habits**: `successes / eligible_days` where success = counted_quantity >= target on a scheduled day
- **Weekly habits**: `successful_weeks / eligible_weeks` where success = week_sum >= target

Dates/weeks before a habit's `created_date` do not count toward eligible periods.

---

## 3.13 `habit due`
Show habits that are **due** (scheduled and not yet complete) for a given date.

**Usage**
```bash
habit due [--date YYYY-MM-DD] [--include-archived] [--format table|json]
```

**Notes**
- Intended for automation (e.g., OpenClaw nag/dispatch) to list what still needs attention.
- The definition of “complete” must respect:
  - targets (day/week)
  - `needs_declaration=true` (a day is not complete unless a declaration exists for that date)

---

## 3.14 `habit export`
Export habits + check-ins.

**Usage**
```bash
habit export --format json|csv [--out <path>] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--include-archived]
```

**Options**
- `--format json|csv` (required)
- `--out <path>`
  - If omitted: write to stdout.
- `--from`, `--to`
  - Optional filter on check-in date.
- `--include-archived`

**JSON output (minimum)**
```json
{
  "version": 1,
  "habits": [ ... ],
  "checkins": [ ... ]
}
```

**CSV output (minimum)**
- `habits.csv`
  - columns: `id,name,schedule,period,target,notes,archived,created_date,archived_date`
- `checkins.csv`
  - columns: `habit_id,date,quantity`

If exporting to stdout as CSV, emit one combined CSV with a `kind` column, or require `--out` (implementation choice). For MVP simplicity, prefer:
- `--out <dir>` creates `habits.csv` and `checkins.csv`.
