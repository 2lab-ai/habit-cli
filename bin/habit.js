#!/usr/bin/env node

const { runCli } = require('../src/cli');

runCli(process.argv.slice(2)).catch((err) => {
  // runCli should already have formatted errors; this is a last resort.
  const msg = (err && err.message) ? err.message : String(err);
  process.stderr.write(msg + "\n");
  process.exit(5);
});
