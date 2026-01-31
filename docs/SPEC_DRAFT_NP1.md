# Habit CLI — NP1 Workflow Spec (Draft)

> **Purpose:** Specify a *day-to-day user experience* built on top of the existing `habit` CLI using **cron + Telegram automation**, including:
> - habit creation from natural language
> - daily check-in loop (prompt → reply → update → recap)
> - declaration sentence enforcement for new habits
> - penalty/trap rule execution + exception policy
> - quiet hours + escalation
>
> **Philosophy:** `habit` remains a **local habit tracker** (no background scheduling). Scheduling and messaging are handled by a thin “automation layer” (cron + Telegram bot) that **calls the CLI**.

---

## 0) Glossary

- **Habit CLI**: the `habit` binary in this repo.
- **Automation layer / habit-bot**: a small wrapper (shell/Python/Rust) that:
  - reads state via `habit status --format json`
  - writes state via `habit checkin ...`
  - sends/receives Telegram messages
  - stores automation-only config/ledger (quiet hours, penalties, excuses)
- **Scheduled**: a habit is “scheduled” on date *D* if its schedule includes that weekday and `D >= created_date`.
- **Done**: for a scheduled day, `quantity >= target` (or for weekly habits, week-sum `>= target`).
- **Excused**: user requested an exception; treated as “not a failure” per the exception policy.
- **Penalty**: a cost incurred when a scheduled habit is not done by the day deadline.
- **Trap**: an additional rule that makes avoidance costly (e.g., escalating penalty if left unresolved by a later time).

---

## 1) System shape (what runs where)

### 1.1 Components

1. **`habit` CLI (this repo)**
   - Source of truth for habits + check-ins.
   - Deterministic JSON outputs.

2. **Telegram bot integration**
   - Sends prompts (daily plan, reminders, penalty notices).
   - Receives user replies (check-ins, exception requests).

3. **Cron (or systemd timers)**
   - Schedules the automation layer at fixed times.

### 1.2 Files / state ownership

- Habit data: `habit` DB JSON (existing; local).
- Automation config (new, owned by habit-bot):
  - Recommended path: `${XDG_CONFIG_HOME}/habit-cli/automation.json` or `~/.config/habit-cli/automation.json`
- Automation ledger (new; for auditability):
  - `${XDG_DATA_HOME}/habit-cli/ledger.jsonl` (append-only)

**Rationale:** penalties/excuses are workflow policy, not core habit tracking. Keeping them in a sidecar allows NP1 iteration without destabilizing the DB schema.

---

## 2) Habit creation from natural language (NL → structured habit)

### 2.1 Entry points (Telegram-first)

User sends one of:

- **Free-form:**
  - `new habit: read 20 pages daily`
  - `add habit meditate 10m weekdays`
  - `habit: run 3x/week on weekdays`

- **Semi-structured shortcut (recommended):**
  - `+ read 20 pages everyday`
  - `+ run 3/week weekdays`
  - `+ stretch weekdays` (defaults apply)

The automation layer parses the message into:

- `name`
- `schedule` (everyday/weekdays/weekends/mon,tue,...)
- `period` (day/week)
- `target` (integer)
- optional `unit` (pages/min/km) for UX only
- **declaration** (required; see §3)
- optional `deadline` (per habit override; otherwise global)
- optional `penalty` / `trap` policy overrides

### 2.2 Deterministic NL parsing (NP1 rules)

NP1 parsing should be **deterministic** and designed for “80% cases”:

1. **Period + target**
   - If input contains `N/week`, `N per week`, `N×/week`, `N x/week` → `period=week`, `target=N`.
   - Else if it contains an integer `N` + a unit word (pages/min/km/sets/etc) → `period=day`, `target=N`.
   - Else → `period=day`, `target=1`.

2. **Schedule**
   - Keywords:
     - `daily`, `everyday` → `everyday`
     - `weekdays` → `weekdays`
     - `weekends` → `weekends`
   - Day lists:
     - `mon wed fri`, `mon,tue` etc → `mon,wed,fri`
   - Else → default `everyday`.

3. **Name**
   - Strip the schedule/target tokens; remaining text becomes the habit `name`.
   - Normalize whitespace; keep original capitalization.

4. **Unit (optional)**
   - If `target>1` came from `N <unit>`, store `<unit>` as `unit`.
   - Units are for display; core check-in remains integer quantities.

### 2.3 Clarification loop (when parsing is ambiguous)

If parsing yields uncertainty, the bot asks *one question at a time*:

- “Is this a daily target or weekly target?”
- “Which days: everyday / weekdays / weekends / mon,tue,... ?”
- “What counts as ‘done’? (a number)”

Only after confirmation does the bot run:

```bash
habit add "<name>" --schedule <pattern> --period <day|week> --target <N> [--notes "unit: pages"]
```

---

## 3) Declaration sentence enforcement (new habits)

### 3.1 What a declaration is

A **declaration** is a short, first-person sentence that makes the habit concrete and binary.

Examples:
- “I will read 20 pages.”
- “I will stretch for 2 minutes.”
- “I will run 3 times this week.”

### 3.2 Enforcement rule

**A habit is not considered ‘active’ in Telegram prompts until it has a declaration.**

When creating a habit, the bot must obtain a declaration via:

1. User-provided declaration, or
2. Bot-suggested declaration + explicit “confirm”.

### 3.3 Validation (NP1)

Declaration must:
- be **one sentence**
- start with `I` (case-insensitive) and contain a verb
- avoid negation forms like “I will not …” (soft rule; warn but allow)

If invalid, bot replies with a correction prompt:
- “Rewrite as one sentence starting with ‘I will …’.”

### 3.4 Storage

**NP1 recommendation:** stored in automation sidecar config keyed by habit id.

Later (optional): store as a new optional field on `Habit` (`declaration: Option<String>`), see §8.

---

## 4) Daily check-in loop (cron-driven) — concrete timeline

> Times below are examples; they should be configurable.

### 4.1 Global timeline (local time)

- **07:30** — Morning plan message (always).
- **12:30** — Midday nudge (only if something is still incomplete).
- **18:30** — Evening nudge (only if still incomplete).
- **21:30** — Final warning (only if still incomplete; includes penalty preview).
- **22:30** — Day deadline.
- **22:31** — Evaluation + penalty trigger messages.
- **23:00–08:00** — Quiet hours (see §7).
- **08:05 (next day)** — If penalties unresolved, send “carryover” summary.

### 4.2 How the bot computes “what’s left”

At any time *T* on date *D*, the bot runs:

```bash
habit status --date <D> --format json
```

It reads `today.habits[]`:
- show only scheduled habits
- consider `done=false` as “left”

For weekly habits, `quantity` is week-sum and `target` is weekly target (already in status JSON).

### 4.3 Morning plan message template

**Message (07:30):**

```
Good morning. Today is {DATE}.

Today’s commitments:
{BULLETS_TODAY}

Reply:
- "done <habit>" (adds +1)
- "set <habit> <N>" (sets quantity)
- "status" (shows the list again)
```

**Bullet formatting rules:**
- Daily habit: `- [ ] Read — 0/20 pages  ("I will read 20 pages.")`
- Weekly habit: `- [ ] Run — 1/3 this week ("I will run 3 times this week.")`

*(If unit is missing: omit it.)*

### 4.4 Midday / evening nudge templates

**Nudge (only if incomplete):**

```
Checkpoint ({TIME}). Still open today:
{BULLETS_LEFT}

Quick replies:
- "done <habit>"
- "skip <habit> <reason>" (request exception)
```

### 4.5 Final warning template (penalty preview)

**Final warning (21:30, only if incomplete):**

```
Final check-in before {DEADLINE}.

Remaining:
{BULLETS_LEFT_WITH_PENALTIES}

If these are not done by {DEADLINE}, penalties will trigger automatically.
```

### 4.6 End-of-day evaluation + penalty trigger

At **22:31**, bot runs status again and computes `missed = {today.habits where done=false}`.

For each missed habit, it emits a penalty notice (see §5), unless an approved exception exists.

---

## 5) Penalty + trap rules

### 5.1 Policy goals

- Make “missing” *costly enough* to matter.
- Keep penalties *doable* (no self-sabotage).
- Keep it auditable (ledger).

### 5.2 Default penalty rule (NP1)

For each missed habit on date *D*:

- Create a **penalty item** with:
  - `habit_id`
  - `date=D`
  - `penalty_text`
  - `status=pending`

Default `penalty_text` examples:
- “20 burpees”
- “Donate ₩5,000”
- “30-minute cleanup sprint”

Penalties should be configured per habit (or global default).

### 5.3 Trap rule (NP1)

If a penalty is still **pending** by **10:00 next day**:

- escalate to **trap** (one of):
  - increase penalty magnitude (e.g., +10 burpees)
  - add an additional penalty (“+ cold shower”) 
  - lock “fun check-ins” (bot refuses to accept non-penalty actions until resolved)

The trap rule must be deterministic and stated explicitly in messages.

### 5.4 Penalty execution message template

**At 22:31:**

```
{DATE} result: Missed {HABIT_NAME}.

Penalty now due:
- {PENALTY_TEXT}

Reply:
- "penalty done {habit}" (marks penalty completed)
- "skip {habit} <reason>" (request exception, if eligible)
```

### 5.5 Logging

Append JSONL entries for every penalty lifecycle event:

- `penalty_created`
- `penalty_completed`
- `trap_escalated`

This ledger is separate from habit DB.

---

## 6) Exception policy (skip/illness/travel)

### 6.1 Why exceptions exist

Exceptions prevent the system from incentivizing lying (“just mark it done”).

### 6.2 Default exception rules (NP1)

- Must include a **reason**.
- Must be requested **before** the day deadline, except emergencies.
- Monthly quota:
  - **2 exceptions per habit per month** (configurable)
  - **6 total exceptions per month** across all habits (configurable)
- No “blanket exceptions” longer than 3 consecutive scheduled occurrences without manual review.

### 6.3 What an exception does

NP1 behavior options (choose one; both are compatible with sidecar ledger):

- **Option A (simplest, stats-friendly for now):** mark the habit as *done* by setting quantity to target.
  - Pros: no CLI schema change; streaks stay intact.
  - Cons: cannot distinguish “done” vs “excused” in CLI-only stats.

- **Option B (more correct, requires CLI support):** store an explicit “excused” outcome so it does not count as a failure but is distinguishable.
  - See §8 (CLI additions).

### 6.4 Exception request message template

User:
- `skip read headache`

Bot (if eligible):

```
Exception request for {HABIT_NAME} ({DATE}).
Reason: {REASON}

Approved. Quota remaining this month:
- {HABIT_QUOTA_LEFT} for this habit
- {TOTAL_QUOTA_LEFT} total
```

If not eligible:

```
Exception denied for {HABIT_NAME} ({DATE}).
Reason: {WHY}

Options:
- "done {habit}" before {DEADLINE}
- or accept penalty after {DEADLINE}
```

---

## 7) Quiet hours + escalation ladder

### 7.1 Quiet hours definition

Default: **23:00–08:00** local time.

During quiet hours:
- Do not send routine nudges.
- Allow only:
  - penalty triggers (if configured as “critical”)
  - trap escalations (if user opted in)
  - a single morning catch-up message at quiet-hours end

### 7.2 Escalation ladder (NP1)

Escalation is based on *miss frequency*.

- Level 0 (normal): morning plan + conditional nudges.
- Level 1 (1 missed in last 7 days): add a 21:30 final warning if anything open.
- Level 2 (2+ missed in last 7 days):
  - final warning always (even if only one habit left)
  - penalty message is **not silent**
- Level 3 (3+ missed in last 14 days OR trap escalated twice):
  - morning message includes “yesterday recap”
  - optionally add a “commitment check” prompt:
    - “Reply with: ‘I commit to completing {TOP_1_HABIT} today.’”

Escalation level should decay automatically after a clean streak (configurable).

---

## 8) Minimal CLI additions needed (to support NP1 cleanly)

The workflow above can be prototyped with **zero** changes to `habit` by keeping declarations/penalties/excuses in sidecar files.

However, to make NP1 robust and reduce glue code, the following **minimal additions** are recommended (small surface area, still local-only):

### 8.1 `habit parse` (deterministic NL → add args)

**Command (new):**
```bash
habit parse "read 20 pages daily"
```

**Output (`--format json` only):**
```json
{
  "name": "Read",
  "schedule": "everyday",
  "period": "day",
  "target": 20,
  "unit": "pages"
}
```

Rationale: allows Telegram bot / scripts to reuse the same parsing logic.

### 8.2 Store declaration on the habit (optional field)

Add to `model::Habit`:
- `declaration: Option<String>`

Add to CLI:
- `habit add ... --declaration "I will ..."`
- `habit show` prints it

Rationale: declaration is first-class UX; storing it in `notes` is brittle.

### 8.3 First-class “excused” outcomes (optional, but unlocks correct stats)

Add one of:

**Option 1:** extend check-ins with a kind:
- `Checkin { habit_id, date, quantity, kind: "done"|"excused" }`

**Option 2:** add a separate list:
- `excuses: [{habit_id,date,reason}]`

Add CLI command:
```bash
habit excuse <habit> [--date YYYY-MM-DD] --reason "..."
```

Rationale: enables exception policy without lying to the data model.

---

## 9) Cron schedule examples

### 9.1 Crontab (example)

```cron
# Morning plan
30 7 * * *  habit-bot send plan

# Nudges
30 12 * * * habit-bot send nudge
30 18 * * * habit-bot send nudge
30 21 * * * habit-bot send final

# Deadline evaluation + penalty
31 22 * * * habit-bot eval day

# Trap escalation next morning
0 10 * * * habit-bot eval traps
```

### 9.2 Bot/CLI contract (minimum)

- Bot reads: `habit status --format json`
- Bot writes:
  - `habit checkin <habit> --qty 1`
  - `habit checkin <habit> --set N`
  - (optional) `habit excuse ...`

---

## Appendix A) Telegram command grammar (NP1)

All commands should be accepted case-insensitively.

### A.1 Check-in
- `done <habit>` → `habit checkin <habit> --qty 1`
- `+<habit>` (shortcut) → same as above
- `set <habit> <N>` → `habit checkin <habit> --set <N>`

### A.2 Status
- `status` → bot re-sends today list

### A.3 Exceptions
- `skip <habit> <reason...>` → request exception

### A.4 Penalties
- `penalty done <habit>` → mark penalty completed in ledger

---

## Appendix B) Message formatting guidelines

- Keep messages scannable (bullets, short lines).
- Always include:
  - date
  - remaining items
  - exact reply syntax
- Prefer stable habit identifiers (habit name + id in parentheses) in penalty contexts.
