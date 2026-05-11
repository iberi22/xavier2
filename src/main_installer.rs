//! xavier-installer — TUI onboarding wizard binary.
//!
//! Generates a `config/xavier2.config.json` via interactive wizard.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "cli-interactive")]
    {
        let state = xavier::installer::wizard::run_wizard()?;
        if state.config_written {
            println!("✓ Configuration saved to {}", state.config_path);
        } else {
            println!("✗ Wizard cancelled.");
        }
        return Ok(());
    }

    #[cfg(not(feature = "cli-interactive"))]
    {
        eprintln!(
            "xavier-installer requires the 'cli-interactive' feature.\n\
             Build with: cargo build --bin xavier-installer --features cli-interactive --no-default-features"
        );
        std::process::exit(1);
    }
}
