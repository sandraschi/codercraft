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
