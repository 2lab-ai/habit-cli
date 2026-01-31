const { ambiguousError, notFoundError, usageError } = require('./errors');
const { parseSchedulePattern, scheduleToString, validateSchedule } = require('./schedule');
const { isoWeekday } = require('./date');

function normalizeName(name) {
  return String(name || '').trim();
}

function validateHabitName(name) {
  const n = normalizeName(name);
  if (!n) throw usageError('Habit name is required');
  return n;
}

function nextHabitId(db) {
  const n = db.meta.next_habit_number;
  const id = 'h' + String(n).padStart(4, '0');
  db.meta.next_habit_number = n + 1;
  return id;
}

function stableHabitSort(a, b) {
  const an = a.name.toLowerCase();
  const bn = b.name.toLowerCase();
  if (an < bn) return -1;
  if (an > bn) return 1;
  if (a.id < b.id) return -1;
  if (a.id > b.id) return 1;
  return 0;
}

function listHabits(db, { includeArchived }) {
  return db.habits
    .filter((h) => includeArchived ? true : !h.archived)
    .slice()
    .sort(stableHabitSort);
}

function selectHabit(db, selector, { includeArchived = true } = {}) {
  const s = String(selector || '').trim();
  if (!s) throw usageError('Habit selector is required');

  if (/^h\d{4}$/.test(s)) {
    const h = db.habits.find((x) => x.id === s);
    if (!h || (!includeArchived && h.archived)) throw notFoundError(`Habit not found: ${selector}`);
    return h;
  }

  const prefix = s.toLowerCase();
  const matches = db.habits
    .filter((h) => (includeArchived ? true : !h.archived))
    .filter((h) => h.name.toLowerCase().startsWith(prefix))
    .slice()
    .sort(stableHabitSort);

  if (matches.length === 0) throw notFoundError(`Habit not found: ${selector}`);
  if (matches.length > 1) {
    const candidates = matches.map((h) => `${h.id} ${h.name}`).join(', ');
    throw ambiguousError(`Ambiguous habit selector '${selector}': ${candidates}`);
  }

  return matches[0];
}

function makeHabit({ id, name, schedulePattern, period, target, notes, today }) {
  const habitName = validateHabitName(name);
  const schedule = parseSchedulePattern(schedulePattern || 'everyday');
  validateSchedule(schedule);

  const p = period || 'day';
  if (p !== 'day' && p !== 'week') throw usageError(`Invalid period: ${period}`);

  const t = target == null ? 1 : Number(target);
  if (!Number.isInteger(t) || t < 1) throw usageError('Invalid target');

  const n = notes == null ? null : String(notes);

  return {
    id,
    name: habitName,
    schedule,
    target: {
      period: p,
      quantity: t
    },
    notes: n,
    archived: false,
    created_date: today,
    archived_date: null
  };
}

function habitToListRow(habit) {
  return {
    id: habit.id,
    name: habit.name,
    schedule: scheduleToString(habit.schedule),
    period: habit.target.period,
    target: habit.target.quantity,
    archived: habit.archived ? 'yes' : 'no'
  };
}

function isScheduledOn(habit, dateStr) {
  if (dateStr < habit.created_date) return false;
  const wd = isoWeekday(dateStr);
  return habit.schedule.days.includes(wd);
}

module.exports = {
  nextHabitId,
  listHabits,
  selectHabit,
  makeHabit,
  habitToListRow,
  stableHabitSort,
  isScheduledOn
};
