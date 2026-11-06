use codercraft_core::{dispatch, Command, SupportedLanguage, CoderCraftCore};
use databasecraft_store::DatabaseStore;
use std::path::Path;

#[tokio::test]
async fn test_codercraft_linter_and_language() {
    let linter = CoderCraftCore::get_linter_for(&SupportedLanguage::Rust);
    assert_eq!(linter.command, "cargo");
    assert!(linter.default_args.contains(&"clippy".to_string()));

    let py_linter = CoderCraftCore::get_linter_for(&SupportedLanguage::Python);
    assert_eq!(py_linter.command, "ruff");
}

#[tokio::test]
async fn test_codercraft_workspace_tree_scan() {
    let tree = CoderCraftCore::scan_tree(Path::new("D:\\Dev\\repos\\codercraft"), 0, 2);
    assert!(tree.is_dir);
    assert!(!tree.children.is_empty());
}

#[tokio::test]
async fn test_codercraft_mcp_and_console_dispatch() {
    let store = DatabaseStore::open_in_memory().expect("db store open");
    let res = dispatch(Command::ListMcpServers, &store).await;
    assert_eq!(res["success"], true);
    assert!(res["servers"].as_array().expect("servers array").len() >= 2);

    let console_res = dispatch(Command::ExecuteConsoleCommand {
        command_line: "Write-Output 'codercraft-ok'".to_string(),
        working_dir: None,
    }, &store).await;
    assert_eq!(console_res["success"], true);
    assert_eq!(console_res["exit_code"], 0);
}

#[tokio::test]
async fn test_codercraft_plan_and_instructions() {
    let store = DatabaseStore::open_in_memory().expect("db store open");
    let plan_res = dispatch(Command::AgentPlan {
        goal: "Implement zero-panic containment".to_string(),
        workspace_dir: "D:\\Dev\\repos\\codercraft".to_string(),
    }, &store).await;
    assert_eq!(plan_res["success"], true);
    assert!(plan_res["plan"].as_str().expect("plan str").contains("Architectural Implementation Plan"));

    let inst_res = dispatch(Command::DiscoverInstructions {
        workspace_dir: "D:\\Dev\\repos\\codercraft".to_string(),
    }, &store).await;
    assert_eq!(inst_res["success"], true);
}

#[tokio::test]
async fn test_codercraft_read_range_and_grep() {
    let store = DatabaseStore::open_in_memory().expect("db store open");
    let read_res = dispatch(Command::ReadFileRange {
        path: "D:\\Dev\\repos\\codercraft\\Cargo.toml".to_string(),
        start_line: Some(1),
        end_line: Some(5),
    }, &store).await;
    assert_eq!(read_res["success"], true);
    assert!(read_res["content"].as_str().expect("content str").contains("[workspace]"));

    let grep_res = dispatch(Command::GrepSearch {
        root: "D:\\Dev\\repos\\codercraft\\crates\\codercraft-core".to_string(),
        query: "CoderCraftCore".to_string(),
    }, &store).await;
    assert_eq!(grep_res["success"], true);
    assert!(grep_res["match_count"].as_u64().expect("match_count") > 0);
}

#[tokio::test]
async fn test_codercraft_fleet_depot_and_swarm() {
    let store = DatabaseStore::open_in_memory().expect("db store open");
    let depot_res = dispatch(Command::ListFleetProjects, &store).await;
    assert_eq!(depot_res["success"], true);
    assert!(depot_res["total_projects"].as_u64().expect("total_projects") >= 10);

    let probe_res = dispatch(Command::ProbeFleetProjects, &store).await;
    assert_eq!(probe_res["success"], true);

    let swarm_res = dispatch(Command::GetSwarmState, &store).await;
    assert_eq!(swarm_res["success"], true);
    assert_eq!(swarm_res["workers"].as_array().expect("workers").len(), 5);

    let step_res = dispatch(Command::StepSwarm, &store).await;
    assert_eq!(step_res["success"], true);

    let sym_res = dispatch(Command::ExtractSymbols {
        path: "D:\\Dev\\repos\\codercraft\\crates\\codercraft-core\\src\\lib.rs".to_string(),
    }, &store).await;
    assert_eq!(sym_res["success"], true);
    assert!(sym_res["symbols"].as_array().expect("symbols").len() >= 3);

    let help_res = dispatch(Command::GetHelp, &store).await;
    assert_eq!(help_res["success"], true);
    assert!(help_res["capabilities"].as_array().expect("caps").len() >= 5);

    let compare_res = dispatch(Command::GetComparison, &store).await;
    assert_eq!(compare_res["success"], true);
    assert!(compare_res["matrix"]["codercraft"]["runtime"].as_str().expect("runtime").contains("Native Rust"));
}

#[tokio::test]
async fn test_p0_eternal_memory_roundtrip() {
    let store = DatabaseStore::open_in_memory().expect("db store open");
    let now_year: i64 = chrono::Utc::now().format("%Y").to_string().parse().expect("year");
    let now_month: u32 = chrono::Utc::now().format("%m").to_string().parse().expect("month");

    let create = dispatch(
        Command::SessionCreate {
            session_id: Some("sess-p0-test".to_string()),
            workspace_dir: "D:/Dev/repos/codercraft test/test-crud-server-exists".to_string(),
            mode: "build".to_string(),
            title: "p0 eternal memory probe".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(create["success"], true);

    for (role, content) in [
        ("user", "how do we add a health check endpoint?"),
        ("assistant", "plan: add GET /healthz returning status ok"),
    ] {
        let ap = dispatch(
            Command::SessionAppend {
                session_id: "sess-p0-test".to_string(),
                role: role.to_string(),
                content: content.to_string(),
                tool_calls_json: None,
            },
            &store,
        )
        .await;
        assert_eq!(ap["success"], true);
    }

    let search = dispatch(
        Command::SessionSearch {
            query: "health check".to_string(),
            top_k: Some(5),
        },
        &store,
    )
    .await;
    assert_eq!(search["success"], true);
    assert!(search["count"].as_u64().expect("count") >= 1);

    let recall = dispatch(
        Command::SessionRecall {
            year: now_year,
            month: now_month,
            limit: Some(50),
        },
        &store,
    )
    .await;
    assert_eq!(recall["success"], true);
    assert!(recall["count"].as_u64().expect("recall count") >= 2);

    let read = dispatch(
        Command::SessionRead {
            session_id: "sess-p0-test".to_string(),
            limit: Some(10),
        },
        &store,
    )
    .await;
    assert_eq!(read["success"], true);
    assert_eq!(read["count"].as_u64().expect("read count"), 2);
}

#[tokio::test]
async fn test_p0_permission_gate() {
    let store = DatabaseStore::open_in_memory().expect("db store open");
    let deny = dispatch(
        Command::CheckPermission {
            tool: "shell".to_string(),
            target: "rm -rf / --no-preserve-root".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(deny["decision"], "deny");

    let ask = dispatch(
        Command::CheckPermission {
            tool: "write".to_string(),
            target: "D:/proj/.env".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(ask["decision"], "ask");

    let blocked = dispatch(
        Command::ExecuteConsoleCommand {
            command_line: "rm -rf /".to_string(),
            working_dir: None,
        },
        &store,
    )
    .await;
    assert_eq!(blocked["success"], false);
    assert_eq!(blocked["permission"], "deny");
}

#[tokio::test]
async fn test_p0_patch_and_glob() {
    let store = DatabaseStore::open_in_memory().expect("db store open");
    let root = std::env::temp_dir().join(format!(
        "codercraft-p0-{}",
        chrono::Utc::now().timestamp_millis()
    ));
    std::fs::create_dir_all(&root).expect("tmp root");
    let ws = root.to_string_lossy().to_string();
    std::fs::write(root.join(".gitignore"), "*.log\n").expect("gitignore");
    std::fs::write(root.join("skip.log"), "noise").expect("log");

    let patch = "*** Add File: src/new.rs\nfn health() -> bool { true }\n*** Update File: src/new.rs\nfn health() -> bool { true }\n*** Move to: src/renamed.rs\n";
    // First apply add+update separately (move needs an existing file).
    let add = dispatch(
        Command::ApplyPatch {
            workspace_dir: ws.clone(),
            patch_text: "*** Add File: src/new.rs\nfn health() -> bool { true }\n".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(add["success"], true);

    let mv = dispatch(
        Command::ApplyPatch {
            workspace_dir: ws.clone(),
            patch_text: "*** Update File: src/new.rs\nfn health() -> bool { true }\n*** Move to: src/renamed.rs\n".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(mv["success"], true);
    assert!(root.join("src/renamed.rs").exists());
    assert!(!root.join("src/new.rs").exists());
    let _ = patch;

    let glob = dispatch(
        Command::GlobSearch {
            root: ws.clone(),
            pattern: "**/*.rs".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(glob["success"], true);
    assert_eq!(glob["match_count"].as_u64().expect("glob count"), 1);

    let glob_log = dispatch(
        Command::GlobSearch {
            root: ws,
            pattern: "**/*.log".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(glob_log["success"], true);
    assert_eq!(glob_log["match_count"].as_u64().expect("log count"), 0);
}

#[tokio::test]
async fn test_p1_agent_run_explores_and_logs() {
    use codercraft_core::autonomy::run_goal;
    let store = DatabaseStore::open_in_memory().expect("db store open");
    let root = std::env::temp_dir().join(format!(
        "codercraft-p1-{}",
        chrono::Utc::now().timestamp_millis()
    ));
    std::fs::create_dir_all(&root).expect("tmp root");
    std::fs::write(root.join("main.py"), "def health():\n    return 'ok'\n").expect("seed");
    std::fs::write(root.join("README.md"), "# p1 fixture\n").expect("seed");
    let ws = root.to_string_lossy().to_string();

    // Dead-endpoint brain: connection refused in ms, deterministic fallback
    // path, no lingering child processes to stall the test runtime.
    let dead = craft_ai_core::CraftAiClient::with_config(craft_ai_core::CraftAiConfig {
        provider: "local".to_string(),
        model: "test-dead".to_string(),
        ollama_endpoint: "http://127.0.0.1:9".to_string(),
        ..Default::default()
    });
    let res = run_goal(
        &store,
        codercraft_core::autonomy::RunRequest {
            workspace_dir: ws,
            goal: "add a health check endpoint".to_string(),
            session_id: Some("sess-p1-test".to_string()),
            max_steps: Some(2),
            mode: "build".to_string(),
            timeout_s: 10,
            brain: Some(dead),
        },
    )
    .await;
    assert_eq!(res["success"], true);
    assert_eq!(res["session_id"], "sess-p1-test");
    assert!(res["step_count"].as_u64().expect("steps") >= 1);

    // Explorer step must have found the seeded files; everything logged.
    let read = dispatch(
        Command::SessionRead {
            session_id: "sess-p1-test".to_string(),
            limit: Some(20),
        },
        &store,
    )
    .await;
    assert_eq!(read["success"], true);
    assert!(read["count"].as_u64().expect("turns") >= 2);
    let transcript = serde_json::to_string(&read["turns"]).expect("json");
    assert!(transcript.contains("add a health check endpoint"));
}

#[test]
fn test_p1_mode_budgets_and_readonly() {
    use codercraft_core::autonomy::{
        default_budget_for_mode, detect_test_command, extract_tool_json, tool_allowed_in_mode,
    };
    assert!(tool_allowed_in_mode("plan", "glob"));
    assert!(tool_allowed_in_mode("plan", "read"));
    assert!(!tool_allowed_in_mode("plan", "patch"));
    assert!(!tool_allowed_in_mode("plan", "shell"));
    assert!(tool_allowed_in_mode("build", "patch"));
    assert!(tool_allowed_in_mode("build", "shell"));
    assert_eq!(default_budget_for_mode("plan"), 5);
    assert_eq!(default_budget_for_mode("build"), 8);

    let v = extract_tool_json("noise ```json {\"tool\": \"glob\", \"args\": {\"pattern\": \"**/*.rs\"}} ``` tail")
        .expect("tool json");
    assert_eq!(v["tool"], "glob");
    assert!(extract_tool_json("FINAL: all done here").is_none());
    assert!(extract_tool_json("just some prose, no json").is_none());

    let root = std::env::temp_dir().join(format!(
        "codercraft-p1detect-{}",
        chrono::Utc::now().timestamp_millis()
    ));
    std::fs::create_dir_all(&root).expect("tmp root");
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").expect("seed");
    let ws = root.to_string_lossy().to_string();
    assert_eq!(detect_test_command(&ws).expect("cargo"), "cargo test");
    std::fs::remove_file(root.join("Cargo.toml")).expect("rm");
    assert!(detect_test_command(&ws).is_none());
    std::fs::write(root.join("pyproject.toml"), "[project]\nname = \"x\"\n").expect("seed");
    assert!(detect_test_command(&ws).expect("pytest").contains("pytest"));
}
