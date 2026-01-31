const fs = require('fs');
const fsp = require('fs/promises');
const os = require('os');
const path = require('path');

const { ioError } = require('./errors');
const { stableStringify } = require('./stable_json');

function defaultDb() {
  return {
    version: 1,
    meta: {
      next_habit_number: 1
    },
    habits: [],
    checkins: []
  };
}

function resolveDbPath(cliDbPath) {
  const byArg = cliDbPath && String(cliDbPath).trim();
  if (byArg) return byArg;

  const byEnv = process.env.HABITCLI_DB_PATH && String(process.env.HABITCLI_DB_PATH).trim();
  if (byEnv) return byEnv;

  const xdg = process.env.XDG_DATA_HOME && String(process.env.XDG_DATA_HOME).trim();
  const base = xdg || path.join(os.homedir(), '.local', 'share');
  return path.join(base, 'habit-cli', 'db.json');
}

function validateDbShape(db) {
  if (!db || typeof db !== 'object') throw ioError('DB corrupted');
  if (db.version !== 1) throw ioError('DB corrupted');
  if (!db.meta || typeof db.meta !== 'object') throw ioError('DB corrupted');
  if (!Number.isInteger(db.meta.next_habit_number) || db.meta.next_habit_number < 1) throw ioError('DB corrupted');
  if (!Array.isArray(db.habits) || !Array.isArray(db.checkins)) throw ioError('DB corrupted');
}

async function readDb(dbPath) {
  try {
    const txt = await fsp.readFile(dbPath, 'utf8');
    const db = JSON.parse(txt);
    validateDbShape(db);
    return db;
  } catch (err) {
    if (err && err.code === 'ENOENT') return defaultDb();
    if (err instanceof SyntaxError) throw ioError('DB corrupted');
    throw ioError('DB IO error');
  }
}

async function ensureParentDir(dbPath) {
  const dir = path.dirname(dbPath);
  await fsp.mkdir(dir, { recursive: true, mode: 0o700 });
  // Best-effort permissions hardening.
  try { await fsp.chmod(dir, 0o700); } catch (_) {}
}

async function withWriteLock(dbPath, fn) {
  const lockPath = `${dbPath}.lock`;
  let fd;
  try {
    fd = await fsp.open(lockPath, 'wx', 0o600);
  } catch (err) {
    if (err && err.code === 'EEXIST') throw ioError('DB is locked');
    throw ioError('DB IO error');
  }

  try {
    return await fn();
  } finally {
    try { await fd.close(); } catch (_) {}
    try { await fsp.unlink(lockPath); } catch (_) {}
  }
}

async function writeDbAtomic(dbPath, db) {
  validateDbShape(db);
  await ensureParentDir(dbPath);

  await withWriteLock(dbPath, async () => {
    const dir = path.dirname(dbPath);
    const tmpPath = path.join(dir, `.db.json.tmp.${process.pid}`);
    const data = stableStringify(db) + '\n';

    try {
      await fsp.writeFile(tmpPath, data, { mode: 0o600 });
      await fsp.rename(tmpPath, dbPath);
      try { await fsp.chmod(dbPath, 0o600); } catch (_) {}
    } catch (err) {
      try { await fsp.unlink(tmpPath); } catch (_) {}
      throw ioError('DB IO error');
    }
  });
}

async function updateDb(dbPath, mutatorFn) {
  const db = await readDb(dbPath);
  await ensureParentDir(dbPath);
  await withWriteLock(dbPath, async () => {
    // Re-read inside lock to avoid lost update.
    const latest = await readDb(dbPath);
    const updated = mutatorFn(latest) || latest;
    validateDbShape(updated);
    const dir = path.dirname(dbPath);
    const tmpPath = path.join(dir, `.db.json.tmp.${process.pid}`);
    const data = stableStringify(updated) + '\n';
    try {
      await fsp.writeFile(tmpPath, data, { mode: 0o600 });
      await fsp.rename(tmpPath, dbPath);
      try { await fsp.chmod(dbPath, 0o600); } catch (_) {}
    } catch (err) {
      try { await fsp.unlink(tmpPath); } catch (_) {}
      throw ioError('DB IO error');
    }
  });
}

module.exports = {
  defaultDb,
  resolveDbPath,
  readDb,
  updateDb,
  writeDbAtomic
};
