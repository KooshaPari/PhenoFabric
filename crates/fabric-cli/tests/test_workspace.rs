//! Tests for fabric CLI workspace subcommand.
//!
//! Uses real filesystem (temp dirs) to validate create/list/show/delete/release.

use std::fs;
use std::path::PathBuf;

/// Helper: create a temp workspace directory.
fn temp_workspace() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fabric-cli-test-{}-{}",
        std::process::id(),
        uuid::Uuid::now_v7()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn workspace_create_and_list() {
    let ws = temp_workspace();
    let result = fabric_cli::commands::workspace::dispatch(
        &fabric_cli::WorkspaceCommand::Create(fabric_cli::commands::workspace::CreateArgs {
            name: "test-ws".into(),
            tier: "L5Loopback".into(),
            topology: None,
        }),
        &ws,
    );
    assert!(result.is_ok(), "workspace create failed: {:?}", result);

    // List should show it
    let result = fabric_cli::commands::workspace::dispatch(
        &fabric_cli::WorkspaceCommand::List(fabric_cli::commands::workspace::ListArgs {
            json: false,
        }),
        &ws,
    );
    assert!(result.is_ok(), "workspace list failed: {:?}", result);

    // Cleanup
    fs::remove_dir_all(&ws).ok();
}

#[test]
fn workspace_create_show_delete() {
    let ws = temp_workspace();

    // Create
    fabric_cli::commands::workspace::dispatch(
        &fabric_cli::WorkspaceCommand::Create(fabric_cli::commands::workspace::CreateArgs {
            name: "my-workspace".into(),
            tier: "L2CrossNumaShm".into(),
            topology: None,
        }),
        &ws,
    )
    .unwrap();

    // Show
    let result = fabric_cli::commands::workspace::dispatch(
        &fabric_cli::WorkspaceCommand::Show(fabric_cli::commands::workspace::ShowArgs {
            name: "my-workspace".into(),
            json: true,
        }),
        &ws,
    );
    assert!(result.is_ok(), "workspace show failed: {:?}", result);

    // Delete with force
    let result = fabric_cli::commands::workspace::dispatch(
        &fabric_cli::WorkspaceCommand::Delete(fabric_cli::commands::workspace::DeleteArgs {
            name: "my-workspace".into(),
            force: true,
        }),
        &ws,
    );
    assert!(result.is_ok(), "workspace delete failed: {:?}", result);

    fs::remove_dir_all(&ws).ok();
}

#[test]
fn workspace_release() {
    let ws = temp_workspace();

    // Create
    fabric_cli::commands::workspace::dispatch(
        &fabric_cli::WorkspaceCommand::Create(fabric_cli::commands::workspace::CreateArgs {
            name: "release-test".into(),
            tier: "L0SameProcess".into(),
            topology: None,
        }),
        &ws,
    )
    .unwrap();

    // Release (no seats, should succeed with 0 released)
    let result = fabric_cli::commands::workspace::dispatch(
        &fabric_cli::WorkspaceCommand::Release(fabric_cli::commands::workspace::ReleaseArgs {
            id: "release-test".into(),
            force: true,
            json: false,
        }),
        &ws,
    );
    assert!(result.is_ok(), "workspace release failed: {:?}", result);

    // Cleanup
    let _ = fabric_cli::commands::workspace::dispatch(
        &fabric_cli::WorkspaceCommand::Delete(fabric_cli::commands::workspace::DeleteArgs {
            name: "release-test".into(),
            force: true,
        }),
        &ws,
    );
    fs::remove_dir_all(&ws).ok();
}
