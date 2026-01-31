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

test('package.json has required fields for CLI distribution', () => {
  const pkgPath = path.join(process.cwd(), 'package.json');
  assert.ok(fs.existsSync(pkgPath), 'expected package.json to exist');

  const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));

  // Basic metadata
  assert.ok(pkg.name, 'package.json should have a name');
  assert.ok(pkg.version, 'package.json should have a version');
  assert.ok(pkg.description, 'package.json should have a description');
  assert.ok(pkg.license, 'package.json should have a license');

  // Engine requirements
  assert.ok(pkg.engines?.node, 'package.json should specify node engine version');

  // CLI entrypoint
  assert.ok(pkg.bin, 'package.json should have a bin field for CLI');
  const binPath = typeof pkg.bin === 'string' ? pkg.bin : Object.values(pkg.bin)[0];
  assert.ok(binPath, 'bin field should specify an entrypoint');

  // Verify bin file exists
  const fullBinPath = path.join(process.cwd(), binPath);
  assert.ok(fs.existsSync(fullBinPath), `bin entrypoint should exist: ${binPath}`);

  // Required scripts
  assert.ok(pkg.scripts?.test, 'package.json should have a test script');
});
