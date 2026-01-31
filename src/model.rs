use crate::schedule::Schedule;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Db {
    pub version: u32,
    pub meta: Meta,
    pub habits: Vec<Habit>,
    pub checkins: Vec<Checkin>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Meta {
    pub next_habit_number: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Habit {
    pub id: String,
    pub name: String,
    pub schedule: Schedule,
    pub target: Target,
    pub notes: Option<String>,
    pub archived: bool,
    pub created_date: String,
    pub archived_date: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Target {
    pub period: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Checkin {
    pub habit_id: String,
    pub date: String,
    pub quantity: u32,
}

pub fn default_db() -> Db {
    Db {
        version: 1,
        meta: Meta {
            next_habit_number: 1,
        },
        habits: Vec::new(),
        checkins: Vec::new(),
    }
}
