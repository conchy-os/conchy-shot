use std::path::PathBuf;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "conchy-shot",
    version,
    about = "Modern Wayland screenshot & annotation tool with unified multi-monitor canvas for Conchy OS"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Open the interactive region selection UI (default when no command is given).
    Area(CaptureOptions),
    /// Capture a whole output directly.
    Screen(ScreenOptions),
    /// Print the resolved config path.
    ConfigPath,
    #[command(hide = true)]
    ClipboardServe,
}

#[derive(Debug, Clone, Args)]
pub struct CaptureOptions {
    /// Save to the configured screenshot path after copying to the clipboard.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub write_to_disk: bool,
    /// Include the pointer in the initial screenshot.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub show_pointer: bool,
    /// Override the output path for this capture.
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// Force unified multi-monitor canvas spanning all displays.
    #[arg(long)]
    pub unified: bool,
    /// Force per-output isolation mode.
    #[arg(long)]
    pub per_output: bool,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            write_to_disk: true,
            show_pointer: true,
            path: None,
            unified: false,
            per_output: false,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct ScreenOptions {
    #[command(flatten)]
    pub capture: CaptureOptions,
    /// Capture a specific output by name.
    #[arg(long)]
    pub output: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hidden_clipboard_subcommand() {
        let cli = Cli::parse_from(["conchy-shot", "clipboard-serve"]);
        assert!(matches!(cli.command, Some(Command::ClipboardServe)));
    }

    #[test]
    fn default_is_none_for_interactive() {
        let cli = Cli::parse_from(["conchy-shot"]);
        assert!(cli.command.is_none());
    }
}
