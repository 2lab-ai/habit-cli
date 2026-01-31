# IDEAS REVIEW (oracle) — for `docs/SPEC_IDEAS_DRAFT_1.md`

Scope of this review:
- Keep the product centered on a **deterministic CLI core** (`habit`)
- Assume **OpenClaw** handles NLP, conversation, scheduling/cron, timers/UX, and any network actions
- Decide: (a) keep for v1, (b) defer, (c) risks/guardrails, (d) missing pieces

Related canonical direction: `docs/SPEC_V1_DETERMINISTIC.md` (role split + deterministic contract).

---

## (a) What to keep for v1

### V1 principle: “engine, not agent”
From the ideas doc, the highest-leverage v1 features are those that:
- are **pure state transitions + deterministic calculations**
- provide **machine-readable outputs** that OpenClaw can orchestrate
- avoid any requirement for background processes or network calls

### Quick mapping: ideas (Section 1) → v1 decision
- **1.1 자동 등록(자연어 → CLI)**: *Yes for the product*, but **OpenClaw-only**. CLI just exposes deterministic `add/edit` primitives.
- **1.2 매일 질문/체크**: *Yes*, via OpenClaw cron + `habit due/pending/status`.
- **1.3 NEW 습관 선언문 강제**: *Yes (v1)*, implement as CLI policy + state (`needs_declaration`).
- **1.4 트랩/패널티**: *Yes, but minimal (v1)*: bounded debt generation + idempotent tick.
- **1.5 루틴(타이머)**: *Yes, but CLI is state machine only (v1)*; timers/UX are OpenClaw.
- **1.6 외부 공개 트랩(해시/해시체인/조건부 공개)**: *Partial*: **offline commitment/hash text generation** maybe v1/v1.1; anything beyond that defer.
- **1.7 제3자+결제+키 조각 공개**: *Defer* (separate service/product).

### Keep 1) M0 Core Tracking (must-have)
Keep the entire M0 framing:
- Habit definitions (schedule/target/onboarding flags)
- Checkins (per-date quantities)
- Derived status: `due/pending/status/stats`
- Archive

Why it’s v1:
- Everything else (penalties, declarations, routines) depends on “what is due and what is done.”
- This is the piece that must be rock-solid and testable.

OpenClaw orchestration hook:
- daily prompt flow: OpenClaw calls `habit due --date … --format json` and renders questions.


### Keep 2) M1 Declaration (minimal, but real)
The “NEW habit requires declaration” idea is **core product differentiation** and fits the role split well.

Recommended v1 slice:
- Store declarations as **append-only** records (`declare`, optional `amend`).
- Add a per-habit onboarding policy flag (example):
  - `declaration_policy = none | first_checkin | always` (v1 can ship with only `first_checkin` + `none`)
- Make declaration requirement **computable** (e.g., `pending.needs_declaration=true`).

What the CLI must do (deterministic):
- Given DB state + `--date/--ts`, return whether a habit can be counted as complete.

What OpenClaw does:
- “응답할 때까지 계속 유도” (nagging / re-asking / persuasion) and any natural-language framing.


### Keep 3) M2 Penalty/Trap Engine (small + bounded)
The penalty concept can still be deterministic if implemented as a pure “debt generator.”

Recommended v1 slice (avoid complexity):
- One penalty rule kind first: **multiplier debt with cap** (as in ideas: 2× with cap).
- A single deterministic daily transition:
  - `penalty tick --date YYYY-MM-DD` produces (idempotently) “debts” based on yesterday’s unmet requirements.

Must-have constraints:
- **Idempotent tick** (running twice never doubles the debt).
- **Hard caps** and explicit “pause/disable penalties” switches.

OpenClaw does:
- When to run `tick` (cron).
- How to message the user (tone, escalation).


### Keep 4) M3 Routine Sessions (state machine only; no real timers)
The routine idea is compatible with the deterministic split if the CLI is *not* a real-time runner.

Recommended v1 slice:
- Define routines (steps with durations, optional text like quotes).
- Sessions are event-sourced or stateful but deterministic:
  - `routine start --date … --ts …`
  - `routine next|skip|done --ts …`
  - `routine status --format json`

Explicit v1 boundary:
- CLI never “counts down” or sleeps.
- OpenClaw owns timer UX (notifications, live prompts, etc.) and passes timestamps back via `--ts`.


### Keep 5) “External trap” ONLY as offline text/commitment generation (optional v1, safer as v1.1)
The ideas include “X anchor / hash-chain” and conditional reveal. The safest v1 interpretation:
- CLI may generate **commitment/hash-chain payloads** as strings/JSON
- CLI must **not** post to X or store OAuth tokens

Whether this is v1 vs v1.1 depends on implementation cost and crypto/key-management readiness.

Minimum viable “anchor” feature (if included):
- Deterministically compute a commitment hash for a declaration (see `SPEC_V1_DETERMINISTIC.md` direction).
- Provide a command that outputs the text OpenClaw should post (but OpenClaw decides to post).

---

## (b) What to defer (not v1)

### Defer 1) M6 Payment / Marketplace / third-party escrow (hard no for v1)
Reasons:
- Legal/regulatory complexity (payments, escrow, gambling-like mechanics).
- Security/compliance burden.
- Requires network services + accounts, violating “local deterministic core.”

Treat as a separate product/service exploration later.


### Defer 2) Deadman switch + gradual key reveal (M5 advanced)
Reasons:
- Complex cryptography + irreversible failure modes.
- High “user harm” potential (accidental disclosure).
- Requires reliable scheduling and possibly online coordination.

If revisited later: design as a separate module with explicit opt-in and extensive safety rails.


### Defer 3) “External public trap” beyond hash text
Defer anything that:
- automatically posts
- manages social tokens/credentials
- tries to prove things to third parties beyond a simple anchor


### Defer 4) iPhone sync/app
Explicitly already out-of-scope for v1 per the ideas doc.


### Defer 5) Any non-deterministic / heavy features
- NLP inside the CLI (must remain OpenClaw)
- background daemons
- network calls
- complex recurrence/time-of-day scheduling inside the CLI

---

## (c) Risks & guardrails

### 1) Determinism erosion (highest engineering risk)
Common ways determinism breaks:
- reading system clock implicitly
- locale/timezone differences
- randomness in ID/crypto nonces
- unstable JSON ordering / non-stable sort

Guardrails:
- For **all write commands**, require `--date` and/or `--ts` (caller-injected).
- Stable sorting in all outputs.
- Canonicalization rules for any hash/commitment payloads.
- Document and test exit codes and error shapes.


### 2) Security & privacy risk (especially declarations)
Declarations are inherently sensitive.

Guardrails:
- Default file permissions (best-effort 0600) and avoid leaking content in errors.
- If encryption is in v1:
  - clear key-management story (passphrase input, env var option for non-interactive)
  - atomic writes + corruption handling
  - zeroize secrets in memory where possible
- If encryption is NOT in v1:
  - be explicit in docs that declaration text is stored in plaintext (do not imply safety).


### 3) User harm / mental health (product risk)
“Trap/penalty” mechanics can escalate shame, anxiety, or self-punishment.

Guardrails:
- Hard caps on penalty growth.
- A first-class **pause** / **disable** path (global and per-habit).
- Explicit “grace / exception” mechanism with quotas.
- Avoid language that encourages self-harm; OpenClaw prompt templates should be conservative.


### 4) Social platform & reputational risk (X anchor)
If the system posts frequently, it can become spammy, get rate-limited, or banned.

Guardrails:
- CLI only generates payload; OpenClaw requires explicit confirmation before posting.
- Rate-limit or batch anchors (policy in OpenClaw).
- Never auto-post on install; opt-in.


### 5) Legal/financial risk (payments / third-party claims)
Reasons to keep out of v1:
- escrow-like behavior, fraud vectors, chargebacks
- minors/consumer-protection issues

Guardrail:
- Keep purely local + no-money in v1.


### 6) Data integrity risk (local DB corruption)
Guardrails:
- atomic write (temp + rename)
- advisory lock
- `validate/doctor` command later (or at least clear error codes + recovery steps)

---

## (d) Missing pieces / decisions needed

These are the main gaps in `SPEC_IDEAS_DRAFT_1.md` that must be nailed down to implement safely and deterministically.

### 1) Declaration enforcement semantics
Decide and document:
- Does missing declaration **block checkin** or just block “counting as complete”?
  - The deterministic spec leans toward blocking checkin for simplicity; confirm.
- Can a declaration be added after-the-fact to “unlock” a past date?
- Amendment rules (what changes, how to compute canonical commitment across amendments).


### 2) Penalty semantics (the debt model)
The ideas say “2배, cap” but missing:
- What exactly is doubled? (required qty? outstanding debt? both?)
- Does debt carry forward indefinitely? Can it be partially repaid?
- Interaction with weekly-target habits.
- Idempotency keys: how do we guarantee tick generates the same debt once?

Minimum v1 recommendation:
- Define `PenaltyDebt` as explicit records keyed by (habit_id, date, kind), and tick is idempotent on that key.


### 3) Exceptions / quota policy
The ideas mention “인정되는 예외/불인정 예외” and quota, but missing:
- exception categories (illness, travel, etc.) and whether they are free-form or enumerated
- quota window (per week? per month?)
- whether exceptions suppress penalties or just mark a day as “excused”

Recommendation:
- v1 keep exceptions simple: `exception grant <habit> --date … --kind excused` with an optional quota enforced per ISO week.


### 4) Routine session model details
Missing:
- how to represent step completion timestamps deterministically
- how to compute “current step” when events are out-of-order or repeated
- whether routine steps map to habits/checkins (optional integration)

Recommendation:
- Treat routine sessions as their own deterministic state machine first; integrate with habits later.


### 5) Crypto/key management and DB format/migrations
The ideas doc has a core principle: **“로컬 저장 + 암호화”**. That implies a real v1 decision:
- either **ship encryption in v1** (at least for `Declaration.text`),
- or explicitly downgrade the principle and document plaintext storage risks.

If encryption/commitments are included:
- how does the user provide the passphrase? (interactive prompt vs env/flag)
- what happens if passphrase is wrong? (exit code)
- schema versioning + migration plan (plaintext MVP DB → encrypted container)
- deterministic encryption nonce/version-counter strategy (must never reuse nonces)


### 6) CLI contracts: stable JSON schemas
To support OpenClaw orchestration reliably, the following must be fully specified:
- `due` output schema
- `pending` output schema (including declaration + exception + debt fields)
- sorting rules for arrays
- consistent exit codes and error JSON (if any)


### 7) “Fast response” constraints
Ideas mention p95 100ms.
Missing:
- explicit budget boundaries (DB size assumptions)
- any indexing strategy (if JSON grows large)

Recommendation:
- v1 keep DB small and optimize for simplicity; add performance tests once data grows.

---

## Suggested v1 cut (one-line)
**Ship v1 as:** M0 (tracking) + M1 (declaration onboarding) + minimal M2 (idempotent debt) + M3 (routine sessions without timers), with crypto/anchor as optional v1.1 depending on readiness.
