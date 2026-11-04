//! Integration and dispatch verification tests for CoderCraft.
use codercraft_core::{dispatch, Command};
use databasecraft_store::DatabaseStore;

#[tokio::test]
async fn test_codercraft_status_and_control_port() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;
    let res = dispatch(Command::Status, &store).await;
    assert_eq!(res["success"], true);
    assert_eq!(res["app"], "codercraft");
    assert_eq!(res["control_port"], 14128);
    Ok(())
}

#[tokio::test]
async fn test_codercraft_editor_and_refactor_dispatch() -> Result<(), Box<dyn std::error::Error>> {
    let store = DatabaseStore::open_in_memory()?;

    // 1. Open file buffer
    let open_res = dispatch(Command::OpenFile { path: "src/main.rs".to_string() }, &store).await;
    assert_eq!(open_res["success"], true);
    assert_eq!(open_res["language"], "rust");

    // 2. Index workspace symbols
    let idx_res = dispatch(Command::IndexWorkspace { workspace_dir: ".".to_string() }, &store).await;
    assert_eq!(idx_res["success"], true);
    assert!(idx_res["symbols_mapped"].as_u64().unwrap_or(0) > 100);

    // 3. Agent synthesize refactor proposal
    let refactor_res = dispatch(
        Command::AgentRefactor {
            intent: "Add zero-panic error handling".to_string(),
            target_file: "src/main.rs".to_string(),
        },
        &store,
    )
    .await;
    assert_eq!(refactor_res["success"], true);
    let patches = refactor_res["proposal"]["patches"].as_array().ok_or("missing patches")?;
    assert_eq!(patches.len(), 1);

    // 4. Run test suite
    let test_res = dispatch(Command::RunTestSuite { test_filter: None }, &store).await;
    assert_eq!(test_res["success"], true);
    assert_eq!(test_res["status"], "ALL_GREEN");

    Ok(())
}
