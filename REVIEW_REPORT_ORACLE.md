# Habit CLI — Code Review Report (Oracle)

Repo: https://github.com/2lab-ai/habit-cli
Local: `/home/zhugehyuk/2lab.ai/habit-cli`

Review focus:
- Alignment with `docs/MVP_SPEC.md` + `docs/CLI_REFERENCE.md`
- Quality of test/CI scaffolding (skip logic, entrypoint detection, determinism)
- Missing pieces needed for implementation to plug in cleanly (`package.json` bin, CLI contract)
- Security/privacy considerations

Date: 2026-01-31

---

## Executive summary

The implementation is broadly aligned with the MVP spec/CLI reference:
- Commands and flags match the contract.
- Determinism hooks exist (`--today`, `--db`, stable IDs, stable sorts, stable JSON).
- Local storage is implemented with atomic writes, best-effort locking, and restrictive file permissions.

The most important remaining work is contract hardening:
- tighten integration test skip logic so it doesn’t hide real regressions
- optionally pin canonical JSON envelope shapes per command in `CLI_REFERENCE.md`

---

## 1) Alignment with docs

### 1.1 MVP command surface
Implemented commands: `add`, `list`, `show`, `archive`, `unarchive`, `checkin`, `status`, `stats`, `export`.

Key behaviors matching the spec/reference:
- Habit selector: exact `hNNNN` or unique case-insensitive prefix.
  - ambiguous selector exits `4`.
  - not found exits `3`.
- Deterministic dates via `--today` / `HABITCLI_TODAY`.
- DB override via `--db` / `HABITCLI_DB_PATH`.
- `checkin` supports additive updates plus deterministic correction (`--set`, `--delete`).
- `export` supports JSON to stdout/file and CSV to directory.

### 1.2 Stats default windows
Docs specify:
- day-period: last 30 days ending today
- week-period: last 12 ISO weeks ending this week

Implementation now matches this **per habit** (even when running `habit stats` with no selector over a mix of periods).

### 1.3 Output determinism
- Habits are stably sorted by name then id.
- Check-ins are stably sorted.
- JSON output is stable-key sorted.

Open question:
- JSON envelope shapes vary by command (e.g. `{ habits: [...] }`, `{ habit: ... }`). This is fine for MVP but can be pinned explicitly in the CLI reference to prevent downstream drift.

---

## 2) Test scaffolding review

### Strengths
- Integration tests are true E2E (spawn the CLI).
- Deterministic test setup:
  - temp DB
  - pinned today
  - `NO_COLOR`
- Entrypoint detection supports:
  - `HABITCLI_BIN` override
  - `package.json` `bin`
  - common fallback paths

### Remaining concern: skip logic
Integration tests currently skip the suite when help output doesn’t “look like help”. This can mask regressions once the CLI exists.

Recommendation:
- Keep skip only for "entrypoint not found" or "module missing".
- If the CLI runs but help changes unexpectedly, the suite should fail.

---

## 3) CI scaffolding review

- GitHub Actions matrix: Node 18/20/22.
- Runs: lint → test → build placeholder.

Suggestion:
- If dependencies are added later, commit a lockfile to ensure deterministic installs.

---

## 4) Security & privacy considerations

Current good practices:
- No network calls / telemetry.
- DB dir best-effort `0700`; DB file best-effort `0600`.
- Export outputs written with `0600`.
- Atomic writes + lock file reduce corruption and concurrent-writer risk.

Potential improvements:
- For CSV export directory: consider best-effort chmod to `0700` after mkdir (mirrors DB dir hardening).
- Ensure error messages never dump full DB contents; keep errors single-line (already done in entrypoint wrapper).

---

## Prioritized fix list

### P0
1) Tighten integration test skip logic (avoid silently skipping contract regressions).

### P1
2) Pin canonical JSON output shapes per command in `docs/CLI_REFERENCE.md`.
3) Export dir hardening: best-effort chmod `0700` on CSV export directory.

### P2
4) CI: add concurrency cancellation (optional QoL).

---

## Changes applied during this review (already in repo)

- CLI entrypoint now propagates `CliError.exitCode` and keeps stderr single-line.
- Integration tests include explicit exit-code assertions for usage/not-found/ambiguous.
- Stats defaults are computed per habit (30 days vs 12 ISO weeks) to match the written contract.
- README expanded with quickstart and deterministic usage notes.
