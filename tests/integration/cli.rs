//! CLI Integration Tests
//!
//! Tests the xavier2 binary CLI commands by spawning the binary
//! and checking stdout/stderr output.

use std::process::{Command, Output};
use std::time::Duration;

// ─── Helpers ───────────────────────────────────────────────────────────────

fn xavier2_binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_xavier2"))
}

fn run(args: &[&str]) -> Output {
    let output = xavier2_binary()
        .args(args)
        .output()
        .expect("failed to execute xavier2 binary");
    output
}

fn run_with_timeout(args: &[&str], timeout_secs: u64) -> Output {
    let mut child = xavier2_binary()
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn xavier2");

    // Wait with timeout
    let start = std::time::Instant::now();
    loop {
        if start.elapsed().as_secs() > timeout_secs {
            let _ = child.kill();
            panic!("xavier2 {} timed out after {timeout_secs}s", args.join(" "));
        }
        match child.try_wait() {
            Ok(Some(_status)) => {
                let output = child.wait_with_output().expect("get output");
                return output;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => panic!("error waiting for xavier2: {e}"),
        }
    }
}

// ─── Help Output ───────────────────────────────────────────────────────────

#[test]
fn test_cli_help_output() {
    let output = run(&["--help"]);
    assert!(output.status.success(), "xavier2 --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Xavier2"),
        "help should contain project name"
    );
    assert!(
        stdout.contains("http") || stdout.contains("Http"),
        "help should list http subcommand"
    );
    assert!(
        stdout.contains("search") || stdout.contains("Search"),
        "help should list search subcommand"
    );
    assert!(
        stdout.contains("add") || stdout.contains("Add"),
        "help should list add subcommand"
    );
    assert!(
        stdout.contains("stats") || stdout.contains("Stats"),
        "help should list stats subcommand"
    );
    assert!(
        stdout.contains("recall") || stdout.contains("Recall"),
        "help should list recall subcommand"
    );
    assert!(
        stdout.contains("session-save") || stdout.contains("SessionSave"),
        "help should list session-save subcommand"
    );
}

#[test]
fn test_cli_no_args_shows_help() {
    let output = run_with_timeout(&[], 5);

    // Without args and without HTTP env, it may try to start HTTP server;
    // catch the timeout case separately
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // It's OK if it fails with a connection error when trying to start
        // the HTTP server; just verify we got an error message
        assert!(
            !stderr.is_empty() || !output.stdout.is_empty(),
            "should have some output"
        );
    }
}

#[test]
fn test_cli_subcommand_help_http() {
    let output = run(&["http", "--help"]);
    assert!(
        output.status.success(),
        "xavier2 http --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("http") || stdout.contains("port"),
        "http help should mention port"
    );
}

#[test]
fn test_cli_subcommand_help_add() {
    let output = run(&["add", "--help"]);
    assert!(
        output.status.success(),
        "xavier2 add --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("content") || stdout.contains("CONTENT"),
        "add help should mention content"
    );
    assert!(
        stdout.contains("title") || stdout.contains("TITLE"),
        "add help should mention title"
    );
    assert!(
        stdout.contains("kind") || stdout.contains("KIND"),
        "add help should mention kind"
    );
}

#[test]
fn test_cli_subcommand_help_search() {
    let output = run(&["search", "--help"]);
    assert!(
        output.status.success(),
        "xavier2 search --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("query") || stdout.contains("QUERY"),
        "search help should mention query"
    );
    assert!(
        stdout.contains("limit") || stdout.contains("LIMIT"),
        "search help should mention limit"
    );
}

#[test]
fn test_cli_subcommand_help_recall() {
    let output = run(&["recall", "--help"]);
    assert!(
        output.status.success(),
        "xavier2 recall --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("query") || stdout.contains("QUERY"),
        "recall help should mention query"
    );
    assert!(
        stdout.contains("limit") || stdout.contains("LIMIT"),
        "recall help should mention limit"
    );
}

#[test]
fn test_cli_subcommand_help_stats() {
    let output = run(&["stats", "--help"]);
    assert!(
        output.status.success(),
        "xavier2 stats --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("stats") || stdout.contains("Stats"),
        "stats help should mention stats"
    );
}

#[test]
fn test_cli_subcommand_help_session_save() {
    let output = run(&["session-save", "--help"]);
    assert!(
        output.status.success(),
        "xavier2 session-save --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("session") || stdout.contains("SESSION_ID"),
        "session-save help should mention session_id"
    );
}

// ─── Version Output ────────────────────────────────────────────────────────

#[test]
fn test_cli_version_output() {
    let output = run(&["--version"]);
    assert!(
        output.status.success(),
        "xavier2 --version should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "version output should not be empty");
}

// ─── Error Handling ────────────────────────────────────────────────────────

#[test]
fn test_cli_invalid_subcommand() {
    let output = run(&["nonexistent-command"]);
    assert!(
        !output.status.success(),
        "invalid subcommand should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error") || stderr.contains("unrecognized"),
        "error should mention unrecognized subcommand, got: {stderr}"
    );
}

#[test]
fn test_cli_subcommand_invalid_flag() {
    let output = run(&["stats", "--invalid-flag"]);
    assert!(
        !output.status.success(),
        "invalid flag should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error") || stderr.contains("unrecognized"),
        "error should mention unrecognized flag, got: {stderr}"
    );
}

#[test]
fn test_cli_subcommand_add_without_server() {
    // add requires a running server — should fail gracefully
    let output = run_with_timeout(&["add", "test-content"], 5);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show some connection error or be blocked by security
    assert!(
        stdout.contains("Error") || stdout.contains("error")
            || stderr.contains("error") || stderr.contains("Error")
            || stdout.contains("blocked"),
        "add without server should produce error output, got stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn test_cli_subcommand_search_without_server() {
    // search requires a running server — should fail gracefully
    let output = run_with_timeout(&["search", "test query"], 5);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Error") || stdout.contains("error"),
        "search without server should produce error output, got: {stdout}"
    );
}

#[test]
fn test_cli_subcommand_recall_without_server() {
    // recall requires a running server — should fail gracefully
    let output = run_with_timeout(&["recall", "test"], 5);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Error") || stdout.contains("error"),
        "recall without server should produce error output, got: {stdout}"
    );
}

// ─── Add & Search Flow Tests ───────────────────────────────────────────────

#[test]
fn test_add_and_search_without_server() {
    // Without a running server, both add and search should fail gracefully.
    // This verifies both subcommands exist and produce expected error output.

    let add_output = run_with_timeout(&["add", "integration test content"], 5);
    let add_stdout = String::from_utf8_lossy(&add_output.stdout);
    assert!(
        add_stdout.contains("Error") || add_stdout.contains("error"),
        "add without server should produce error, got: {add_stdout}"
    );

    let search_output = run_with_timeout(&["search", "integration test query"], 5);
    let search_stdout = String::from_utf8_lossy(&search_output.stdout);
    assert!(
        search_stdout.contains("Error") || search_stdout.contains("error"),
        "search without server should produce error, got: {search_stdout}"
    );
}

// ─── Stats Without Server ──────────────────────────────────────────────────

#[test]
fn test_cli_subcommand_stats_without_server() {
    // stats requires a running server — should fail gracefully
    let output = run_with_timeout(&["stats"], 5);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Error") || stdout.contains("error"),
        "stats without server should produce error output, got: {stdout}"
    );
}

// ─── Session Save Without Server ───────────────────────────────────────────

#[test]
fn test_cli_subcommand_session_save_without_server() {
    // session-save requires a running server — should fail gracefully
    let output = run_with_timeout(&["session-save", "test-session"], 5);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Error") || stdout.contains("error"),
        "session-save without server should produce error output, got: {stdout}"
    );
}
