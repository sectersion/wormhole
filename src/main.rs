pub mod app;
pub mod cli;
pub mod exec;
pub mod modules;
pub mod theme;
pub mod tui;

use clap::Parser;
use cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if cli.cli || !cli.use_tui() {
        cli::run_cli(&cli)
    } else {
        tui::run_tui(&cli)
    }
}
