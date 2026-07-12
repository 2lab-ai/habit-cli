use crate::schedule::Schedule;

fn default_next_counter() -> u32 {
    1
}

fn default_excuse_quota_per_week() -> u32 {
    2
}

fn default_cadence_minutes() -> u32 {
    180
}

fn default_quiet_start() -> String {
    "23:00".to_string()
}

fn default_quiet_end() -> String {
    "08:00".to_string()
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

    /// Routine templates.
    #[serde(default)]
    pub routines: Vec<Routine>,

    /// Routine session instances.
    #[serde(default)]
    pub routine_sessions: Vec<RoutineSession>,

    /// Nag configuration + state (automation-facing; messaging is handled by OpenClaw).
    #[serde(default)]
    pub nag: Nag,
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

    #[serde(default = "default_next_counter")]
    pub next_routine_number: u32,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Routine {
    pub id: String,
    pub name: String,
    /// Optional display/scheduling hint for external orchestrators.
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default)]
    pub steps: Vec<RoutineStep>,
    pub archived: bool,
    pub created_date: String,
    pub archived_date: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutineStep {
    pub index: u32,
    pub name: String,
    pub minutes: u32,
    #[serde(default)]
    pub quote: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutineSession {
    pub id: String,
    pub routine_id: String,
    pub routine_name: String,
    pub date: String,
    pub started_ts: String,
    pub state: RoutineSessionState,
    #[serde(default)]
    pub steps: Vec<RoutineSessionStep>,
    /// Append-only session events for audit + retry-safety.
    #[serde(default)]
    pub actions: Vec<RoutineAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutineSessionState {
    Active,
    Done,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutineSessionStep {
    pub index: u32,
    pub name: String,
    pub minutes: u32,
    #[serde(default)]
    pub quote: Option<String>,
    pub status: RoutineStepStatus,
    #[serde(default)]
    pub action_ts: Option<String>,
    #[serde(default)]
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutineStepStatus {
    Pending,
    Done,
    Skipped,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutineAction {
    pub id: String,
    pub kind: RoutineActionKind,
    pub ts: String,
    #[serde(default)]
    pub step_index: Option<u32>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutineActionKind {
    Next,
    Skip,
    Done,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Nag {
    #[serde(default)]
    pub config: NagConfig,
    #[serde(default)]
    pub state: NagState,
}

impl Default for Nag {
    fn default() -> Self {
        Self {
            config: NagConfig::default(),
            state: NagState::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NagConfig {
    #[serde(default = "default_quiet_start")]
    pub quiet_start: String,
    #[serde(default = "default_quiet_end")]
    pub quiet_end: String,
    #[serde(default = "default_cadence_minutes")]
    pub cadence_minutes: u32,
}

impl Default for NagConfig {
    fn default() -> Self {
        Self {
            quiet_start: default_quiet_start(),
            quiet_end: default_quiet_end(),
            cadence_minutes: default_cadence_minutes(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NagState {
    #[serde(default)]
    pub snoozed_until: Option<String>,
    #[serde(default)]
    pub snooze_reason: Option<String>,
    #[serde(default)]
    pub last_sent_ts: Option<String>,
}

impl Default for NagState {
    fn default() -> Self {
        Self {
            snoozed_until: None,
            snooze_reason: None,
            last_sent_ts: None,
        }
    }
}

pub fn default_db() -> Db {
    Db {
        version: 1,
        meta: Meta {
            next_habit_number: 1,
            next_declaration_number: 1,
            next_excuse_number: 1,
            next_penalty_rule_number: 1,
            next_routine_number: 1,
        },
        habits: Vec::new(),
        checkins: Vec::new(),
        declarations: Vec::new(),
        excuses: Vec::new(),
        penalty_rules: Vec::new(),
        penalty_debts: Vec::new(),
        penalty_actions: Vec::new(),
        routines: Vec::new(),
        routine_sessions: Vec::new(),
        nag: Nag::default(),
    }
}
