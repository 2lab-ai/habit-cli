// Minimal build hook.
//
// This repo is currently docs-first. Once an implementation exists (e.g. TS/JS build
// output), replace this file with the real build step.
import fs from 'node:fs';

const hasSrc = fs.existsSync(new URL('../src', import.meta.url));
const hasBin = fs.existsSync(new URL('../bin', import.meta.url));

if (!hasSrc && !hasBin) {
  console.log('[build] No build step yet (no src/ or bin/ directory found).');
  process.exit(0);
}

console.log('[build] Nothing to build yet. (Placeholder)');
