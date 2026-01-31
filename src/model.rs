use crate::schedule::Schedule;

fn default_next_counter() -> u32 {
    1
}

fn default_excuse_quota_per_week() -> u32 {
    2
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Db {
    pub version: u32,
    pub meta: Meta,
    pub habits: Vec<Habit>,
    pub checkins: Vec<Checkin>,

    /// Append-only declarations.
    #[serde(default)]
    pub declarations: Vec<Declaration>,

    /// Append-only excuse records.
    #[serde(default)]
    pub excuses: Vec<Excuse>,

    /// Penalty rules (state).
    #[serde(default)]
    pub penalty_rules: Vec<PenaltyRule>,

    /// Penalty debts (append-only).
    #[serde(default)]
    pub penalty_debts: Vec<PenaltyDebt>,

    /// Penalty actions (append-only): resolve/void.
    #[serde(default)]
    pub penalty_actions: Vec<PenaltyAction>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Meta {
    pub next_habit_number: u32,

    #[serde(default = "default_next_counter")]
    pub next_declaration_number: u32,

    #[serde(default = "default_next_counter")]
    pub next_excuse_number: u32,

    #[serde(default = "default_next_counter")]
    pub next_penalty_rule_number: u32,
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

    /// If true, completion for a given date is only recognized if a declaration exists for that date.
    #[serde(default)]
    pub needs_declaration: bool,

    /// Maximum number of allowed excused days per ISO week.
    #[serde(default = "default_excuse_quota_per_week")]
    pub excuse_quota_per_week: u32,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Declaration {
    pub id: String,
    pub habit_id: String,
    pub date: String,
    pub ts: String,
    pub text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Excuse {
    pub id: String,
    pub habit_id: String,
    pub date: String,
    pub ts: String,
    pub kind: ExcuseKind,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExcuseKind {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PenaltyRule {
    pub id: String,
    pub habit_id: String,
    /// Multiplier used for escalation (default 2).
    pub multiplier: u32,
    /// Maximum debt quantity.
    pub cap: u32,
    /// Deadline window (days) for the debt; informational for MVP.
    pub deadline_days: u32,

    pub armed_date: String,
    pub armed_ts: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PenaltyDebt {
    pub id: String,
    pub habit_id: String,
    /// The date whose evaluation produced this debt.
    pub trigger_date: String,
    /// The next day on which the debt is due.
    pub due_date: String,
    pub quantity: u32,
    pub rule_id: String,
    pub created_date: String,
    pub created_ts: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PenaltyAction {
    pub id: String,
    pub debt_id: String,
    pub kind: PenaltyActionKind,
    pub date: String,
    pub ts: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PenaltyActionKind {
    Resolve,
    Void,
}

pub fn default_db() -> Db {
    Db {
        version: 1,
        meta: Meta {
            next_habit_number: 1,
            next_declaration_number: 1,
            next_excuse_number: 1,
            next_penalty_rule_number: 1,
        },
        habits: Vec::new(),
        checkins: Vec::new(),
        declarations: Vec::new(),
        excuses: Vec::new(),
        penalty_rules: Vec::new(),
        penalty_debts: Vec::new(),
        penalty_actions: Vec::new(),
    }
}
