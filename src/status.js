const { isoWeekId, isoWeekStart, isoWeekEnd, dateRangeInclusive } = require('./date');
const { stableHabitSort, isScheduledOn } = require('./habits');
const { getQuantity } = require('./checkins');

function weekSumForHabit(db, habit, weekStartDate) {
  const start = weekStartDate;
  const end = isoWeekEnd(weekStartDate);
  const days = dateRangeInclusive(start, end);
  let sum = 0;
  for (const d of days) {
    if (d < habit.created_date) continue;
    sum += getQuantity(db, habit.id, d);
  }
  return sum;
}

function buildStatus(db, { date, weekOf, includeArchived }) {
  const today = date;
  const weekStart = isoWeekStart(weekOf || today);
  const weekEnd = isoWeekEnd(weekStart);
  const weekId = isoWeekId(weekStart);

  const habits = db.habits
    .filter((h) => includeArchived ? true : !h.archived)
    .slice()
    .sort(stableHabitSort);

  const todayRows = [];
  for (const h of habits) {
    if (!isScheduledOn(h, today)) continue;
    if (h.target.period === 'day') {
      const qty = getQuantity(db, h.id, today);
      const done = qty >= h.target.quantity;
      todayRows.push({
        id: h.id,
        name: h.name,
        period: 'day',
        target: h.target.quantity,
        quantity: qty,
        done
      });
    } else {
      const sum = weekSumForHabit(db, h, weekStart);
      const done = sum >= h.target.quantity;
      todayRows.push({
        id: h.id,
        name: h.name,
        period: 'week',
        target: h.target.quantity,
        quantity: sum,
        done
      });
    }
  }

  const weekRows = [];
  const weekDays = dateRangeInclusive(weekStart, weekEnd);

  for (const h of habits) {
    if (h.target.period === 'day') {
      let scheduled = 0;
      let doneDays = 0;
      for (const d of weekDays) {
        if (!isScheduledOn(h, d)) continue;
        scheduled += 1;
        const qty = getQuantity(db, h.id, d);
        if (qty >= h.target.quantity) doneDays += 1;
      }
      weekRows.push({
        id: h.id,
        name: h.name,
        period: 'day',
        scheduled_days: scheduled,
        done_scheduled_days: doneDays
      });
    } else {
      const sum = weekSumForHabit(db, h, weekStart);
      weekRows.push({
        id: h.id,
        name: h.name,
        period: 'week',
        target: h.target.quantity,
        quantity: sum
      });
    }
  }

  return {
    today: {
      date: today,
      habits: todayRows
    },
    week: {
      id: weekId,
      start_date: weekStart,
      end_date: weekEnd,
      habits: weekRows
    }
  };
}

module.exports = { buildStatus };
