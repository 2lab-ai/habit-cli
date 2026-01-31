const { usageError } = require('./errors');
const { parseDateString } = require('./date');

function findCheckinIndex(db, habitId, date) {
  return db.checkins.findIndex((c) => c.habit_id === habitId && c.date === date);
}

function getQuantity(db, habitId, date) {
  const idx = findCheckinIndex(db, habitId, date);
  if (idx === -1) return 0;
  return db.checkins[idx].quantity;
}

function setQuantity(db, habitId, date, quantity) {
  parseDateString(date);
  if (!Number.isInteger(quantity) || quantity < 0) throw usageError('Invalid quantity');

  const idx = findCheckinIndex(db, habitId, date);
  if (quantity === 0) {
    if (idx !== -1) db.checkins.splice(idx, 1);
    return;
  }
  if (idx === -1) {
    db.checkins.push({ habit_id: habitId, date, quantity });
  } else {
    db.checkins[idx].quantity = quantity;
  }
}

function addQuantity(db, habitId, date, delta) {
  parseDateString(date);
  if (!Number.isInteger(delta) || delta < 1) throw usageError('Invalid quantity');
  const cur = getQuantity(db, habitId, date);
  setQuantity(db, habitId, date, cur + delta);
  return cur + delta;
}

function listCheckinsForHabit(db, habitId) {
  return db.checkins
    .filter((c) => c.habit_id === habitId)
    .slice()
    .sort((a, b) => (a.date < b.date ? -1 : a.date > b.date ? 1 : (a.habit_id < b.habit_id ? -1 : a.habit_id > b.habit_id ? 1 : 0)));
}

function listCheckinsInRange(db, { from, to, habitIds }) {
  return db.checkins
    .filter((c) => {
      if (habitIds && !habitIds.has(c.habit_id)) return false;
      if (from && c.date < from) return false;
      if (to && c.date > to) return false;
      return true;
    })
    .slice()
    .sort((a, b) => (a.date < b.date ? -1 : a.date > b.date ? 1 : (a.habit_id < b.habit_id ? -1 : a.habit_id > b.habit_id ? 1 : 0)));
}

module.exports = {
  getQuantity,
  setQuantity,
  addQuantity,
  listCheckinsForHabit,
  listCheckinsInRange
};
