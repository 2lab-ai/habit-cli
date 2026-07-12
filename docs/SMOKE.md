# Smoke Test Checklist

Quick verification that habit-cli is functional. All commands should succeed (exit 0 unless noted).

## Prerequisites

```bash
# Ensure habit is installed
habit --version   # expect: habit 0.1.0

# Set deterministic date for tests
export HABITCLI_TODAY=2026-02-02
```

## Core Commands

### 1. list
```bash
habit list --format json
```
**Expected:** JSON with `habits` array (may be empty on fresh install)

### 2. status
```bash
habit status --format json
```
**Expected:** JSON with `today.date` and `today.habits` array

### 3. stats
```bash
habit stats --format json
```
**Expected:** JSON with `stats` array containing per-habit metrics

### 4. recap
```bash
habit recap --format json
```
**Expected:** JSON with `recap` array containing completion percentages

### 5. add (requires write)
```bash
habit add "smoke-test-habit" --schedule everyday --target 1 --period day --format json
```
**Expected:** JSON with `habit.id` (e.g., `h0001`)

### 6. checkin (requires write)
```bash
habit checkin smoke-test-habit --date 2026-02-02 --qty 1 --format json
```
**Expected:** JSON with `action: "add"`, `quantity` incremented

### 7. declare (requires write)
```bash
habit declare smoke-test-habit --date 2026-02-02 --ts 2026-02-02T10:00:00Z --text "test" --format json
```
**Expected:** JSON with `declaration.id` (e.g., `d000001`)

### 8. penalty tick (requires write)
```bash
habit penalty tick --date 2026-02-01 --ts 2026-02-02T00:00:00Z --format json
```
**Expected:** JSON with `date` and `created` array (may be empty if no penalties due)

### 9. penalty status
```bash
habit penalty status --format json
```
**Expected:** JSON with `date` and `debts` array

### 10. export
```bash
habit export --format json
```
**Expected:** JSON with `version`, `habits`, `checkins` arrays

## Optional (v0.2+)

### 11. routine session flow (requires write)
```bash
habit routine add "smoke-routine" --at 09:00 --format json
habit routine step-add smoke-routine --name "water" --minutes 5 --format json
habit routine step-add smoke-routine --name "stretch" --minutes 5 --format json
habit routine start smoke-routine --date 2026-02-02 --ts 2026-02-02T09:00:00Z --format json
# use returned session id:
habit routine next <session_id> --ts 2026-02-02T09:05:00Z --format json
habit routine skip <session_id> --ts 2026-02-02T09:06:00Z --reason "test" --format json
habit routine done <session_id> --ts 2026-02-02T09:07:00Z --format json
habit routine status <session_id> --format json
```
**Expected:** `routine status` JSON에 `counts.pending=0`

### 12. nag plan (requires write for `sent`/`snooze`)
```bash
habit nag show --format json
habit nag plan --date 2026-02-02 --now-ts 2026-02-02T09:00:00Z --format json
habit nag sent --ts 2026-02-02T09:00:00Z --format json
habit nag snooze --until 2026-02-02T13:00:00Z --reason "test" --format json
habit nag plan --date 2026-02-02 --now-ts 2026-02-02T12:30:00Z --format json
```
**Expected:** snooze 동안 `should_send=false`, `next_check_at`가 `until`로 나옴

## Quick One-Liner (read-only)

```bash
export HABITCLI_TODAY=2026-02-02 && \
  habit list --format json >/dev/null && \
  habit status --format json >/dev/null && \
  habit stats --format json >/dev/null && \
  habit recap --format json >/dev/null && \
  habit penalty status --format json >/dev/null && \
  echo "✓ All read-only smoke tests pass"
```

## Notes

- Commands `edit` and `due` are in source but may not be in released binary (check `habit --help`)
- Use `--format json` for scripting; output is stable/sorted
- Use `HABITCLI_TODAY` or `--today` for deterministic tests
