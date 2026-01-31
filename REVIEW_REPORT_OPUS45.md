# Code Review Report — habit-cli

**Reviewer:** opus45 (AI agent)  
**Date:** 2026-01-31  
**Scope:** Developer ergonomics, test design, CI sanity, refactor suggestions

---

## Executive Summary

The habit-cli repository is in good shape for an MVP. The documentation-first approach is solid, and the implementation closely follows the spec. Tests are well-designed with graceful degradation when implementation is missing.

**Key findings:**
- ✅ Implementation is complete and passes all tests
- ⚠️ Implementation files (`src/`, `bin/`) are untracked in git
- ⚠️ Missing `bin` field in package.json (tests use heuristics to find CLI)
- ⚠️ No `.gitignore` file
- ⚠️ CI lacks npm caching (slow builds)

---

## 1. Developer Ergonomics

### 1.1 Package.json

**Issue: Missing `bin` field**

```json
// Current
{
  "name": "habit-cli",
  "scripts": { ... }
}

// Recommended
{
  "name": "habit-cli",
  "bin": {
    "habit": "./bin/habit.js"
  },
  "scripts": { ... }
}
```

The test helper `findHabitCliBin()` currently searches through multiple heuristic paths. Adding an explicit `bin` field:
- Makes the entrypoint explicit
- Enables `npm link` for local development
- Allows `npx habit` to work

**Applied:** ✅ Added in this PR

### 1.2 Missing .gitignore

No `.gitignore` exists. Should ignore:
- `node_modules/`
- `*.log`
- `.DS_Store`
- Coverage reports
- Editor configs

**Applied:** ✅ Added in this PR

### 1.3 Scripts

Current scripts are minimal but functional:
- `npm test` — runs integration tests ✅
- `npm run lint` — runs contract tests ✅
- `npm run build` — placeholder ✅

**Suggestion:** Consider adding:
```json
{
  "scripts": {
    "test:unit": "node --test test/unit/**/*.test.mjs",
    "test:integration": "node --test test/integration/**/*.test.mjs",
    "test": "node --test test/**/*.test.mjs"
  }
}
```

This would allow running unit tests separately as they're added.

---

## 2. Test Suite Design

### 2.1 Strengths

1. **Graceful degradation**: Tests skip cleanly when CLI isn't implemented
   ```js
   let suiteSkip = !bin;
   if (bin) {
     const probe = runHabit(['--help'], ...);
     // Skip if module not found or help doesn't work
   }
   ```

2. **Deterministic test environment**: Uses `--today`, `--db`, `NO_COLOR`

3. **Temp directory isolation**: Each test run gets fresh DB
   ```js
   tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'habit-cli-test-'));
   ```

4. **JSON assertions**: Uses `--format json` for machine-readable verification

### 2.2 Concerns

**Single mega-test is fragile**

The integration test is one large test that runs ~15 operations sequentially:
```js
test('add/list/show/checkin/status/stats/export + archive/unarchive ...', () => {
  // 0) list on empty
  // 1) add a daily habit
  // 2) add a weekly habit
  // ... 10 more steps
});
```

If step 5 fails, you lose visibility into whether steps 6-10 would pass.

**Recommendation:** Split into focused tests with shared setup:

```js
describe('habit-cli integration', () => {
  let dbPath, tmpDir;
  
  before(() => {
    tmpDir = fs.mkdtempSync(...);
    dbPath = path.join(tmpDir, 'db.json');
  });

  test('list returns empty array on fresh DB', () => { ... });
  test('add creates habit with correct ID format', () => { ... });
  test('list sorts habits by name', () => { ... });
  // etc.
});
```

**Not applied** (would require careful migration to avoid breaking tests)

### 2.3 Lint Tests

Current lint test is minimal:
```js
test('docs include determinism hooks', () => {
  assert.ok(text.includes('HABITCLI_DB_PATH'));
  assert.ok(text.includes('--today'));
});
```

**Suggestion:** Add more contract checks:
- Verify package.json has required fields
- Verify bin entrypoint exists and is executable
- Verify src modules export expected functions

**Applied:** ✅ Added package.json lint checks

---

## 3. CI Workflow Sanity

### 3.1 Current Workflow

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        node: [18, 20, 22]
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - name: Install
      - name: Lint
      - name: Test
      - name: Build
```

**Strengths:**
- Tests on Node 18, 20, 22 ✅
- `fail-fast: false` shows all failures ✅
- Handles both `npm ci` and `npm install` ✅

### 3.2 Missing: npm caching

Without caching, every CI run downloads dependencies fresh. For a zero-dependency project this is minor, but as deps grow:

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: ${{ matrix.node }}
    cache: 'npm'  # Add this
```

**Applied:** ✅ Added npm caching

### 3.3 Suggestion: Add timeout

Long-hanging tests can block CI indefinitely:

```yaml
- name: Test
  run: npm test
  timeout-minutes: 5
```

**Not applied** (low priority for MVP)

---

## 4. Code Quality Observations

### 4.1 Module Structure (Good)

```
src/
├── cli.js        # Main entrypoint, arg parsing
├── db.js         # Storage layer with atomic writes
├── habits.js     # Habit CRUD operations
├── checkins.js   # Check-in operations
├── schedule.js   # Schedule pattern parsing
├── date.js       # Date utilities (ISO week, etc.)
├── status.js     # Status dashboard builder
├── stats.js      # Streak/success-rate calculations
├── export.js     # JSON/CSV export
├── output.js     # Table rendering, colors
├── errors.js     # Typed CLI errors
└── stable_json.js # Deterministic JSON serialization
```

Each module has a single responsibility. Dependencies flow downward (cli → domain → utilities).

### 4.2 Error Handling (Good)

Consistent exit codes per spec:
```js
function usageError(message) {
  return new CliError(message, 2);
}
function notFoundError(message) {
  return new CliError(message, 3);
}
function ambiguousError(message) {
  return new CliError(message, 4);
}
function ioError(message) {
  return new CliError(message, 5);
}
```

### 4.3 Atomic Writes (Good)

```js
async function writeDbAtomic(dbPath, db) {
  await withWriteLock(dbPath, async () => {
    const tmpPath = path.join(dir, `.db.json.tmp.${process.pid}`);
    await fsp.writeFile(tmpPath, data, { mode: 0o600 });
    await fsp.rename(tmpPath, dbPath);
  });
}
```

Uses temp file + rename pattern for crash safety.

### 4.4 Deterministic Output (Good)

- Habits sorted by name then ID
- Dates in ascending order
- `stableStringify` for reproducible JSON

---

## 5. Recommended Refactors (Prioritized)

### P0 — Must Do

1. **Track implementation in git** — `src/` and `bin/` are untracked!
   ```bash
   git add bin/ src/
   git commit -m "feat: implement MVP CLI"
   ```

2. **Add `bin` field to package.json** — enables `npm link`, `npx`

### P1 — Should Do

3. **Add `.gitignore`** — prevent accidental commits of node_modules, logs

4. **Add npm caching to CI** — faster builds as dependencies grow

5. **Add package.json lint test** — catch missing fields early

### P2 — Nice to Have

6. **Split mega-test into focused tests** — better failure isolation

7. **Add unit tests for pure functions** — `date.js`, `schedule.js` are good candidates

8. **Add `npm run typecheck`** — JSDoc types exist, could use `tsc --noEmit`

---

## 6. Applied Changes

The following safe improvements were applied in this review:

1. ✅ Added `bin` field to package.json
2. ✅ Added `.gitignore`
3. ✅ Added `.editorconfig` for consistent formatting
4. ✅ Added npm caching to CI workflow
5. ✅ Added package.json contract tests to lint suite
6. ✅ Tracked `src/` and `bin/` in git

All tests pass after changes.

---

## Appendix: Test Output

```
$ npm test

TAP version 13
# Subtest: habit-cli integration (MVP contract)
    ok 1 - CLI entrypoint exists (or tests are skipped)
    ok 2 - add/list/show/checkin/status/stats/export + archive/unarchive
    1..2
ok 1 - habit-cli integration (MVP contract)
# Subtest: docs include determinism hooks
ok 2 - docs include determinism hooks
# Subtest: package.json has required fields
ok 3 - package.json has required fields
1..3
# tests 4
# pass 4
# fail 0
```
