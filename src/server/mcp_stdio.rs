use crate::server::mcp_server::dispatch_mcp_value;
use crate::{workspace::WorkspaceContext, AppState};
use anyhow::Result;
use std::io::{self, BufRead, Write};

pub async fn run_stdio_loop(state: AppState, workspace: WorkspaceContext) -> Result<()> {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut input = String::new();

    loop {
        input.clear();
        if stdin.read_line(&mut input)? == 0 {
            break;
        }

        let payload = match serde_json::from_str(&input) {
            Ok(payload) => payload,
            Err(_) => continue,
        };

        let Some(response) = dispatch_mcp_value(state.clone(), workspace.clone(), payload)
            .await
            .ok()
            .flatten()
        else {
            continue;
        };

        let output = serde_json::to_vec(&response)?;
        stdout.write_all(&output)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }

    Ok(())
}
