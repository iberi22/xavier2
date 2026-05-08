use clap::Subcommand;
use anyhow::Result;

#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum ChronicleCommand {
    /// Recolecta datos locales: commits, memoria, sesiones, cambios de código
    Harvest {
        /// Recolectar desde esta fecha
        #[arg(long)]
        since: Option<String>,
        /// Ruta al workspace
        #[arg(long)]
        workspace: Option<String>,
    },
    /// Toma el harvest más reciente, aplica redact, genera el post vía LLM
    Generate {
        /// Generar desde esta fecha
        #[arg(long)]
        since: Option<String>,
        /// Archivo de salida
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Muestra el post generado con placeholders visibles
    Preview {
        /// Archivo a previsualizar
        #[arg(long)]
        file: Option<String>,
    },
    /// Exporta el post a markdown en la ruta especificada
    Publish {
        /// Ruta de destino del archivo markdown
        #[arg(long)]
        to: Option<String>,
    },
}

pub async fn handle_chronicle_command(cmd: ChronicleCommand) -> Result<()> {
    match cmd {
        ChronicleCommand::Harvest { since, workspace } => {
            println!("Chronicle: Harvesting data...");
            if let Some(s) = since { println!("  Since: {}", s); }
            if let Some(w) = workspace { println!("  Workspace: {}", w); }
            // TODO: Implementation
        }
        ChronicleCommand::Generate { since, output } => {
            println!("Chronicle: Generating post...");
            if let Some(s) = since { println!("  Since: {}", s); }
            if let Some(o) = output { println!("  Output: {}", o); }
            // TODO: Implementation
        }
        ChronicleCommand::Preview { file } => {
            println!("Chronicle: Previewing post...");
            if let Some(f) = file { println!("  File: {}", f); }
            // TODO: Implementation
        }
        ChronicleCommand::Publish { to } => {
            println!("Chronicle: Publishing post...");
            if let Some(t) = to { println!("  To: {}", t); }
            // TODO: Implementation
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use crate::cli::{Cli, Command};

    #[test]
    fn test_chronicle_harvest_parsing() {
        let args = vec!["xavier2", "chronicle", "harvest", "--since", "2026-05-01", "--workspace", "/path/to/ws"];
        let cli = Cli::parse_from(args);

        match cli.cmd.unwrap() {
            Command::Chronicle { cmd } => {
                match cmd {
                    ChronicleCommand::Harvest { since, workspace } => {
                        assert_eq!(since, Some("2026-05-01".to_string()));
                        assert_eq!(workspace, Some("/path/to/ws".to_string()));
                    },
                    _ => panic!("Expected Harvest command"),
                }
            },
            _ => panic!("Expected Chronicle command"),
        }
    }

    #[test]
    fn test_chronicle_generate_parsing() {
        let args = vec!["xavier2", "chronicle", "generate", "--since", "2026-05-01", "--output", "post.md"];
        let cli = Cli::parse_from(args);

        match cli.cmd.unwrap() {
            Command::Chronicle { cmd } => {
                match cmd {
                    ChronicleCommand::Generate { since, output } => {
                        assert_eq!(since, Some("2026-05-01".to_string()));
                        assert_eq!(output, Some("post.md".to_string()));
                    },
                    _ => panic!("Expected Generate command"),
                }
            },
            _ => panic!("Expected Chronicle command"),
        }
    }

    #[test]
    fn test_chronicle_preview_parsing() {
        let args = vec!["xavier2", "chronicle", "preview", "--file", "harvest.json"];
        let cli = Cli::parse_from(args);

        match cli.cmd.unwrap() {
            Command::Chronicle { cmd } => {
                match cmd {
                    ChronicleCommand::Preview { file } => {
                        assert_eq!(file, Some("harvest.json".to_string()));
                    },
                    _ => panic!("Expected Preview command"),
                }
            },
            _ => panic!("Expected Chronicle command"),
        }
    }

    #[test]
    fn test_chronicle_publish_parsing() {
        let args = vec!["xavier2", "chronicle", "publish", "--to", "./daily.md"];
        let cli = Cli::parse_from(args);

        match cli.cmd.unwrap() {
            Command::Chronicle { cmd } => {
                match cmd {
                    ChronicleCommand::Publish { to } => {
                        assert_eq!(to, Some("./daily.md".to_string()));
                    },
                    _ => panic!("Expected Publish command"),
                }
            },
            _ => panic!("Expected Chronicle command"),
        }
    }
}
