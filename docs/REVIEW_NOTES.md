# Review Notes — Habit CLI MVP Spec

Scope of this review (as requested):
- Ensure alignment with **habit/routine management** (not cron/workflow runner)
- Ensure **MVP command completeness**
- Reduce/avoid **scope creep**
- Improve **testability / acceptance criteria**
- Add **security/privacy** considerations for local data

## Summary of edits made
Edits were applied directly to:
- `docs/MVP_SPEC.md`
- `docs/CLI_REFERENCE.md` (kept interface contract consistent with the spec)
- `docs/EXAMPLES.md` (examples updated to reflect correction/unarchive flows)

### 1) Tightened alignment: habit tracker, not scheduler/workflow runner
**Change:** Expanded the Summary + Non-goals to explicitly state:
- Not a scheduler
- Not a workflow runner (never executes scripts “on schedule”)

**Why:** Specs for “routine” tools often drift into cron-like behavior. This makes the product boundary harder to enforce and test.

### 2) Clarified calendar semantics (date/week/scheduled-day rules)
**Change:** Added a new section **“Time & calendar conventions”**:
- Date-only model (`YYYY-MM-DD`), no timestamps
- ISO week definition (Mon–Sun)
- Recommended rule: dates before `created_date` are not considered scheduled for streak/success calculations

**Why:** Streak/success-rate bugs usually come from ambiguous calendar conventions. Making these explicit improves correctness and prevents hidden scope creep into time zones / RRULE complexity.

### 3) Improved testability: deterministic “today”, stable errors, and exit codes
**Change:**
- Documented precedence for `--db` vs `HABITCLI_DB_PATH`, and `--today` vs `HABITCLI_TODAY`.
- Added an **Error handling** section with recommended stable exit codes:
  - 0 success, 2 validation, 3 not found, 4 ambiguous selector, 5 IO/DB error

**Why:** Stable output is necessary but not sufficient; stable failure modes make integration tests reliable and reduce flakiness.

### 4) Added a minimal “correction path” for check-ins
**Change:** Extended `habit checkin` to include:
- `--set N` (set exact aggregate quantity)
- `--delete` (remove the check-in record)
- and specified conflict behavior when `--qty` and `--set` are both provided

**Why:** Without a correction path, users (and tests) are forced to edit JSON manually to fix mistakes, which undermines both usability and acceptance tests.

### 5) Security & privacy expectations for local-first storage
**Change:** Added **Security & privacy** section:
- No network / no telemetry
- File permission expectations (DB file `0600`, directory `0700` when possible)
- Avoid leaking notes/DB contents in errors
- Reminded that exports are sensitive

**Why:** Local-first can still leak data via loose file permissions, logs, or accidental sharing of exports.

## Notes on scope creep (kept constrained)
- Kept schedule grammar limited to day-of-week patterns.
- Added `unarchive` as the minimal recovery path (prevents “edit JSON” recovery).
- Did *not* add reminders, timers, time-of-day rules, tags, journaling, templates, sync, encryption, etc.

## Potential follow-ups (not required for MVP)
If there’s time after MVP, the next most valuable additions would be:
- Advisory file locking implementation details (platform specifics)
- `habit validate` / `habit doctor` for corrupted DB handling (currently only noted as best practice)
- Optional export redaction (e.g. omit notes) if users commonly share exports
