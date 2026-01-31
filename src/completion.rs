use crate::checkins::get_quantity;
use crate::declarations::has_declaration;
use crate::model::{Db, Habit};

pub fn is_declared(db: &Db, habit: &Habit, date: &str) -> bool {
    if !habit.needs_declaration {
        return true;
    }
    has_declaration(db, &habit.id, date)
}

/// Quantity that counts toward completion semantics.
pub fn counted_quantity(db: &Db, habit: &Habit, date: &str) -> u32 {
    let raw = get_quantity(db, &habit.id, date);
    if habit.needs_declaration && !has_declaration(db, &habit.id, date) {
        0
    } else {
        raw
    }
}

pub fn is_done_for_date(db: &Db, habit: &Habit, date: &str) -> bool {
    counted_quantity(db, habit, date) >= habit.target.quantity
}
