const { usageError } = require('./errors');

/** @returns {boolean} */
function isValidDateString(s) {
  return /^\d{4}-\d{2}-\d{2}$/.test(s);
}

function parseDateString(s, label = 'date') {
  if (!isValidDateString(s)) throw usageError(`Invalid ${label}: ${s}`);
  const [yy, mm, dd] = s.split('-').map((x) => Number(x));
  const d = new Date(Date.UTC(yy, mm - 1, dd));
  if (
    d.getUTCFullYear() !== yy ||
    d.getUTCMonth() !== mm - 1 ||
    d.getUTCDate() !== dd
  ) {
    throw usageError(`Invalid ${label}: ${s}`);
  }
  return { y: yy, m: mm, d: dd };
}

function toDateUtc(dateStr) {
  const { y, m, d } = parseDateString(dateStr);
  return new Date(Date.UTC(y, m - 1, d));
}

function formatDateUtc(date) {
  const y = date.getUTCFullYear();
  const m = String(date.getUTCMonth() + 1).padStart(2, '0');
  const d = String(date.getUTCDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

function addDays(dateStr, deltaDays) {
  const dt = toDateUtc(dateStr);
  dt.setUTCDate(dt.getUTCDate() + deltaDays);
  return formatDateUtc(dt);
}

/**
 * ISO weekday number: Mon=1..Sun=7
 */
function isoWeekday(dateStr) {
  const dt = toDateUtc(dateStr);
  const dow = dt.getUTCDay(); // 0..6, Sun=0
  return dow === 0 ? 7 : dow;
}

function compareDates(a, b) {
  // YYYY-MM-DD lexicographic compare works.
  if (a < b) return -1;
  if (a > b) return 1;
  return 0;
}

function dateRangeInclusive(from, to) {
  parseDateString(from, 'from');
  parseDateString(to, 'to');
  if (from > to) throw usageError(`Invalid range: from > to`);
  const out = [];
  let cur = from;
  while (cur <= to) {
    out.push(cur);
    cur = addDays(cur, 1);
  }
  return out;
}

function isoWeekStart(dateStr) {
  const wd = isoWeekday(dateStr);
  return addDays(dateStr, -(wd - 1));
}

function isoWeekEnd(dateStr) {
  return addDays(isoWeekStart(dateStr), 6);
}

function isoWeekId(dateStr) {
  // Algorithm: week-year is year of Thursday in that ISO week.
  const dt = toDateUtc(dateStr);
  const dow = dt.getUTCDay() || 7; // 1..7
  dt.setUTCDate(dt.getUTCDate() + (4 - dow)); // shift to Thursday
  const weekYear = dt.getUTCFullYear();
  const yearStart = new Date(Date.UTC(weekYear, 0, 1));
  const yearStartDow = yearStart.getUTCDay() || 7;
  // ISO week 1 is the week containing Jan 4.
  const week1Start = new Date(Date.UTC(weekYear, 0, 1 + (4 - yearStartDow)));
  // week1Start is Thursday of week1; we want Monday of that week
  week1Start.setUTCDate(week1Start.getUTCDate() - 3);
  const diffDays = Math.floor((dt - week1Start) / (24 * 3600 * 1000));
  const week = 1 + Math.floor(diffDays / 7);
  return `${weekYear}-W${String(week).padStart(2, '0')}`;
}

function systemToday() {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth() + 1;
  const d = now.getDate();
  const mm = String(m).padStart(2, '0');
  const dd = String(d).padStart(2, '0');
  return `${y}-${mm}-${dd}`;
}

module.exports = {
  isValidDateString,
  parseDateString,
  addDays,
  isoWeekday,
  compareDates,
  dateRangeInclusive,
  isoWeekStart,
  isoWeekEnd,
  isoWeekId,
  systemToday
};
