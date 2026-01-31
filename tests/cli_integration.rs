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
            j1.get("habit").unwrap().get("id").unwrap().as_str().unwrap(),
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
