const fs = require('fs/promises');
const path = require('path');

const { scheduleToString } = require('./schedule');

function csvEscape(value) {
  const s = value == null ? '' : String(value);
  if (/[\n\r",]/.test(s)) return '"' + s.replace(/"/g, '""') + '"';
  return s;
}

function toCsvLine(values) {
  return values.map(csvEscape).join(',');
}

async function exportCsvToDir({ outDir, habits, checkins }) {
  await fs.mkdir(outDir, { recursive: true, mode: 0o700 });

  const habitsHeader = ['id', 'name', 'schedule', 'period', 'target', 'notes', 'archived', 'created_date', 'archived_date'];
  const habitLines = [toCsvLine(habitsHeader)];
  for (const h of habits) {
    habitLines.push(toCsvLine([
      h.id,
      h.name,
      scheduleToString(h.schedule),
      h.target.period,
      String(h.target.quantity),
      h.notes == null ? '' : h.notes,
      h.archived ? 'true' : 'false',
      h.created_date,
      h.archived_date == null ? '' : h.archived_date
    ]));
  }

  const checkinsHeader = ['habit_id', 'date', 'quantity'];
  const checkinLines = [toCsvLine(checkinsHeader)];
  for (const c of checkins) {
    checkinLines.push(toCsvLine([c.habit_id, c.date, String(c.quantity)]));
  }

  await fs.writeFile(path.join(outDir, 'habits.csv'), habitLines.join('\n') + '\n', { mode: 0o600 });
  await fs.writeFile(path.join(outDir, 'checkins.csv'), checkinLines.join('\n') + '\n', { mode: 0o600 });
}

module.exports = {
  exportCsvToDir
};
