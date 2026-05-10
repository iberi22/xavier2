//! Xavier2 Installer — TUI Onboarding Wizard
//!
//! Launches the interactive setup wizard that generates `config/xavier2.config.json`.
//! Required feature: `cli-interactive`

use anyhow::Result;

fn main() -> Result<()> {
    println!("Starting Xavier2 installer...");

    let state = xavier::installer::wizard::run_wizard()?;

    if state.config_written {
        println!("\n✓ Configuration saved to {}", state.config_path);
        println!("  Start Xavier2 with: xavier serve");
        println!("  Dashboard:          xavier tui");
    } else {
        println!("\n⚠ Installer exited without saving.");
    }

    Ok(())
}
