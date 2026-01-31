const { dateRangeInclusive, isoWeekStart, isoWeekEnd, addDays } = require('./date');
const { stableHabitSort, isScheduledOn } = require('./habits');
const { getQuantity } = require('./checkins');

function computeDailyStats(db, habit, { from, to }) {
  const days = dateRangeInclusive(from, to);
  const scheduledDays = days.filter((d) => isScheduledOn(habit, d));
  const successes = scheduledDays.filter((d) => getQuantity(db, habit.id, d) >= habit.target.quantity).length;
  const eligible = scheduledDays.length;
  const rate = eligible === 0 ? null : successes / eligible;

  // streaks are sequences of successful scheduled days
  let current = 0;
  if (eligible > 0) {
    // find last scheduled day in window
    const lastIdx = scheduledDays.length - 1;
    for (let i = lastIdx; i >= 0; i--) {
      const d = scheduledDays[i];
      const ok = getQuantity(db, habit.id, d) >= habit.target.quantity;
      if (!ok) break;
      current += 1;
    }
  }

  let longest = 0;
  let run = 0;
  for (const d of scheduledDays) {
    const ok = getQuantity(db, habit.id, d) >= habit.target.quantity;
    if (ok) {
      run += 1;
      if (run > longest) longest = run;
    } else {
      run = 0;
    }
  }

  return {
    habit_id: habit.id,
    name: habit.name,
    period: 'day',
    target: habit.target.quantity,
    window: { from, to },
    current_streak: current,
    longest_streak: longest,
    success_rate: {
      successes,
      eligible,
      rate
    }
  };
}

function weekSumForHabit(db, habit, weekStartDate) {
  const end = isoWeekEnd(weekStartDate);
  const days = dateRangeInclusive(weekStartDate, end);
  let sum = 0;
  for (const d of days) {
    if (d < habit.created_date) continue;
    sum += getQuantity(db, habit.id, d);
  }
  return sum;
}

function weekRangeInclusive(fromWeekStart, toWeekStart) {
  const weeks = [];
  let cur = fromWeekStart;
  while (cur <= toWeekStart) {
    weeks.push(cur);
    cur = addDays(cur, 7);
  }
  return weeks;
}

function computeWeeklyStats(db, habit, { from, to }) {
  const startWeek = isoWeekStart(from);
  const endWeek = isoWeekStart(to);
  const weekStartsAll = weekRangeInclusive(startWeek, endWeek);
  const eligibleWeekStarts = weekStartsAll.filter((ws) => isoWeekEnd(ws) >= habit.created_date);

  const successes = eligibleWeekStarts.filter((ws) => weekSumForHabit(db, habit, ws) >= habit.target.quantity).length;
  const eligible = eligibleWeekStarts.length;
  const rate = eligible === 0 ? null : successes / eligible;

  let current = 0;
  for (let i = eligibleWeekStarts.length - 1; i >= 0; i--) {
    const ws = eligibleWeekStarts[i];
    const ok = weekSumForHabit(db, habit, ws) >= habit.target.quantity;
    if (!ok) break;
    current += 1;
  }

  let longest = 0;
  let run = 0;
  for (const ws of eligibleWeekStarts) {
    const ok = weekSumForHabit(db, habit, ws) >= habit.target.quantity;
    if (ok) {
      run += 1;
      if (run > longest) longest = run;
    } else {
      run = 0;
    }
  }

  return {
    habit_id: habit.id,
    name: habit.name,
    period: 'week',
    target: habit.target.quantity,
    window: { from, to },
    current_streak: current,
    longest_streak: longest,
    success_rate: {
      successes,
      eligible,
      rate
    }
  };
}

function buildStats(db, habits, { from, to, windowForHabit } = {}) {
  const sortedHabits = habits.slice().sort(stableHabitSort);
  const rows = [];
  for (const h of sortedHabits) {
    const w = windowForHabit ? windowForHabit(h) : { from, to };
    if (!w || !w.from || !w.to) throw new Error('Stats window is required');

    if (h.target.period === 'day') rows.push(computeDailyStats(db, h, { from: w.from, to: w.to }));
    else rows.push(computeWeeklyStats(db, h, { from: w.from, to: w.to }));
  }
  return rows;
}

module.exports = {
  buildStats
};
