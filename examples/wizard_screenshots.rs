//! Generate `.ansi` files from the TUI wizard for documentation screenshots.
//!
//! Usage:
//!   cargo run --example wizard_screenshots --features cli-interactive --no-default-features
//!
//! Output: docs/screenshots/*.ansi

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = "docs/screenshots";
    let files = xavier::installer::wizard::render_all_steps_ansi(out_dir)?;
    println!("Generated {} .ansi files:", files.len());
    for f in &files {
        println!("  {}", f);
    }
    Ok(())
}
