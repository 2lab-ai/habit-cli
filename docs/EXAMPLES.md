# Habit CLI — Examples (v0)

These examples assume the command is `habit` and demonstrate deterministic usage by pinning “today”.

Tip for repeatable outputs:
```bash
export HABITCLI_TODAY=2026-01-31
```

---

## Example 1 — Create a couple of simple daily habits

```bash
habit add "Stretch" --schedule weekdays --target 1 --period day
habit add "Read" --schedule everyday --target 1 --period day --notes "10 pages counts"
habit list
```

Expected characteristics:
- `habit list` shows habits sorted by name.
- IDs are deterministic (`h0001`, `h0002`, …).

---

## Example 2 — Check in for today (default date) and re-check (adds)

```bash
habit checkin stretch
habit checkin stretch --qty 1
habit show stretch
```

Expected characteristics:
- Second check-in increases the same date’s quantity (aggregate), e.g. total becomes 2.

---

## Example 2b — Correct a mistaken check-in (set / delete)

```bash
# Suppose you meant the total to be exactly 1, not 2
habit checkin stretch --set 1

# Or remove the check-in entirely
habit checkin stretch --delete
```

Expected characteristics:
- `--set` overwrites the aggregate quantity for that date.
- `--delete` removes the record (quantity becomes 0).

---

## Example 3 — Weekly-target habit (e.g., run 3× per week)

```bash
habit add "Run" --period week --target 3 --schedule weekdays
habit status
habit checkin run --date 2026-01-27
habit checkin run --date 2026-01-29
habit status
```

Expected characteristics:
- Status shows `Run 2/3 (weekly)` after two check-ins in the week.

---

## Example 4 — Backfill a missed day

```bash
habit checkin read --date 2026-01-30
habit status --date 2026-01-30
habit status --date 2026-01-31
```

Expected characteristics:
- Backfilled check-in shows up when looking at that date.

---

## Example 5 — Stats (streak + success rate)

```bash
habit stats stretch --from 2026-01-01 --to 2026-01-31
habit stats run
```

Expected characteristics:
- `stretch` (daily target) streak counts consecutive scheduled days.
- `run` (weekly target) streak counts consecutive ISO weeks.
- Success rate is computed over the requested window (or defaults if omitted).

---

## Example 6 — Archive something and confirm dashboards hide it

```bash
habit archive read
habit list
habit status
habit status --include-archived

# Recover
habit unarchive read
habit status
```

Expected characteristics:
- `read` no longer appears in default `status` after archiving.
- `--include-archived` shows it.
- `unarchive` restores it to the default dashboard.

---

## Example 7 — Export data

### 7.1 Export JSON to stdout
```bash
habit export --format json
```

### 7.2 Export CSV to a directory
```bash
mkdir -p /tmp/habit-export
habit export --format csv --out /tmp/habit-export
ls -la /tmp/habit-export
# habits.csv, checkins.csv
```

Expected characteristics:
- JSON contains `version`, `habits`, `checkins`.
- CSV files have deterministic column order and stable sorting.
