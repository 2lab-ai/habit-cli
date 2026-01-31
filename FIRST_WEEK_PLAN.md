# First Week Plan (incremental)

Goal: ship a usable MVP quickly (target: 1–2 days of build time), then harden.

## Day 1 — Skeleton + storage
1. Pick command name and module layout.
2. Implement DB layer:
   - resolve DB path (env + defaults)
   - initialize empty DB if missing
   - atomic write (temp + rename)
   - schema `version: 1`, `next_habit_number`
3. Implement deterministic date handling:
   - global `--today` + `HABITCLI_TODAY`
4. Implement `habit add`, `habit list`, `habit show`.
5. Add a tiny golden test fixture DB + snapshot tests for `list/show` (stable ordering).

## Day 2 — Tracking + dashboards
1. Implement schedule parsing:
   - `everyday`, `weekdays`, `weekends`, `mon,tue,...`
2. Implement `habit checkin` (aggregate by habit/date).
3. Implement `habit status`:
   - Today section (only scheduled habits)
   - This week section (daily habits: X/Y scheduled done; weekly habits: sum/target)
4. Add deterministic tests for `checkin` and `status` (use `--today`).

## Day 3 — Stats
1. Implement streak calculations:
   - daily-target: consecutive scheduled days
   - weekly-target: consecutive ISO weeks
2. Implement success rate over a fixed window or user-provided `--from/--to`.
3. Implement `habit stats` (table + json).
4. Tests for streak edge cases (unscheduled days, partial weeks).

## Day 4 — Export
1. Implement `habit export --format json` (stdout + file).
2. Implement `habit export --format csv`:
   - prefer `--out <dir>` to write `habits.csv` + `checkins.csv`
3. Ensure stable ordering (habits by name, checkins by date then habit_id).
4. Tests comparing exported output to fixtures.

## Day 5 — Polish + docs
1. Implement `habit archive`.
2. Improve errors:
   - ambiguous habit selector
   - invalid schedule/period/target
   - DB corruption message
3. Add `--no-color`, respect `NO_COLOR`.
4. Review docs in `docs/` and keep examples aligned with actual outputs.

---

## Minimal “ship” checklist
- `add/list/show/checkin/status/stats/export` work end-to-end
- deterministic outputs with `--today`
- safe DB writes
- 10–20 fast CLI tests
