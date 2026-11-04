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
    assert!(res["servers"].as_array().unwrap().len() >= 2);

    let console_res = dispatch(Command::ExecuteConsoleCommand {
        command_line: "cargo --version".to_string(),
        working_dir: None,
    }, &store).await;
    assert_eq!(console_res["success"], true);
    assert_eq!(console_res["exit_code"], 0);
}
