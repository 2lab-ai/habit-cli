import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

// Lint = lightweight, dependency-free contract checks.
// This is intentionally minimal; it helps keep the repo deterministic and
// documents-first until implementation lands.

test('docs include determinism hooks (HABITCLI_DB_PATH and --today)', () => {
  const specPath = path.join(process.cwd(), 'docs', 'MVP_SPEC.md');
  assert.ok(fs.existsSync(specPath), 'expected docs/MVP_SPEC.md to exist');

  const text = fs.readFileSync(specPath, 'utf8');
  assert.ok(text.includes('HABITCLI_DB_PATH'));
  assert.ok(text.includes('--today'));
});
