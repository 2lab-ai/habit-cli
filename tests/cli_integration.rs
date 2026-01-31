use std::process::Command;

fn habit_bin() -> &'static str {
    env!("CARGO_BIN_EXE_habit")
}

fn run_habit(args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(habit_bin());
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("failed to run habit binary")
}

fn stdout_str(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr_str(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[test]
fn mvp_flow_is_deterministic_in_json_mode() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("db.json");
    let db = db_path.to_string_lossy().to_string();

    let today = "2026-01-31";

    let shared_env = [
        ("HABITCLI_DB_PATH", db.as_str()),
        ("HABITCLI_TODAY", today),
        ("NO_COLOR", "1"),
    ];

    let global = ["--db", db.as_str(), "--today", today, "--no-color"];

    // 0) list on empty
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["list", "--format", "json"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let habits = json.get("habits").and_then(|v| v.as_array()).unwrap();
        assert_eq!(habits.len(), 0);
    }

    // 1) add daily habit (Stretch)
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "add",
            "Stretch",
            "--schedule",
            "weekdays",
            "--period",
            "day",
            "--target",
            "1",
            "--notes",
            "2 minutes is fine",
            "--format",
            "json",
        ]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let id = json
            .get("habit")
            .and_then(|h| h.get("id"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(id.starts_with('h') && id.len() == 5);
    }

    // 2) add daily habit (Read)
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "add",
            "Read",
            "--schedule",
            "everyday",
            "--period",
            "day",
            "--target",
            "1",
            "--format",
            "json",
        ]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // 3) add weekly habit (Run)
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "add",
            "Run",
            "--schedule",
            "weekdays",
            "--period",
            "week",
            "--target",
            "3",
            "--format",
            "json",
        ]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // 4) list should be deterministic (sorted by name)
    let stretch_id: String;
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["list", "--format", "json"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let habits = json.get("habits").and_then(|v| v.as_array()).unwrap();
        assert_eq!(habits.len(), 3);

        let names: Vec<&str> = habits
            .iter()
            .map(|h| h.get("name").unwrap().as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["Read", "Run", "Stretch"]);

        stretch_id = habits
            .iter()
            .find(|h| h.get("name").unwrap().as_str().unwrap() == "Stretch")
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
    }

    // 5) show should accept id or unique name prefix
    {
        let mut args1: Vec<&str> = Vec::new();
        args1.extend_from_slice(&global);
        args1.extend_from_slice(&["show", &stretch_id, "--format", "json"]);

        let out1 = run_habit(&args1, &shared_env);
        assert_eq!(out1.status.code(), Some(0), "stderr: {}", stderr_str(&out1));
        let j1: serde_json::Value = serde_json::from_str(stdout_str(&out1).trim()).unwrap();
        assert_eq!(
            j1.get("habit")
                .unwrap()
                .get("id")
                .unwrap()
                .as_str()
                .unwrap(),
            stretch_id
        );

        let mut args2: Vec<&str> = Vec::new();
        args2.extend_from_slice(&global);
        args2.extend_from_slice(&["show", "str", "--format", "json"]);

        let out2 = run_habit(&args2, &shared_env);
        assert_eq!(out2.status.code(), Some(0), "stderr: {}", stderr_str(&out2));
        let j2: serde_json::Value = serde_json::from_str(stdout_str(&out2).trim()).unwrap();
        assert_eq!(
            j2.get("habit")
                .unwrap()
                .get("id")
                .unwrap()
                .as_str()
                .unwrap(),
            stretch_id
        );
    }

    // 6) checkin should be deterministic using explicit date
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["checkin", "Stretch", "--date", today, "--qty", "1"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
        let text = stdout_str(&out) + &stderr_str(&out);
        assert!(text.contains(today));
    }

    // 7) status should render and include today's date
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["status", "--date", today, "--format", "json"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let hay = json.to_string();
        assert!(hay.contains(today));
        assert!(hay.to_lowercase().contains("stretch") || hay.contains(&stretch_id));
    }

    // 8) archive/unarchive should affect dashboards
    {
        // Archive Read
        let mut args_a: Vec<&str> = Vec::new();
        args_a.extend_from_slice(&global);
        args_a.extend_from_slice(&["archive", "read", "--format", "json"]);
        let out_a = run_habit(&args_a, &shared_env);
        assert_eq!(
            out_a.status.code(),
            Some(0),
            "stderr: {}",
            stderr_str(&out_a)
        );

        // status (default) should not include read
        let mut args_s: Vec<&str> = Vec::new();
        args_s.extend_from_slice(&global);
        args_s.extend_from_slice(&["status", "--date", today, "--format", "json"]);
        let out_s = run_habit(&args_s, &shared_env);
        assert_eq!(
            out_s.status.code(),
            Some(0),
            "stderr: {}",
            stderr_str(&out_s)
        );
        let json_s: serde_json::Value = serde_json::from_str(stdout_str(&out_s).trim()).unwrap();
        assert!(!json_s.to_string().to_lowercase().contains("read"));

        // status --include-archived should include it
        let mut args_si: Vec<&str> = Vec::new();
        args_si.extend_from_slice(&global);
        args_si.extend_from_slice(&[
            "status",
            "--date",
            today,
            "--include-archived",
            "--format",
            "json",
        ]);
        let out_si = run_habit(&args_si, &shared_env);
        assert_eq!(
            out_si.status.code(),
            Some(0),
            "stderr: {}",
            stderr_str(&out_si)
        );
        let json_si: serde_json::Value = serde_json::from_str(stdout_str(&out_si).trim()).unwrap();
        assert!(json_si.to_string().to_lowercase().contains("read"));

        // Unarchive
        let mut args_u: Vec<&str> = Vec::new();
        args_u.extend_from_slice(&global);
        args_u.extend_from_slice(&["unarchive", "read", "--format", "json"]);
        let out_u = run_habit(&args_u, &shared_env);
        assert_eq!(
            out_u.status.code(),
            Some(0),
            "stderr: {}",
            stderr_str(&out_u)
        );

        // status should include read again
        let out_s2 = run_habit(&args_s, &shared_env);
        assert_eq!(
            out_s2.status.code(),
            Some(0),
            "stderr: {}",
            stderr_str(&out_s2)
        );
        let json_s2: serde_json::Value = serde_json::from_str(stdout_str(&out_s2).trim()).unwrap();
        assert!(json_s2.to_string().to_lowercase().contains("read"));
    }

    // 9) stats should provide required metrics
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "stats",
            "--from",
            "2026-01-01",
            "--to",
            today,
            "--format",
            "json",
        ]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let stats = json.get("stats").and_then(|v| v.as_array()).unwrap();
        assert!(stats.iter().all(|r| r.get("current_streak").is_some()));
        assert!(stats.iter().all(|r| r.get("longest_streak").is_some()));
        assert!(stats.iter().all(|r| r.get("success_rate").is_some()));
    }

    // 10) export JSON to stdout
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["export", "--format", "json"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        assert_eq!(json.get("version").unwrap().as_i64().unwrap(), 1);
        assert!(json.get("habits").unwrap().is_array());
        assert!(json.get("checkins").unwrap().is_array());
    }

    // 11) export CSV to a directory
    {
        let out_dir = tmp.path().join("export");
        let out_dir_s = out_dir.to_string_lossy().to_string();

        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["export", "--format", "csv", "--out", out_dir_s.as_str()]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        assert!(out_dir.join("habits.csv").exists());
        assert!(out_dir.join("checkins.csv").exists());
    }
}

#[test]
fn ambiguous_selector_exits_4() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("db.json");
    let db = db_path.to_string_lossy().to_string();

    let today = "2026-01-31";

    let shared_env = [
        ("HABITCLI_DB_PATH", db.as_str()),
        ("HABITCLI_TODAY", today),
        ("NO_COLOR", "1"),
    ];
    let global = ["--db", db.as_str(), "--today", today, "--no-color"];

    // add two habits with same prefix
    for name in ["Stretch", "Strength"] {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["add", name, "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // show str should be ambiguous
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["show", "str", "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(4));
        assert!(stderr_str(&out).to_lowercase().contains("ambiguous"));
    }
}

#[test]
fn new_habit_completion_requires_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("db.json");
    let db = db_path.to_string_lossy().to_string();

    let today = "2026-01-31";

    let shared_env = [
        ("HABITCLI_DB_PATH", db.as_str()),
        ("HABITCLI_TODAY", today),
        ("NO_COLOR", "1"),
    ];
    let global = ["--db", db.as_str(), "--today", today, "--no-color"];

    // add habit (defaults to needs_declaration=true)
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["add", "Gate", "--schedule", "everyday", "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // checkin without declaration
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["checkin", "Gate", "--date", today, "--qty", "1"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // status should show done=false (counted_quantity=0)
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["status", "--date", today, "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let habits = json
            .get("today")
            .unwrap()
            .get("habits")
            .unwrap()
            .as_array()
            .unwrap();
        let gate = habits
            .iter()
            .find(|h| h.get("name").unwrap().as_str().unwrap() == "Gate")
            .unwrap();
        assert_eq!(gate.get("done").unwrap().as_bool().unwrap(), false);
        assert_eq!(gate.get("quantity").unwrap().as_u64().unwrap(), 0);
        assert_eq!(gate.get("raw_quantity").unwrap().as_u64().unwrap(), 1);
        assert_eq!(
            gate.get("needs_declaration").unwrap().as_bool().unwrap(),
            true
        );
        assert_eq!(gate.get("declared").unwrap().as_bool().unwrap(), false);
    }

    // declare and check status again
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "declare",
            "Gate",
            "--date",
            today,
            "--ts",
            "2026-01-31T10:00:00Z",
            "--text",
            "I will do it",
            "--format",
            "json",
        ]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["status", "--date", today, "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let habits = json
            .get("today")
            .unwrap()
            .get("habits")
            .unwrap()
            .as_array()
            .unwrap();
        let gate = habits
            .iter()
            .find(|h| h.get("name").unwrap().as_str().unwrap() == "Gate")
            .unwrap();
        assert_eq!(gate.get("done").unwrap().as_bool().unwrap(), true);
        assert_eq!(gate.get("quantity").unwrap().as_u64().unwrap(), 1);
        assert_eq!(gate.get("declared").unwrap().as_bool().unwrap(), true);
    }
}

#[test]
fn exceptions_affect_penalty_tick() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("db.json");
    let db = db_path.to_string_lossy().to_string();

    let today = "2026-01-31";
    let tomorrow = "2026-02-01";
    let day_after = "2026-02-02";

    let shared_env = [
        ("HABITCLI_DB_PATH", db.as_str()),
        ("HABITCLI_TODAY", today),
        ("NO_COLOR", "1"),
    ];
    let global = ["--db", db.as_str(), "--today", today, "--no-color"];

    // add habit
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "add",
            "Pushups",
            "--schedule",
            "everyday",
            "--needs-declaration",
            "false",
            "--format",
            "json",
        ]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // arm penalty rule
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "penalty",
            "arm",
            "Pushups",
            "--multiplier",
            "2",
            "--cap",
            "8",
            "--deadline-days",
            "1",
            "--date",
            today,
            "--ts",
            "2026-01-31T09:00:00Z",
            "--format",
            "json",
        ]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // excuse today (allowed) => tick should NOT create debt due tomorrow
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "excuse",
            "Pushups",
            "--date",
            today,
            "--ts",
            "2026-01-31T09:30:00Z",
            "--reason",
            "sick",
            "--kind",
            "allowed",
            "--format",
            "json",
        ]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "penalty",
            "tick",
            "--date",
            today,
            "--ts",
            "2026-01-31T23:59:00Z",
            "--format",
            "json",
        ]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["penalty", "status", "--date", tomorrow, "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        assert_eq!(json.get("debts").unwrap().as_array().unwrap().len(), 0);
    }

    // no excuse tomorrow => tick should create debt due day_after
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "penalty",
            "tick",
            "--date",
            tomorrow,
            "--ts",
            "2026-02-01T23:59:00Z",
            "--format",
            "json",
        ]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["penalty", "status", "--date", day_after, "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let debts = json.get("debts").unwrap().as_array().unwrap();
        assert_eq!(debts.len(), 1);
    }
}

#[test]
fn penalty_tick_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("db.json");
    let db = db_path.to_string_lossy().to_string();

    let today = "2026-01-31";
    let tomorrow = "2026-02-01";

    let shared_env = [
        ("HABITCLI_DB_PATH", db.as_str()),
        ("HABITCLI_TODAY", today),
        ("NO_COLOR", "1"),
    ];
    let global = ["--db", db.as_str(), "--today", today, "--no-color"];

    // add habit + arm rule
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "add",
            "Journal",
            "--schedule",
            "everyday",
            "--needs-declaration",
            "false",
            "--format",
            "json",
        ]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&[
            "penalty",
            "arm",
            "Journal",
            "--multiplier",
            "2",
            "--cap",
            "8",
            "--deadline-days",
            "1",
            "--date",
            today,
            "--ts",
            "2026-01-31T09:00:00Z",
            "--format",
            "json",
        ]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // tick twice for same date
    {
        let mut args1: Vec<&str> = Vec::new();
        args1.extend_from_slice(&global);
        args1.extend_from_slice(&[
            "penalty",
            "tick",
            "--date",
            today,
            "--ts",
            "2026-01-31T23:00:00Z",
            "--format",
            "json",
        ]);
        let out1 = run_habit(&args1, &shared_env);
        assert_eq!(out1.status.code(), Some(0), "stderr: {}", stderr_str(&out1));
        let j1: serde_json::Value = serde_json::from_str(stdout_str(&out1).trim()).unwrap();
        assert_eq!(j1.get("created").unwrap().as_array().unwrap().len(), 1);

        let mut args2: Vec<&str> = Vec::new();
        args2.extend_from_slice(&global);
        args2.extend_from_slice(&[
            "penalty",
            "tick",
            "--date",
            today,
            "--ts",
            "2026-01-31T23:10:00Z",
            "--format",
            "json",
        ]);
        let out2 = run_habit(&args2, &shared_env);
        assert_eq!(out2.status.code(), Some(0), "stderr: {}", stderr_str(&out2));
        let j2: serde_json::Value = serde_json::from_str(stdout_str(&out2).trim()).unwrap();
        assert_eq!(j2.get("created").unwrap().as_array().unwrap().len(), 0);
    }

    // status should show exactly one outstanding debt due tomorrow
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["penalty", "status", "--date", tomorrow, "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        assert_eq!(json.get("debts").unwrap().as_array().unwrap().len(), 1);
    }
}

#[test]
fn declare_missing_required_flags_exits_2() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("db.json");
    let db = db_path.to_string_lossy().to_string();

    let today = "2026-01-31";

    let shared_env = [
        ("HABITCLI_DB_PATH", db.as_str()),
        ("HABITCLI_TODAY", today),
        ("NO_COLOR", "1"),
    ];
    let global = ["--db", db.as_str(), "--today", today, "--no-color"];

    // add habit
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["add", "Decl", "--format", "json"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0));
    }

    // missing --ts
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["declare", "Decl", "--date", today, "--text", "hi"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(2));
    }
}

#[test]
fn recap_json_shape_and_deterministic_values() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("db.json");
    let db = db_path.to_string_lossy().to_string();

    // Fixed date for determinism
    let today = "2026-01-31";

    let shared_env = [
        ("HABITCLI_DB_PATH", db.as_str()),
        ("HABITCLI_TODAY", today),
        ("NO_COLOR", "1"),
    ];
    let global = ["--db", db.as_str(), "--today", today, "--no-color"];

    // Add a daily habit (everyday, created on 2026-01-01 via --today override)
    {
        let args = [
            "--db", db.as_str(),
            "--today", "2026-01-01",
            "--no-color",
            "add",
            "Water",
            "--schedule", "everyday",
            "--period", "day",
            "--target", "8",
            "--needs-declaration", "false",
            "--format", "json",
        ];
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // Add a weekly habit (created on 2026-01-01)
    {
        let args = [
            "--db", db.as_str(),
            "--today", "2026-01-01",
            "--no-color",
            "add",
            "Exercise",
            "--schedule", "weekdays",
            "--period", "week",
            "--target", "3",
            "--needs-declaration", "false",
            "--format", "json",
        ];
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // Check-in Water for 10 specific days (2026-01-05 through 2026-01-14)
    // Each check-in qty=8 to meet target
    for day in 5..=14 {
        let date = format!("2026-01-{:02}", day);
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["checkin", "Water", "--date", &date, "--qty", "8"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // Check-in Exercise for weeks 2 and 3 (to get 2 successful weeks out of 5)
    // Week 2: 2026-01-06 to 2026-01-12, need 3 check-ins
    for date in ["2026-01-06", "2026-01-07", "2026-01-08"] {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["checkin", "Exercise", "--date", date, "--qty", "1"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }
    // Week 3: 2026-01-13 to 2026-01-19, need 3 check-ins
    for date in ["2026-01-13", "2026-01-14", "2026-01-15"] {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["checkin", "Exercise", "--date", date, "--qty", "1"]);
        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));
    }

    // Test recap with --range month (default) JSON output
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["recap", "--range", "month", "--format", "json"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let recap = json.get("recap").and_then(|v| v.as_array()).unwrap();
        assert_eq!(recap.len(), 2, "Expected 2 habits in recap");

        // Validate JSON shape for each recap row
        for row in recap.iter() {
            assert!(row.get("habit_id").is_some(), "Missing habit_id");
            assert!(row.get("name").is_some(), "Missing name");
            assert!(row.get("period").is_some(), "Missing period");
            assert!(row.get("target_label").is_some(), "Missing target_label");
            assert!(row.get("target").is_some(), "Missing target");
            assert!(row.get("successes").is_some(), "Missing successes");
            assert!(row.get("eligible").is_some(), "Missing eligible");
            assert!(row.get("rate").is_some(), "Missing rate");
            assert!(row.get("percent").is_some(), "Missing percent");
            assert!(row.get("range").is_some(), "Missing range");

            let range = row.get("range").unwrap();
            assert!(range.get("kind").is_some(), "Missing range.kind");
            assert!(range.get("from").is_some(), "Missing range.from");
            assert!(range.get("to").is_some(), "Missing range.to");

            // Verify range is "month"
            assert_eq!(range.get("kind").unwrap().as_str().unwrap(), "month");
            // Verify from/to for month range (30 days back from 2026-01-31)
            assert_eq!(range.get("from").unwrap().as_str().unwrap(), "2026-01-02");
            assert_eq!(range.get("to").unwrap().as_str().unwrap(), "2026-01-31");
        }

        // Find Water habit and validate deterministic values
        let water = recap
            .iter()
            .find(|r| r.get("name").unwrap().as_str().unwrap() == "Water")
            .expect("Water habit not found");

        assert_eq!(water.get("period").unwrap().as_str().unwrap(), "day");
        assert_eq!(water.get("target_label").unwrap().as_str().unwrap(), "8/day");
        assert_eq!(water.get("target").unwrap().as_u64().unwrap(), 8);
        // 10 successful days out of 30 eligible (Jan 2-31)
        assert_eq!(water.get("successes").unwrap().as_u64().unwrap(), 10);
        assert_eq!(water.get("eligible").unwrap().as_u64().unwrap(), 30);
        // 10/30 = 33%
        assert_eq!(water.get("percent").unwrap().as_u64().unwrap(), 33);

        // Find Exercise habit and validate deterministic values
        let exercise = recap
            .iter()
            .find(|r| r.get("name").unwrap().as_str().unwrap() == "Exercise")
            .expect("Exercise habit not found");

        assert_eq!(exercise.get("period").unwrap().as_str().unwrap(), "week");
        assert_eq!(
            exercise.get("target_label").unwrap().as_str().unwrap(),
            "3/week"
        );
        assert_eq!(exercise.get("target").unwrap().as_u64().unwrap(), 3);
        // 2 successful weeks (weeks 2 and 3) out of eligible weeks in month range
        // Month range 2026-01-02 to 2026-01-31 covers weeks:
        // W01 (partial: Jan 2-4), W02 (Jan 5-11), W03 (Jan 12-18), W04 (Jan 19-25), W05 (Jan 26-31 partial)
        // All 5 weeks have at least some days in range and habit existed, so 5 eligible
        // But only weeks 2 and 3 have 3+ check-ins
        assert_eq!(exercise.get("successes").unwrap().as_u64().unwrap(), 2);
    }

    // Test recap with --range ytd
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["recap", "--range", "ytd", "--format", "json"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let recap = json.get("recap").and_then(|v| v.as_array()).unwrap();
        assert_eq!(recap.len(), 2);

        // Verify range is ytd (Jan 1 to Jan 31)
        let first = &recap[0];
        let range = first.get("range").unwrap();
        assert_eq!(range.get("kind").unwrap().as_str().unwrap(), "ytd");
        assert_eq!(range.get("from").unwrap().as_str().unwrap(), "2026-01-01");
        assert_eq!(range.get("to").unwrap().as_str().unwrap(), "2026-01-31");
    }

    // Test recap with --range week
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["recap", "--range", "week", "--format", "json"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let json: serde_json::Value = serde_json::from_str(stdout_str(&out).trim()).unwrap();
        let recap = json.get("recap").and_then(|v| v.as_array()).unwrap();
        assert_eq!(recap.len(), 2);

        // Verify range is week (Jan 25 to Jan 31)
        let first = &recap[0];
        let range = first.get("range").unwrap();
        assert_eq!(range.get("kind").unwrap().as_str().unwrap(), "week");
        assert_eq!(range.get("from").unwrap().as_str().unwrap(), "2026-01-25");
        assert_eq!(range.get("to").unwrap().as_str().unwrap(), "2026-01-31");
    }

    // Test table output runs without error
    {
        let mut args: Vec<&str> = Vec::new();
        args.extend_from_slice(&global);
        args.extend_from_slice(&["recap", "--range", "month", "--format", "table"]);

        let out = run_habit(&args, &shared_env);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_str(&out));

        let output = stdout_str(&out);
        // Should contain column headers
        assert!(output.contains("name"));
        assert!(output.contains("target"));
        assert!(output.contains("progress"));
        // Should contain habit names
        assert!(output.contains("Water"));
        assert!(output.contains("Exercise"));
        // Should contain percentages
        assert!(output.contains("%"));
    }
}
