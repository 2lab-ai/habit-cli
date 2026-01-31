const fs = require('fs/promises');

const { CliError, usageError, ioError } = require('./errors');
const { resolveDbPath, readDb, updateDb } = require('./db');
const { systemToday, parseDateString, isoWeekStart, addDays } = require('./date');
const { makeStyler, renderSimpleTable } = require('./output');
const { stableStringify } = require('./stable_json');
const { nextHabitId, listHabits, selectHabit, makeHabit } = require('./habits');
const { scheduleToString } = require('./schedule');
const { getQuantity, addQuantity, setQuantity, listCheckinsForHabit, listCheckinsInRange } = require('./checkins');
const { buildStatus } = require('./status');
const { buildStats } = require('./stats');
const { exportCsvToDir } = require('./export');

const COMMANDS = ['add', 'list', 'show', 'archive', 'unarchive', 'checkin', 'status', 'stats', 'export'];

function printJson(obj) {
  process.stdout.write(stableStringify(obj) + '\n');
}

function printLine(s) {
  process.stdout.write(String(s) + '\n');
}

function printErrLine(s) {
  process.stderr.write(String(s) + '\n');
}

function pickCommandIndex(argv) {
  for (let i = 0; i < argv.length; i++) {
    if (COMMANDS.includes(argv[i])) return i;
  }
  return -1;
}

function parseGlobalOptsFromArgs(args, { allowFormat } = {}) {
  /** @type {{db?: string, today?: string, format?: string, noColor?: boolean, help?: boolean}} */
  const opts = {};
  const rest = [];

  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === '--help' || a === '-h') {
      opts.help = true;
      continue;
    }
    if (a === '--no-color') {
      opts.noColor = true;
      continue;
    }
    if (a === '--db') {
      const v = args[++i];
      if (!v) throw usageError('Missing value for --db');
      opts.db = v;
      continue;
    }
    if (a === '--today') {
      const v = args[++i];
      if (!v) throw usageError('Missing value for --today');
      opts.today = v;
      continue;
    }
    if (allowFormat && a === '--format') {
      const v = args[++i];
      if (!v) throw usageError('Missing value for --format');
      if (v !== 'table' && v !== 'json') throw usageError(`Invalid format: ${v}`);
      opts.format = v;
      continue;
    }

    rest.push(a);
  }

  return { opts, rest };
}

function parseCommandOpts(args, { allowed = {} } = {}) {
  /** @type {Record<string, any>} */
  const opts = {};
  /** @type {string[]} */
  const positionals = [];

  for (let i = 0; i < args.length; i++) {
    const a = args[i];

    if (a === '--help' || a === '-h') {
      opts.help = true;
      continue;
    }

    if (a.startsWith('--')) {
      const key = a.slice(2);
      const spec = allowed[key];
      if (!spec) throw usageError(`Unknown option: ${a}`);

      if (spec.type === 'boolean') {
        opts[key] = true;
        continue;
      }

      const v = args[++i];
      if (!v) throw usageError(`Missing value for ${a}`);

      if (spec.type === 'enum') {
        if (!spec.values.includes(v)) throw usageError(`Invalid ${key}: ${v}`);
        opts[key] = v;
        continue;
      }

      // string/number (validated later)
      opts[key] = v;
      continue;
    }

    positionals.push(a);
  }

  return { opts, positionals };
}

function resolveToday(cliToday) {
  const byArg = cliToday && String(cliToday).trim();
  const byEnv = process.env.HABITCLI_TODAY && String(process.env.HABITCLI_TODAY).trim();
  const today = byArg || byEnv || systemToday();
  parseDateString(today, 'today');
  return today;
}

function resolveColorEnabled(noColorFlag) {
  if (noColorFlag) return false;
  if (process.env.NO_COLOR != null) return false;
  return true;
}

function usage() {
  return [
    'habit — local habit tracking CLI',
    '',
    'Usage:',
    '  habit [global options] <command> [options]',
    '',
    'Commands:',
    `  ${COMMANDS.join(', ')}`,
    '',
    'Global options:',
    '  --db <path>',
    '  --today <YYYY-MM-DD>',
    '  --format table|json',
    '  --no-color',
    '  --help',
    ''
  ].join('\n');
}

async function runCli(argv) {
  const cmdIndex = pickCommandIndex(argv);
  const hasHelp = argv.includes('--help') || argv.includes('-h');

  if (cmdIndex === -1) {
    if (hasHelp || argv.length === 0) {
      printLine(usage());
      return;
    }
    throw usageError('Missing command');
  }

  const cmd = argv[cmdIndex];
  const pre = argv.slice(0, cmdIndex);
  const post = argv.slice(cmdIndex + 1);

  const preParsed = parseGlobalOptsFromArgs(pre, { allowFormat: true });
  if (preParsed.rest.length > 0) throw usageError('Invalid arguments');

  const postParsed = parseGlobalOptsFromArgs(post, { allowFormat: false });
  const globalOpts = {
    ...preParsed.opts,
    ...postParsed.opts
  };

  if (globalOpts.help) {
    printLine(usage());
    return;
  }

  const dbPath = resolveDbPath(globalOpts.db);
  const today = resolveToday(globalOpts.today);
  const colorEnabled = resolveColorEnabled(globalOpts.noColor);
  const styler = makeStyler({ colorEnabled });

  const cmdArgs = postParsed.rest;

  // Helper: print habit list row
  function renderHabitRow(h) {
    return {
      id: h.id,
      name: h.name,
      schedule: scheduleToString(h.schedule),
      target: `${h.target.quantity}/${h.target.period}`,
      archived: h.archived ? 'yes' : 'no'
    };
  }

  if (cmd === 'add') {
    const { opts, positionals } = parseCommandOpts(cmdArgs, {
      allowed: {
        schedule: { type: 'string' },
        period: { type: 'string' },
        target: { type: 'string' },
        notes: { type: 'string' },
        format: { type: 'enum', values: ['table', 'json'] }
      }
    });
    const format = opts.format || globalOpts.format || 'table';
    if (opts.help) throw usageError('Usage: habit add <name> [--schedule ...] [--target N] [--period day|week] [--notes ...]');
    const name = positionals[0];
    if (!name) throw usageError('Habit name is required');
    if (positionals.length > 1) throw usageError('Too many arguments');

    let created;
    await updateDb(dbPath, (db) => {
      const id = nextHabitId(db);
      const habit = makeHabit({
        id,
        name,
        schedulePattern: opts.schedule || 'everyday',
        period: opts.period || 'day',
        target: opts.target == null ? 1 : Number(opts.target),
        notes: opts.notes,
        today
      });
      db.habits.push(habit);
      created = habit;
      return db;
    });

    if (format === 'json') {
      printJson({ habit: created });
    } else {
      printLine(renderSimpleTable([renderHabitRow(created)], [
        { header: 'id', value: (r) => r.id },
        { header: 'name', value: (r) => r.name },
        { header: 'schedule', value: (r) => r.schedule },
        { header: 'target', value: (r) => r.target }
      ]));
    }
    return;
  }

  if (cmd === 'list') {
    const { opts, positionals } = parseCommandOpts(cmdArgs, {
      allowed: {
        all: { type: 'boolean' },
        format: { type: 'enum', values: ['table', 'json'] }
      }
    });
    const format = opts.format || globalOpts.format || 'table';
    if (opts.help) throw usageError('Usage: habit list [--all] [--format table|json]');
    if (positionals.length > 0) throw usageError('Too many arguments');

    const db = await readDb(dbPath);
    const habits = listHabits(db, { includeArchived: Boolean(opts.all) });

    if (format === 'json') {
      printJson({ habits });
    } else {
      const rows = habits.map(renderHabitRow);
      printLine(renderSimpleTable(rows, [
        { header: 'id', value: (r) => r.id },
        { header: 'name', value: (r) => r.name },
        { header: 'schedule', value: (r) => r.schedule },
        { header: 'target', value: (r) => r.target },
        { header: 'archived', value: (r) => r.archived }
      ]));
    }
    return;
  }

  if (cmd === 'show') {
    const { opts, positionals } = parseCommandOpts(cmdArgs, {
      allowed: {
        format: { type: 'enum', values: ['table', 'json'] }
      }
    });
    const format = opts.format || globalOpts.format || 'table';
    if (opts.help) throw usageError('Usage: habit show <habit>');
    const sel = positionals[0];
    if (!sel) throw usageError('Habit selector is required');
    if (positionals.length > 1) throw usageError('Too many arguments');

    const db = await readDb(dbPath);
    const habit = selectHabit(db, sel, { includeArchived: true });
    const checkins = listCheckinsForHabit(db, habit.id);

    if (format === 'json') {
      printJson({ habit, checkins });
    } else {
      printLine(`${habit.name} (${habit.id})`);
      printLine(`schedule: ${scheduleToString(habit.schedule)}`);
      printLine(`target: ${habit.target.quantity}/${habit.target.period}`);
      printLine(`archived: ${habit.archived ? 'yes' : 'no'}`);
      printLine(`created_date: ${habit.created_date}`);
      if (habit.archived_date) printLine(`archived_date: ${habit.archived_date}`);
      if (habit.notes) printLine(`notes: ${habit.notes}`);
      if (checkins.length) {
        printLine('checkins:');
        for (const c of checkins) {
          printLine(`- ${c.date} ${c.quantity}`);
        }
      }
    }
    return;
  }

  if (cmd === 'archive' || cmd === 'unarchive') {
    const { opts, positionals } = parseCommandOpts(cmdArgs, {
      allowed: {
        format: { type: 'enum', values: ['table', 'json'] }
      }
    });
    const format = opts.format || globalOpts.format || 'table';
    if (opts.help) throw usageError(`Usage: habit ${cmd} <habit>`);
    const sel = positionals[0];
    if (!sel) throw usageError('Habit selector is required');
    if (positionals.length > 1) throw usageError('Too many arguments');

    let updated;
    await updateDb(dbPath, (db) => {
      const habit = selectHabit(db, sel, { includeArchived: true });
      if (cmd === 'archive') {
        habit.archived = true;
        habit.archived_date = habit.archived_date || today;
      } else {
        habit.archived = false;
        habit.archived_date = null;
      }
      updated = habit;
      return db;
    });

    if (format === 'json') {
      printJson({ habit: updated });
    } else {
      const action = cmd === 'archive' ? 'Archived' : 'Unarchived';
      printLine(`${action}: ${updated.name} (${updated.id})`);
    }
    return;
  }

  if (cmd === 'checkin') {
    const { opts, positionals } = parseCommandOpts(cmdArgs, {
      allowed: {
        date: { type: 'string' },
        qty: { type: 'string' },
        set: { type: 'string' },
        delete: { type: 'boolean' },
        format: { type: 'enum', values: ['table', 'json'] }
      }
    });
    const format = opts.format || globalOpts.format || 'table';
    if (opts.help) throw usageError('Usage: habit checkin <habit> [--date YYYY-MM-DD] [--qty N] [--set N] [--delete]');
    const sel = positionals[0];
    if (!sel) throw usageError('Habit selector is required');
    if (positionals.length > 1) throw usageError('Too many arguments');

    const date = opts.date || today;
    parseDateString(date);

    const hasQty = Object.prototype.hasOwnProperty.call(opts, 'qty');
    const hasSet = Object.prototype.hasOwnProperty.call(opts, 'set');
    const hasDelete = Boolean(opts.delete);

    if (hasDelete && (hasQty || hasSet)) throw usageError('Invalid flags: --delete conflicts with --qty/--set');
    if (hasQty && hasSet) throw usageError('Invalid flags: --qty conflicts with --set');

    const qty = hasQty ? Number(opts.qty) : 1;
    const set = hasSet ? Number(opts.set) : null;

    if (hasQty && (!Number.isInteger(qty) || qty < 1)) throw usageError('Invalid quantity');
    if (hasSet && (!Number.isInteger(set) || set < 0)) throw usageError('Invalid quantity');

    let result;
    await updateDb(dbPath, (db) => {
      const habit = selectHabit(db, sel, { includeArchived: true });

      if (hasDelete) {
        const prev = getQuantity(db, habit.id, date);
        setQuantity(db, habit.id, date, 0);
        result = { habit, date, action: 'delete', previous_quantity: prev, quantity: 0 };
        return db;
      }

      if (hasSet) {
        const prev = getQuantity(db, habit.id, date);
        setQuantity(db, habit.id, date, set);
        result = { habit, date, action: 'set', previous_quantity: prev, quantity: set };
        return db;
      }

      const prev = getQuantity(db, habit.id, date);
      const total = addQuantity(db, habit.id, date, qty);
      result = { habit, date, action: 'add', delta: qty, previous_quantity: prev, quantity: total };
      return db;
    });

    if (format === 'json') {
      printJson({
        habit: { id: result.habit.id, name: result.habit.name },
        date: result.date,
        action: result.action,
        quantity: result.quantity,
        delta: result.delta || null
      });
    } else {
      if (result.action === 'delete') {
        printLine(`Deleted check-in: ${result.habit.name} (${result.habit.id}) on ${result.date}`);
      } else if (result.action === 'set') {
        printLine(`Set check-in: ${result.habit.name} (${result.habit.id}) on ${result.date} =${result.quantity}`);
      } else {
        printLine(`Checked in: ${result.habit.name} (${result.habit.id}) on ${result.date} +${result.delta} (total ${result.quantity})`);
      }
    }
    return;
  }

  if (cmd === 'status') {
    const { opts, positionals } = parseCommandOpts(cmdArgs, {
      allowed: {
        date: { type: 'string' },
        'week-of': { type: 'string' },
        'include-archived': { type: 'boolean' },
        format: { type: 'enum', values: ['table', 'json'] }
      }
    });
    const format = opts.format || globalOpts.format || 'table';
    if (opts.help) throw usageError('Usage: habit status [--date YYYY-MM-DD] [--week-of YYYY-MM-DD] [--include-archived]');
    if (positionals.length > 0) throw usageError('Too many arguments');

    const date = opts.date || today;
    parseDateString(date);
    const weekOf = opts['week-of'] || null;
    if (weekOf) parseDateString(weekOf, 'week-of');

    const db = await readDb(dbPath);
    const data = buildStatus(db, { date, weekOf, includeArchived: Boolean(opts['include-archived']) });

    if (format === 'json') {
      printJson(data);
    } else {
      printLine(`Today (${data.today.date})`);
      if (data.today.habits.length === 0) {
        printLine(styler.gray('(no scheduled habits)'));
      } else {
        for (const h of data.today.habits) {
          const mark = h.done ? styler.green('[x]') : '[ ]';
          const progress = h.period === 'day'
            ? `${h.quantity}/${h.target}`
            : `${h.quantity}/${h.target} (weekly)`;
          printLine(`- ${mark} ${h.name} ${progress}`);
        }
      }

      printLine('');
      printLine(`This week (${data.week.id})`);
      for (const h of data.week.habits) {
        if (h.period === 'day') {
          printLine(`- ${h.name} ${h.done_scheduled_days}/${h.scheduled_days} scheduled days done`);
        } else {
          printLine(`- ${h.name} ${h.quantity}/${h.target} (weekly)`);
        }
      }
    }
    return;
  }

  if (cmd === 'stats') {
    const { opts, positionals } = parseCommandOpts(cmdArgs, {
      allowed: {
        from: { type: 'string' },
        to: { type: 'string' },
        format: { type: 'enum', values: ['table', 'json'] }
      }
    });
    const format = opts.format || globalOpts.format || 'table';
    if (opts.help) throw usageError('Usage: habit stats [<habit>] [--from YYYY-MM-DD] [--to YYYY-MM-DD]');

    const db = await readDb(dbPath);

    const selector = positionals[0] || null;
    if (positionals.length > 1) throw usageError('Too many arguments');

    let habits;
    if (selector) {
      habits = [selectHabit(db, selector, { includeArchived: true })];
    } else {
      habits = db.habits.filter((h) => !h.archived);
    }

    // Determine range defaults based on habit periods when selector omitted.
    let from = opts.from || null;
    let to = opts.to || null;
    if (to) parseDateString(to, 'to');
    if (from) parseDateString(from, 'from');

    const toEff = to || today;

    if (!from) {
      // If multiple habits with mixed periods, choose a broad window (30 days).
      const allWeek = habits.every((h) => h.target.period === 'week');
      if (allWeek) {
        // last 12 weeks ending this week
        const endWeek = isoWeekStart(toEff);
        from = addDays(endWeek, -7 * (12 - 1));
        to = addDays(endWeek, 6);
      } else {
        from = addDays(toEff, -29);
        to = toEff;
      }
    } else {
      to = toEff;
    }

    const rows = buildStats(db, habits, { from, to });

    if (format === 'json') {
      printJson({ stats: rows });
    } else {
      const tableRows = rows.map((r) => {
        const rate = r.success_rate.eligible === 0 ? 'n/a' : `${Math.round(r.success_rate.rate * 100)}%`;
        return {
          id: r.habit_id,
          name: r.name,
          period: r.period,
          current: r.current_streak,
          longest: r.longest_streak,
          success: `${rate} (${r.success_rate.successes}/${r.success_rate.eligible})`
        };
      });
      printLine(renderSimpleTable(tableRows, [
        { header: 'id', value: (x) => x.id },
        { header: 'name', value: (x) => x.name },
        { header: 'period', value: (x) => x.period },
        { header: 'current', value: (x) => x.current },
        { header: 'longest', value: (x) => x.longest },
        { header: 'success', value: (x) => x.success }
      ]));
    }
    return;
  }

  if (cmd === 'export') {
    const { opts, positionals } = parseCommandOpts(cmdArgs, {
      allowed: {
        format: { type: 'enum', values: ['json', 'csv'] },
        out: { type: 'string' },
        from: { type: 'string' },
        to: { type: 'string' },
        'include-archived': { type: 'boolean' }
      }
    });
    if (opts.help) throw usageError('Usage: habit export --format json|csv [--out <path>] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--include-archived]');
    if (positionals.length > 0) throw usageError('Too many arguments');

    const format = opts.format;
    if (!format) throw usageError('Missing required --format');
    if (format !== 'json' && format !== 'csv') throw usageError(`Invalid format: ${format}`);

    const from = opts.from || null;
    const to = opts.to || null;
    if (from) parseDateString(from, 'from');
    if (to) parseDateString(to, 'to');
    if (from && to && from > to) throw usageError('Invalid range: from > to');

    const out = opts.out || null;

    const db = await readDb(dbPath);
    const habits = listHabits(db, { includeArchived: Boolean(opts['include-archived']) });
    const habitIds = new Set(habits.map((h) => h.id));
    const checkins = listCheckinsInRange(db, { from, to, habitIds });

    if (format === 'json') {
      const payload = { version: 1, habits, checkins };
      const data = stableStringify(payload) + '\n';
      if (out) {
        await fs.writeFile(out, data, { mode: 0o600 });
      } else {
        process.stdout.write(data);
      }
      return;
    }

    // CSV
    if (!out) throw usageError('CSV export requires --out <dir>');
    await exportCsvToDir({ outDir: out, habits, checkins });
    return;
  }

  throw usageError(`Unknown command: ${cmd}`);
}

module.exports = { runCli };

// Top-level error handler used by bin/habit.js
module.exports._internal = {
  usage
};
