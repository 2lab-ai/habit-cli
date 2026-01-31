import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

export function findHabitCliBin({ cwd } = {}) {
  const root = cwd ?? process.cwd();

  // Allow explicit override for CI/dev.
  if (process.env.HABITCLI_BIN) {
    const p = path.isAbsolute(process.env.HABITCLI_BIN)
      ? process.env.HABITCLI_BIN
      : path.join(root, process.env.HABITCLI_BIN);
    if (fs.existsSync(p)) return p;
  }

  // Try package.json "bin" mapping.
  const pkgPath = path.join(root, 'package.json');
  if (fs.existsSync(pkgPath)) {
    try {
      const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
      const bin = pkg?.bin;
      const candidate =
        typeof bin === 'string'
          ? bin
          : typeof bin === 'object'
            ? (bin.habit ?? bin['habit-cli'] ?? Object.values(bin)[0])
            : null;

      if (candidate) {
        const p = path.isAbsolute(candidate) ? candidate : path.join(root, candidate);
        if (fs.existsSync(p)) return p;
      }
    } catch {
      // ignore
    }
  }

  // Common paths used in small CLIs.
  const candidates = [
    'bin/habit.mjs',
    'bin/habit.js',
    'bin/habit-cli.mjs',
    'bin/habit-cli.js',
    'dist/habit.mjs',
    'dist/habit.js',
    'src/cli.mjs',
    'src/cli.js',
    'habit.mjs',
    'habit.js'
  ];

  for (const rel of candidates) {
    const p = path.join(root, rel);
    if (fs.existsSync(p)) return p;
  }

  return null;
}

export function runHabit(args, { cwd, env, expectExitCode } = {}) {
  const root = cwd ?? process.cwd();
  const bin = findHabitCliBin({ cwd: root });
  if (!bin) {
    const err = new Error(
      'Habit CLI entrypoint not found. Set HABITCLI_BIN or add a package.json bin mapping.'
    );
    err.code = 'HABITCLI_BIN_NOT_FOUND';
    throw err;
  }

  const ext = path.extname(bin);
  const isNodeScript = ['.js', '.mjs', '.cjs'].includes(ext);

  const command = isNodeScript ? process.execPath : bin;
  const commandArgs = isNodeScript ? [bin, ...args] : args;

  const result = spawnSync(command, commandArgs, {
    cwd: root,
    env: {
      ...process.env,
      ...env
    },
    encoding: 'utf8'
  });

  if (typeof expectExitCode === 'number' && result.status !== expectExitCode) {
    const msg = [
      `habit ${args.join(' ')} exited with ${result.status} (expected ${expectExitCode})`,
      result.stderr?.trim() ? `stderr: ${result.stderr.trim()}` : null,
      result.stdout?.trim() ? `stdout: ${result.stdout.trim()}` : null
    ]
      .filter(Boolean)
      .join('\n');
    const err = new Error(msg);
    err.result = result;
    throw err;
  }

  return result;
}

export function parseJsonStdout(result) {
  const out = (result.stdout ?? '').trim();
  if (!out) throw new Error('Expected JSON on stdout, got empty output');
  try {
    return JSON.parse(out);
  } catch (e) {
    throw new Error(`Failed to parse JSON stdout.\nstdout: ${out}\nerror: ${e?.message ?? e}`);
  }
}

export function unwrapHabits(json) {
  if (Array.isArray(json)) return json;
  if (json && Array.isArray(json.habits)) return json.habits;
  return null;
}

export function findAnyHabit(json) {
  if (!json) return null;
  if (json.id && typeof json.id === 'string') return json;
  if (json.habit && json.habit.id) return json.habit;
  if (Array.isArray(json.habits) && json.habits[0]?.id) return json.habits[0];
  return null;
}
