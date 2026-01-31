const { usageError } = require('./errors');

const DAY_NAME_TO_ISO = {
  mon: 1,
  tue: 2,
  wed: 3,
  thu: 4,
  fri: 5,
  sat: 6,
  sun: 7
};

const ISO_TO_DAY_NAME = {
  1: 'mon',
  2: 'tue',
  3: 'wed',
  4: 'thu',
  5: 'fri',
  6: 'sat',
  7: 'sun'
};

function parseSchedulePattern(patternRaw) {
  const pattern = String(patternRaw || '').trim().toLowerCase();
  if (!pattern) throw usageError('Invalid schedule pattern');

  /** @type {number[]} */
  let days;
  if (pattern === 'everyday') days = [1, 2, 3, 4, 5, 6, 7];
  else if (pattern === 'weekdays') days = [1, 2, 3, 4, 5];
  else if (pattern === 'weekends') days = [6, 7];
  else {
    const parts = pattern.split(',').map((p) => p.trim()).filter(Boolean);
    if (parts.length === 0) throw usageError(`Invalid schedule pattern: ${patternRaw}`);
    days = [];
    for (const p of parts) {
      const iso = DAY_NAME_TO_ISO[p];
      if (!iso) throw usageError(`Invalid schedule pattern: ${patternRaw}`);
      if (!days.includes(iso)) days.push(iso);
    }
    days.sort((a, b) => a - b);
  }

  return { type: 'days_of_week', days };
}

function scheduleToString(schedule) {
  const days = (schedule && schedule.days) ? schedule.days.slice().sort((a, b) => a - b) : [];
  const isEveryday = days.length === 7 && days.every((d, i) => d === i + 1);
  const isWeekdays = days.length === 5 && days.every((d, i) => d === i + 1);
  const isWeekends = days.length === 2 && days[0] === 6 && days[1] === 7;
  if (isEveryday) return 'everyday';
  if (isWeekdays) return 'weekdays';
  if (isWeekends) return 'weekends';
  return days.map((d) => ISO_TO_DAY_NAME[d]).join(',');
}

function validateSchedule(schedule) {
  if (!schedule || schedule.type !== 'days_of_week' || !Array.isArray(schedule.days)) {
    throw usageError('Invalid schedule');
  }
  const days = schedule.days;
  if (days.length < 1) throw usageError('Invalid schedule');
  for (const d of days) {
    if (!Number.isInteger(d) || d < 1 || d > 7) throw usageError('Invalid schedule');
  }
}

module.exports = {
  parseSchedulePattern,
  scheduleToString,
  validateSchedule
};
