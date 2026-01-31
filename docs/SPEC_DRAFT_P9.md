# habit-cli v1 — Critical Spec Proposal (P9 Proxy)

**Author:** p9-proxy (AI agent)  
**Date:** 2026-01-31  
**Status:** DRAFT — Requires CEO review  
**Builds on:** MVP_SPEC.md (v0)

---

## 0. Executive Summary

The MVP (v0) delivers basic habit tracking: add → checkin → status → stats. It's a passive observer.

v1 transforms habit-cli into an **accountability system** with:
- **Commitments** (promises with deadlines)
- **Penalties** (stakes that hurt when you fail)
- **Declarations** (public/logged statements of intent)
- **Encryption** (local secrets stay secret)
- **Private sync** (GitHub as encrypted backup)

The core thesis: **Habits without teeth are wishes.** Penalties must be enforceable and painful enough to change behavior.

---

## 1. Scope Control — The Anti-Creep Manifesto

### 1.1 What v1 IS

- A **commitment tracker** with enforceable penalties
- An **encrypted local-first** data store
- A **private GitHub sync** mechanism (disaster recovery + multi-device)
- A **declaration log** (immutable record of stated intentions)

### 1.2 What v1 IS NOT (hard boundaries)

| Feature | Status | Rationale |
|---------|--------|-----------|
| Push notifications | ❌ OUT | OS-specific hell; use external tools (cron, shortcuts) |
| Mobile app | ❌ OUT | Web/native is a different product |
| Multi-user / sharing | ❌ OUT | Single-user system; collaboration is scope explosion |
| Financial integrations | ❌ OUT | Too much liability; penalties are self-enforced |
| AI coaching / suggestions | ❌ OUT | Distraction from core: tracking + accountability |
| Social features | ❌ OUT | No leaderboards, no "friends," no gamification |
| Complex recurrence (iCal) | ❌ OUT | Keep schedule patterns simple (day-of-week only) |
| Time-of-day tracking | ❌ OUT | Date granularity only; times add complexity |
| Habit "categories" / tags | ❌ OUT | Flat list works; hierarchy is premature |
| Goal-based tracking | ❌ OUT | Habits ≠ goals; don't conflate them |

### 1.3 Scope Boundary Enforcement

Any feature request hitting the "OUT" list must satisfy ALL of:
1. CEO explicit approval with written rationale
2. Implementation ≤ 1 day (or it's a v2 candidate)
3. Zero new dependencies on external services
4. No UI changes beyond existing CLI patterns

---

## 2. Core Concepts — v1 Additions

### 2.1 Commitment

A **commitment** is a time-bound promise attached to a habit or standalone.

```
commitment = {
  habit_id: Option<HabitId>,  // linked to habit, or standalone
  declaration: String,        // what you promised
  deadline: Date,             // when it must be done
  success_criteria: String,   // measurable outcome
  status: pending | met | failed | excused
}
```

**Key distinction from habits:**
- Habits are recurring, open-ended ("exercise 3x/week forever")
- Commitments are discrete, deadline-bound ("run a 5K by 2026-03-01")

### 2.2 Penalty (Trap)

A **penalty** is a pre-committed consequence for failing a commitment or habit target.

```
penalty = {
  id: PenaltyId,
  commitment_id: Option<CommitmentId>,
  habit_id: Option<HabitId>,
  trigger: TriggerCondition,
  consequence: String,
  amount: Option<u32>,        // if monetary
  recipient: Option<String>,  // "charity:X" or "person:Y"
  status: armed | triggered | executed | voided
}
```

#### 2.2.1 Trigger Conditions

```
TriggerCondition = 
  | MissedDeadline(commitment_id)
  | StreakBroken(habit_id, min_streak: u32)
  | WeeklyTargetMissed(habit_id)
  | ConsecutiveMisses(habit_id, count: u32)
  | ManualTrigger
```

#### 2.2.2 Enforceability Problem

**The hard truth:** Software cannot force you to pay a penalty. We can only:
1. **Log it immutably** — the penalty was triggered, and that's on record
2. **Declare intent publicly** — social pressure via declarations
3. **Require manual acknowledgment** — can't proceed until you confirm

**Design decision:** Penalties are **self-enforced with friction**. The system tracks them, displays them prominently, and makes it *awkward* to ignore them.

### 2.3 Declaration

A **declaration** is an immutable, timestamped statement of intent.

```
declaration = {
  id: DeclId,
  timestamp: Timestamp,       // actual timestamp (exception to date-only rule)
  content: String,
  declaration_type: commitment | intent | penalty_accepted | penalty_executed | exception_claimed
  linked_ids: Vec<(EntityType, EntityId)>
}
```

**Purpose:** Create an audit trail. You can't silently delete a commitment you failed.

**Immutability:** Declarations cannot be edited or deleted. They can only be superseded by new declarations.

### 2.4 Exception

An **exception** is a pre-declared or retroactive excuse for missing a target.

```
exception = {
  id: ExceptionId,
  date_range: (Date, Date),
  reason: String,
  affects: Vec<HabitId>,
  declared_date: Date,
  is_retroactive: bool
}
```

**Rules for exceptions:**
- **Pre-declared** exceptions (before the date) don't require justification
- **Retroactive** exceptions require a declaration explaining why
- Retroactive exceptions older than 7 days are marked as `late_claim`
- Streaks are **not** broken by days covered by exceptions

### 2.5 Routine (Optional v1.1)

A **routine** groups habits that should be done together in a sequence.

```
routine = {
  id: RoutineId,
  name: String,
  habits: Vec<HabitId>,       // ordered
  schedule: Schedule,
  notes: Option<String>
}
```

**v1 scope decision:** Routines are **deferred to v1.1**. They're useful but not critical for the penalty system. Users can create pseudo-routines with naming conventions ("Morning: stretch", "Morning: journal").

---

## 3. Data Model — v1 Schema

### 3.1 Entities Overview

```
v1 Schema
├── habits[]           (from v0, unchanged)
├── checkins[]         (from v0, unchanged)
├── commitments[]      (NEW)
├── penalties[]        (NEW)
├── declarations[]     (NEW)
├── exceptions[]       (NEW)
└── meta
    ├── next_habit_number
    ├── next_commitment_number
    ├── next_penalty_number
    ├── next_declaration_number
    └── next_exception_number
```

### 3.2 Full Schema (JSON)

```json
{
  "version": 2,
  "meta": {
    "next_habit_number": 1,
    "next_commitment_number": 1,
    "next_penalty_number": 1,
    "next_declaration_number": 1,
    "next_exception_number": 1,
    "encryption": {
      "enabled": false,
      "key_derivation": null,
      "salt": null
    }
  },
  "habits": [],
  "checkins": [],
  "commitments": [
    {
      "id": "c0001",
      "habit_id": "h0001",
      "declaration": "Run a 5K without stopping",
      "deadline": "2026-03-01",
      "success_criteria": "Complete 5K run tracked by any app",
      "status": "pending",
      "created_date": "2026-01-31",
      "resolved_date": null,
      "resolution_note": null
    }
  ],
  "penalties": [
    {
      "id": "p0001",
      "commitment_id": "c0001",
      "habit_id": null,
      "trigger": {
        "type": "missed_deadline",
        "commitment_id": "c0001"
      },
      "consequence": "Donate $50 to local food bank",
      "amount": 50,
      "recipient": "charity:local_food_bank",
      "status": "armed",
      "created_date": "2026-01-31",
      "triggered_date": null,
      "executed_date": null
    }
  ],
  "declarations": [
    {
      "id": "d0001",
      "timestamp": "2026-01-31T15:30:00Z",
      "content": "I commit to running a 5K by March 1. If I fail, I will donate $50.",
      "declaration_type": "commitment",
      "linked_ids": [["commitment", "c0001"], ["penalty", "p0001"]]
    }
  ],
  "exceptions": []
}
```

### 3.3 Migration Strategy (v1 → v2)

```rust
fn migrate_v1_to_v2(db: &mut Database) {
    if db.version == 1 {
        db.version = 2;
        db.meta.next_commitment_number = 1;
        db.meta.next_penalty_number = 1;
        db.meta.next_declaration_number = 1;
        db.meta.next_exception_number = 1;
        db.meta.encryption = EncryptionMeta::default();
        db.commitments = vec![];
        db.penalties = vec![];
        db.declarations = vec![];
        db.exceptions = vec![];
    }
}
```

---

## 4. Penalty Enforcement Design

### 4.1 The Trust Problem

You control the data. You can always:
- Edit the JSON to mark a penalty as "voided"
- Delete the entire database
- Claim a fake exception

**We cannot prevent cheating. We can only make it uncomfortable.**

### 4.2 Enforcement Mechanisms

#### 4.2.1 Immutable Declaration Log

Every penalty creation, triggering, and execution is logged in `declarations[]`. Even if you modify the penalty status, the declaration remains.

```bash
$ habit declarations --type penalty
d0001  2026-01-31T15:30:00Z  commitment  "I commit to... $50 penalty"
d0015  2026-03-02T09:00:00Z  penalty_triggered  "Penalty p0001 triggered: missed deadline for c0001"
d0016  2026-03-02T09:05:00Z  penalty_executed  "Confirmed: donated $50 to local food bank"
```

#### 4.2.2 Status Dashboard Friction

```bash
$ habit status

Today (2026-03-02)
...

⚠️  PENALTIES PENDING EXECUTION (1)
- p0001: Donate $50 to local food bank
  Triggered: 2026-03-02 (missed deadline: c0001)
  Run `habit penalty execute p0001` to confirm payment
  Run `habit penalty void p0001 --reason "..."` to void (logged)
```

Penalties stay visible until explicitly resolved. The dashboard becomes annoying until you deal with it.

#### 4.2.3 Confirmation Requirements

```bash
$ habit penalty execute p0001
Are you sure you executed this penalty? (y/n): y
Enter execution note (e.g., "Paid via PayPal to food bank"): Paid via PayPal

✓ Penalty p0001 marked as executed.
Declaration logged: d0016
```

#### 4.2.4 Void with Public Shame

```bash
$ habit penalty void p0001 --reason "I decided not to pay"

⚠️  Warning: Voiding a penalty is logged permanently.
Declaration will read: "Penalty p0001 voided. Reason: I decided not to pay"
Proceed? (y/n): y

Penalty p0001 voided.
Declaration logged: d0017
```

The point: You *can* void, but it's on the record forever.

### 4.3 Automatic Penalty Triggering

Penalties are evaluated on relevant commands:

```rust
fn check_penalties(db: &mut Database, today: &str) -> Vec<PenaltyId> {
    let mut triggered = vec![];
    
    for penalty in db.penalties.iter_mut() {
        if penalty.status != "armed" { continue; }
        
        if should_trigger(&penalty.trigger, db, today) {
            penalty.status = "triggered";
            penalty.triggered_date = Some(today.to_string());
            
            // Create immutable declaration
            let decl = Declaration::penalty_triggered(&penalty);
            db.declarations.push(decl);
            
            triggered.push(penalty.id.clone());
        }
    }
    
    triggered
}
```

Evaluated on: `habit status`, `habit checkin`, `habit penalty check`.

---

## 5. Encryption Strategy

### 5.1 Requirements

1. **At-rest encryption** — DB file is unreadable without passphrase
2. **No key storage** — passphrase is never saved; entered each session or cached briefly
3. **Transparent operation** — once unlocked, CLI works normally
4. **Graceful degradation** — unencrypted mode for users who don't need it

### 5.2 Encryption Scheme

```
Passphrase → Argon2id(salt, passphrase) → 256-bit key
Key + plaintext JSON → AES-256-GCM(key, nonce, plaintext) → ciphertext
File format: MAGIC_HEADER || salt (16 bytes) || nonce (12 bytes) || ciphertext || tag (16 bytes)
```

**Implementation:**
- Use `ring` or `rust-crypto` for AES-256-GCM
- Use `argon2` crate for key derivation
- MAGIC_HEADER: `HABIT_ENC_V1` (12 bytes) for format detection

### 5.3 Key Management

```bash
# Enable encryption (one-time setup)
$ habit encrypt init
Enter passphrase: ********
Confirm passphrase: ********
✓ Database encrypted. Keep your passphrase safe — there is no recovery.

# Normal usage (passphrase required)
$ habit status
Enter passphrase: ********
Today (2026-01-31)
...

# Session caching (optional)
$ export HABITCLI_PASSPHRASE_TTL=300  # cache for 5 minutes
$ habit status
Enter passphrase: ********
[passphrase cached for 300s]

# Disable encryption
$ habit encrypt disable
Enter passphrase: ********
⚠️  This will write your data as plaintext. Continue? (y/n): y
✓ Database decrypted.
```

### 5.4 Threat Model

**Protected against:**
- Casual snooping (roommate, coworker)
- Cloud storage providers reading your data
- Device theft (if powered off)

**NOT protected against:**
- Keyloggers / compromised system
- Memory forensics while unlocked
- Sophisticated targeted attacks

**Appropriate for:** Personal accountability data, not state secrets.

---

## 6. Private GitHub Sync Strategy

### 6.1 Requirements

1. **Encrypted before upload** — GitHub sees only ciphertext
2. **Conflict-free** — single-writer assumption (your devices)
3. **Disaster recovery** — clone repo, decrypt, you're back
4. **No GitHub API dependency** — just git push/pull

### 6.2 Sync Architecture

```
Local DB (plaintext) 
    ↓ encrypt
Encrypted blob
    ↓ git add/commit/push
Private GitHub repo (ciphertext)
    ↓ git pull
Encrypted blob
    ↓ decrypt
Local DB (plaintext)
```

### 6.3 Implementation

```bash
# Setup sync (one-time)
$ habit sync init git@github.com:username/habit-data-private.git
Cloning into '.habit-sync'...
✓ Sync repository configured.

# Manual sync
$ habit sync push
Encrypting database...
Committing: "sync: 2026-01-31T15:30:00Z"
Pushing to origin...
✓ Synced.

$ habit sync pull
Pulling from origin...
Decrypting database...
✓ Local database updated.

# Auto-sync on write (optional)
$ export HABITCLI_AUTO_SYNC=1
$ habit checkin stretch
Checked in: Stretch (h0001) on 2026-01-31 +1 (total 1)
[auto-syncing...]
✓ Synced.
```

### 6.4 Conflict Resolution

**Strategy: Last-write-wins with backup.**

```rust
fn sync_pull(repo: &Repository, passphrase: &str) -> Result<(), SyncError> {
    git_pull(repo)?;
    
    let remote_blob = read_encrypted_blob(repo)?;
    let remote_db = decrypt(remote_blob, passphrase)?;
    let local_db = read_local_db()?;
    
    if remote_db.meta.last_modified > local_db.meta.last_modified {
        // Remote is newer — backup local, use remote
        backup_local_db("db.json.backup")?;
        write_local_db(remote_db)?;
        println!("Local updated from remote. Backup saved.");
    } else {
        // Local is newer or same — no action
        println!("Local is up-to-date.");
    }
    
    Ok(())
}
```

### 6.5 Repository Structure

```
habit-data-private/
├── .gitignore
├── README.md           # "Encrypted habit-cli data. Do not edit manually."
├── data.enc            # encrypted database blob
└── meta.json           # unencrypted metadata
    {
      "format_version": 1,
      "last_sync": "2026-01-31T15:30:00Z",
      "client_id": "blade-4090"  # for debugging multi-device issues
    }
```

---

## 7. CLI Surface — v1 Additions

### 7.1 Commitment Commands

```bash
habit commit <description> [--habit <habit>] [--deadline YYYY-MM-DD] [--criteria <text>]
habit commit list [--status pending|met|failed|excused] [--format table|json]
habit commit show <commitment>
habit commit resolve <commitment> --status met|failed|excused [--note <text>]
```

### 7.2 Penalty Commands

```bash
habit penalty add [--commitment <commitment>] [--habit <habit>] --trigger <trigger> --consequence <text> [--amount N]
habit penalty list [--status armed|triggered|executed|voided] [--format table|json]
habit penalty show <penalty>
habit penalty check                    # manually evaluate all armed penalties
habit penalty execute <penalty>        # confirm penalty was paid
habit penalty void <penalty> --reason <text>
```

### 7.3 Declaration Commands

```bash
habit declare <statement>              # freeform declaration
habit declarations [--type commitment|intent|penalty_accepted|...] [--from ...] [--to ...] [--format table|json]
```

### 7.4 Exception Commands

```bash
habit exception add <reason> --from YYYY-MM-DD --to YYYY-MM-DD [--habits h0001,h0002]
habit exception list [--format table|json]
```

### 7.5 Encryption Commands

```bash
habit encrypt init
habit encrypt disable
habit encrypt change-passphrase
habit encrypt status
```

### 7.6 Sync Commands

```bash
habit sync init <git-url>
habit sync push
habit sync pull
habit sync status
habit sync disconnect
```

---

## 8. Staged Roadmap

### Phase 1: Core Penalty System (v1.0)

**Duration:** 1-2 weeks  
**Goal:** Commitments + penalties + declarations working

| Task | Estimate | Priority |
|------|----------|----------|
| Schema migration (v1 → v2) | 2h | P0 |
| Commitment CRUD | 4h | P0 |
| Penalty CRUD | 4h | P0 |
| Declaration log (append-only) | 2h | P0 |
| Automatic penalty triggering | 4h | P0 |
| Status dashboard integration | 2h | P0 |
| Tests for penalty edge cases | 4h | P0 |

**Deliverable:** `habit commit`, `habit penalty`, `habit declare` commands.

### Phase 2: Exceptions (v1.1)

**Duration:** 3-4 days  
**Goal:** Exception system for legitimate misses

| Task | Estimate | Priority |
|------|----------|----------|
| Exception CRUD | 3h | P1 |
| Streak recalculation with exceptions | 4h | P1 |
| Retroactive exception rules | 2h | P1 |
| Tests | 3h | P1 |

**Deliverable:** `habit exception` commands, streaks respect exceptions.

### Phase 3: Encryption (v1.2)

**Duration:** 1 week  
**Goal:** At-rest encryption with passphrase

| Task | Estimate | Priority |
|------|----------|----------|
| Encryption module (AES-256-GCM) | 4h | P1 |
| Key derivation (Argon2id) | 2h | P1 |
| File format (magic header + blob) | 2h | P1 |
| Passphrase caching (in-memory, TTL) | 3h | P2 |
| `habit encrypt` commands | 3h | P1 |
| Tests (encrypt/decrypt roundtrip) | 3h | P1 |

**Deliverable:** `habit encrypt init|disable|status` working.

### Phase 4: GitHub Sync (v1.3)

**Duration:** 1 week  
**Goal:** Private encrypted sync via git

| Task | Estimate | Priority |
|------|----------|----------|
| Git operations wrapper (libgit2 or shell) | 4h | P1 |
| Sync repository initialization | 2h | P1 |
| Push/pull logic | 4h | P1 |
| Conflict detection + backup | 3h | P1 |
| Auto-sync on write (optional) | 2h | P2 |
| `habit sync` commands | 3h | P1 |
| Tests (mock git operations) | 4h | P1 |

**Deliverable:** `habit sync init|push|pull|status` working.

### Phase 5: Polish + Documentation (v1.4)

**Duration:** 3-4 days

| Task | Estimate | Priority |
|------|----------|----------|
| Error message improvements | 2h | P1 |
| CLI help text audit | 2h | P1 |
| Update docs/CLI_REFERENCE.md | 3h | P0 |
| Update docs/MVP_SPEC.md → V1_SPEC.md | 4h | P0 |
| README update | 2h | P1 |
| End-to-end integration tests | 4h | P1 |

---

## 9. Open Questions for CEO

1. **Penalty amounts:** Should we support recurring penalties (e.g., "$5 per missed day") or only one-shot?

2. **Declaration visibility:** Should declarations be exportable separately (for manual review/sharing)?

3. **Sync frequency:** Default to manual sync, or offer scheduled sync via external cron?

4. **Encryption passphrase recovery:** No recovery by design (secure), or allow optional recovery key?

5. **Routine priority:** Is v1.1 the right place for routines, or push to v2?

---

## 10. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Scope creep (CEO wants "one more feature") | High | High | Hard boundaries in §1.2; require explicit override |
| Encryption complexity delays launch | Medium | Medium | Phase 3 can ship after v1.0; encryption is incremental |
| Git sync edge cases (merge conflicts) | Medium | Low | Single-writer assumption; backup on conflict |
| Users don't actually enforce penalties | High | Low | Not our problem — we provide the tool, not the discipline |
| Passphrase forgotten = data loss | Medium | High | Clear warnings at setup; no backdoors by design |

---

## 11. Success Metrics (v1)

1. **Core functionality:** All Phase 1-4 commands work end-to-end
2. **Test coverage:** ≥80% for penalty triggering logic
3. **Documentation:** CLI_REFERENCE.md updated with all v1 commands
4. **Performance:** Encrypt/decrypt ≤100ms for typical DB sizes (<1MB)
5. **User feedback:** Dogfood internally for 2 weeks before public release

---

## Appendix A: Penalty Trigger Examples

```bash
# Penalty for missing a commitment deadline
habit penalty add --commitment c0001 \
  --trigger "missed_deadline" \
  --consequence "Donate $50 to charity" \
  --amount 50

# Penalty for breaking a 7-day streak
habit penalty add --habit h0001 \
  --trigger "streak_broken:7" \
  --consequence "No coffee for a week"

# Penalty for missing weekly target
habit penalty add --habit h0002 \
  --trigger "weekly_target_missed" \
  --consequence "Extra 30 min workout"

# Penalty for 3 consecutive misses
habit penalty add --habit h0001 \
  --trigger "consecutive_misses:3" \
  --consequence "Delete Twitter app for a week"
```

---

## Appendix B: Declaration Types

| Type | Created By | Purpose |
|------|------------|---------|
| `commitment` | `habit commit` | Record of making a commitment |
| `intent` | `habit declare` | Freeform statement of intent |
| `penalty_accepted` | `habit penalty add` | Record of accepting a penalty |
| `penalty_triggered` | auto | Penalty condition was met |
| `penalty_executed` | `habit penalty execute` | Penalty was paid |
| `penalty_voided` | `habit penalty void` | Penalty was voided (with reason) |
| `exception_claimed` | `habit exception add` | Exception declared |
| `commitment_resolved` | `habit commit resolve` | Commitment marked met/failed/excused |

---

## Appendix C: File Format Detection

```rust
const MAGIC_PLAINTEXT: &[u8] = b"{"; // JSON starts with {
const MAGIC_ENCRYPTED: &[u8] = b"HABIT_ENC_V1";

fn detect_format(path: &Path) -> FileFormat {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 12];
    file.read_exact(&mut magic)?;
    
    if magic.starts_with(MAGIC_ENCRYPTED) {
        FileFormat::EncryptedV1
    } else if magic[0] == b'{' {
        FileFormat::PlaintextJson
    } else {
        FileFormat::Unknown
    }
}
```

---

*End of spec. Awaiting CEO review.*
