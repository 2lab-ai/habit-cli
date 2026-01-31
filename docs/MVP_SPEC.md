# Habit CLI — MVP (v0) Product Spec

## 0. Summary
A **local, personal habit/routine tracking CLI**.

- **Not a scheduler**: cron/OS notifications are out of scope.
- **Not a workflow runner**: the CLI never executes scripts or jobs “on schedule”.
- This CLI focuses on: **define habits**, **check in**, **see status**, **see stats**, **export data**.
- Must stay **small** (implementable in ~1–2 days) with **deterministic outputs** for tests.

---

## 1. Goals (MVP scope)

### 1.1 Core user outcomes
1. Define habits with a simple schedule pattern and a target (per day or per week).
2. Mark a habit as done (optionally with a quantity) for a specific date (default: today).
3. View a dashboard for **today** and **this week**.
4. View stats per habit: **current streak**, **longest streak**, **success rate**.
5. Export raw data to JSON/CSV (stdout or file).

### 1.2 Operating constraints
- Local-first, single-user.
- No background processes.
- No notifications.
- Works offline.
- Output is stable (sorted, no random IDs, controllable “today”).

---

## 2. Non-goals (explicit)
- Scheduling, reminders, push notifications, cron replacement.
- Running “workflows” or executing scripts on schedules.
- Multi-device sync, collaboration, cloud accounts.
- Complex recurrence rules (iCal RRULE), time-of-day, time zones beyond **date**.
- Rich journaling, mood tracking, tags, goals, templates.
- Fancy TUI/interactive UI (MVP is plain CLI).
- Encryption/secret management (can be a later add-on).

---

## 3. Key concepts

### Habit
A habit is something you want to do repeatedly, tracked by date.

A habit has:
- **schedule**: which dates are “scheduled/eligible”
- **target**: how much counts as “success” per period (day or week)

### Check-in
A check-in is the amount done for a habit on a given date.

MVP stores **one aggregate quantity per habit per date** (calling check-in twice adds).

### Success
- **Daily-target habits** succeed on a scheduled day if quantity ≥ daily target.
- **Weekly-target habits** succeed for a week if sum(quantity in that week) ≥ weekly target.

### Streak
- Daily-target: consecutive **scheduled days** with success.
- Weekly-target: consecutive **weeks** with success.

---

## 4. Time & calendar conventions (important for correctness)

### 4.1 Dates (no times)
- All user-facing dates are `YYYY-MM-DD`.
- No timestamps in the MVP data model.
- “Today” is **logical today**, overridable for tests.

### 4.2 Weeks
- Weeks are **ISO weeks** (Mon–Sun).
- A “week” in computations means the ISO week containing the relevant date.

### 4.3 Scheduled days
For a habit with a `days_of_week` schedule, a date is **scheduled** iff its weekday is included.

Recommended MVP rule to avoid retroactive weirdness:
- A date **before** `created_date` is treated as **not scheduled** (and should not count toward streak/success-rate). You may still allow check-ins before `created_date`, but they should be ignored by default stats/status.

---

## 5. Data model (MVP)

### 5.1 IDs
- Habits have a stable ID: `h0001`, `h0002`, …
- IDs are assigned incrementally from the DB to keep creation deterministic.

### 5.2 Entities

#### Habit
```json
{
  "id": "h0001",
  "name": "Stretch",
  "schedule": {
    "type": "days_of_week",
    "days": [1,2,3,4,5]
  },
  "target": {
    "period": "day",
    "quantity": 1
  },
  "notes": "2 minutes is fine",
  "archived": false,
  "created_date": "2026-01-31",
  "archived_date": null
}
```

- `days`: integers `1..7` (Mon=1 … Sun=7).
- `created_date` is a date (not timestamp) to keep things simple and deterministic.

#### Check-in (aggregate per day)
```json
{
  "habit_id": "h0001",
  "date": "2026-01-31",
  "quantity": 1
}
```

- If a check-in exists for (habit_id, date), new check-ins add to `quantity`.
- A missing check-in means quantity is `0`.

### 5.3 Derived concepts (not stored)
- Completion status for a date
- Weekly rollups
- Streaks / success rates

---

## 6. Storage format

### 6.1 Location
Single JSON file.

Default path (in priority order):
1. `HABITCLI_DB_PATH` (env)
2. `${XDG_DATA_HOME}/habit-cli/db.json`
3. `~/.local/share/habit-cli/db.json`

Rationale: easy to inspect, backup, and export.

### 6.2 Schema (v1)
```json
{
  "version": 1,
  "meta": {
    "next_habit_number": 1
  },
  "habits": [],
  "checkins": []
}
```

### 6.3 File semantics
- Reads entire file, writes entire file (MVP).
- Writes must be **atomic** (write temp + rename) to reduce corruption risk.
- Recommended: best-effort **advisory lock** during write to avoid concurrent edits by two CLI processes.

---

## 7. Schedule patterns (MVP)
Schedule is a filter that decides whether a date is “scheduled”.

Supported input forms:
- `everyday`
- `weekdays` (Mon–Fri)
- `weekends` (Sat–Sun)
- comma list: `mon,tue,wed` (case-insensitive; accepts `mon`..`sun`)

Internal representation is always `days_of_week`.

---

## 8. CLI surface (MVP)
Command name in docs: `habit` (can be changed).

### 8.1 Global flags / env (testability)
- `--db <path>` (or env `HABITCLI_DB_PATH`) overrides the DB path for this invocation.
  - Precedence: `--db` > `HABITCLI_DB_PATH` > default path.
- `--today YYYY-MM-DD` (or env `HABITCLI_TODAY`) overrides “today” for *all* commands.
  - Precedence: `--today` > `HABITCLI_TODAY` > system date.
- `--format table|json` selects the output format for commands that support it.
- `--no-color` disables ANSI color.

### 8.2 Habit management
- `habit add <name> [--schedule <pattern>] [--target <N>] [--period day|week] [--notes <text>]`
- `habit list [--all] [--format table|json]`
- `habit show <habit>` (habit = id or unique name prefix)
- `habit archive <habit>` (hide from status by default)
- `habit unarchive <habit>` (restore an archived habit)

MVP may omit `edit` to stay small (users can re-add or edit JSON manually). If included:
- `habit edit <habit> [--name ...] [--schedule ...] [--target ...] [--period ...] [--notes ...]`

### 8.3 Tracking
- `habit checkin <habit> [--date YYYY-MM-DD] [--qty N] [--set N] [--delete]`
  - default `--qty 1`
  - default `--date` = today
  - `--set` sets the aggregate quantity for that date (useful for corrections & deterministic tests)
  - `--delete` removes the check-in record for that date (quantity becomes 0)
  - if both `--qty` and `--set` are provided: error

### 8.4 Dashboards
- `habit status [--date YYYY-MM-DD] [--week-of YYYY-MM-DD] [--include-archived] [--format table|json]`
  - shows a “today” section + a “this-week” section

### 8.5 Stats
- `habit stats [<habit>] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--format table|json]`
  - per-habit: current streak, longest streak, success rate
  - default range:
    - daily-target habits: last 30 days ending today
    - weekly-target habits: last 12 weeks ending this week

### 8.6 Export
- `habit export --format json|csv [--out <path>] [--from ...] [--to ...] [--include-archived]`

Export semantics (MVP, keep simple):
- JSON: include schema version + habits + check-ins (filtered by date range if provided).
- CSV: stable headers and stable sort order.

---

## 9. UX and output principles

### 9.1 Determinism for testing
- All table rows are sorted:
  - habits sorted by `name` (then id)
  - dates in ascending order
- Avoid printing timestamps, random IDs, relative time strings.
- Provide `--format json` for machine checks.

### 9.2 Error handling (testability)
Make failures easy to assert in tests.

Recommended (MVP) exit codes:
- `0` success
- `2` usage / validation error (bad flag value, invalid date)
- `3` not found (no habit matches)
- `4` ambiguous selector (name prefix matches multiple habits)
- `5` IO error / DB corruption

Error messages should be single-line and stable.

### 9.3 Human-friendly defaults
- `habit status` should be the “daily driver” command.
- Keep output compact and legible in a terminal.

---

## 10. Example outputs (illustrative)

### 10.1 `habit status`
```
Today (2026-01-31)
- [x] Stretch            1/1
- [ ] Read               0/1
- [ ] Run                0/3 (weekly)

This week (2026-W05)
- Stretch   5/5 scheduled days done
- Read      3/5 scheduled days done
- Run       2/3 (weekly)
```

### 10.2 `habit checkin`
```
Checked in: Stretch (h0001) on 2026-01-31 +1 (total 1)
```

---

## 11. Security & privacy (local-first does not mean “no risk”)

MVP expectations:
- **No network access**: the CLI must not call external services or send telemetry.
- **File permissions**: create the DB file with user-only permissions when possible (e.g. `0600`) and parent dir as user-only (e.g. `0700`).
- **Do not leak contents**: error messages should not print habit notes or full DB contents.
- **Exports are sensitive**: exported JSON/CSV includes personal data (habit names/notes); users should treat exports as private.

---

## 12. Acceptance criteria (MVP)

### 12.1 Habit definition
- Can add a habit with:
  - name
  - schedule pattern (`everyday|weekdays|weekends|mon,tue,...`)
  - target quantity + period (day or week)
  - optional notes
- `habit list` returns deterministic ordering.
- Can archive and restore habits:
  - `habit archive` hides habits from dashboards by default.
  - `habit unarchive` restores them.

### 12.2 Check-ins
- `habit checkin` creates or updates the aggregate quantity for that date.
- `habit checkin --date` supports deterministic backfilling.
- Can correct mistakes without editing JSON:
  - `habit checkin --set N` sets an exact quantity
  - `habit checkin --delete` removes a check-in

### 12.3 Status
- `habit status` shows today and current-week summary.
- Archived habits are excluded by default.

### 12.4 Stats
- `habit stats` computes current streak, longest streak, and success rate.
- Works for both day-target and week-target habits.
- Stats computations exclude dates before `created_date`.

### 12.5 Export
- JSON export includes schema version + habits + check-ins.
- CSV export is stable and includes headers.

### 12.6 Storage
- If DB does not exist, commands that create data initialize it.
- Reads/writes are safe (atomic write).
- DB file is created with user-only permissions when possible.

---

## 13. Decision points (only if needed)
Keep to 3–5.

1. **Command name**: `habit` vs `habits` vs `habit-cli`.
2. **ID scheme**: incremental `h0001` (recommended for determinism) vs UUID.
3. **Week definition**: ISO week (Mon–Sun) (recommended for MVP).
4. **Schedule grammar**: keep MVP to day-of-week patterns only (recommended).
5. **Default stats window**: last 30 days / last 12 weeks (recommended) vs configurable.
