#!/usr/bin/env node

const { runCli } = require('../src/cli');

runCli(process.argv.slice(2)).catch((err) => {
  // runCli should already have formatted errors; this is a last resort.
  // Keep stderr single-line + stable for tests/logs.
  const msg = (err && err.message) ? String(err.message) : String(err);
  const line = msg.split(/\r?\n/)[0];

  const exitCode = (err && typeof err.exitCode === 'number') ? err.exitCode : 5;

  process.stderr.write(line + "\n");
  process.exit(exitCode);
});
