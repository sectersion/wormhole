use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(name = "wormhole", version)]
pub struct Cli {
    /// Force CLI mode (default: TUI if TTY else CLI)
    #[arg(long, global = true)]
    pub cli: bool,
    /// Check only, no changes
    #[arg(long, global = true)]
    pub dry_run: bool,
    /// Auto-approve all fixes
    #[arg(long, global = true)]
    pub yes: bool,
    /// Machine-readable output (CLI only)
    #[arg(long, global = true)]
    pub json: bool,
    /// Disable colors
    #[arg(long, global = true)]
    pub no_color: bool,
    /// Theme: cyber | amber | mono
    #[arg(long, global = true, default_value = "cyber")]
    pub theme: String,
    /// Layout: wide | compact
    #[arg(long, global = true, default_value = "wide")]
    pub layout: String,
    /// Filter modules by id
    #[arg(long, global = true)]
    pub modules: Option<String>,
    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Cmd {
    List,
    Check { modules: Vec<String> },
    Fix { modules: Vec<String> },
}

impl Cli {
    pub fn use_tui(&self) -> bool {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }
    pub fn module_filter(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Some(m) = &self.modules {
            out.extend(m.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
        }
        match &self.cmd {
            Some(Cmd::Check { modules }) | Some(Cmd::Fix { modules }) => out.extend(modules.clone()),
            _ => {}
        }
        out
    }
}

pub fn run_cli(cli: &Cli) -> anyhow::Result<()> {
    use crate::modules::registry;
    let filter = cli.module_filter();
    let mods = registry();
    let mods: Vec<_> = mods
        .into_iter()
        .filter(|m| filter.is_empty() || filter.contains(&m.id().to_string()))
        .collect();
    match &cli.cmd {
        None | Some(Cmd::List) => {
            if cli.json {
                let v: Vec<_> = mods
                    .iter()
                    .map(|m| {
                        serde_json::json!({"id": m.id(), "platform": format!("{:?}", m.platform()), "supported": m.supported(), "desc": m.description()})
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                for m in &mods {
                    let sup = if m.supported() { "" } else { " [skipped: platform]" };
                    println!("{:<20} {:?}{} - {}", m.id(), m.platform(), sup, m.description());
                }
            }
        }
        Some(Cmd::Check { .. }) => {
            for m in &mods {
                if !m.supported() {
                    println!("{}: skipped (platform {:?})", m.id(), m.platform());
                    continue;
                }
                let ctx = crate::app::Ctx { dry_run: true, auto_yes: cli.yes };
                match m.check(&ctx) {
                    Ok(findings) => {
                        if cli.json {
                            println!("{}", serde_json::to_string_pretty(&serde_json::json!({"module": m.id(), "findings": findings}))?);
                        } else {
                            println!("== {}: {} finding(s)", m.id(), findings.len());
                            for f in findings {
                                println!("  [{:?}] {} (fixable: {})", f.severity, f.message, f.fixable);
                            }
                        }
                    }
                    Err(e) => println!("{}: check failed: {:#}", m.id(), e),
                }
            }
        }
        Some(Cmd::Fix { .. }) => {
            for m in &mods {
                if !m.supported() {
                    println!("{}: skipped", m.id());
                    continue;
                }
                let ctx = crate::app::Ctx { dry_run: cli.dry_run, auto_yes: cli.yes };
                let findings = m.check(&ctx)?;
                for f in findings {
                    if !f.fixable {
                        println!("[info] {}", f.message);
                        continue;
                    }
                    let apply = if cli.dry_run {
                        println!("[dry-run] would fix: {}", f.message);
                        false
                    } else if cli.yes {
                        true
                    } else {
                        println!("Fix '{}'? [y/N/a/q]", f.message);
                        let mut s = String::new();
                        std::io::stdin().read_line(&mut s)?;
                        matches!(s.trim().to_lowercase().as_str(), "y" | "yes" | "a" | "all")
                    };
                    if apply {
                        m.fix(&ctx, &f.id)?;
                        println!("fixed: {}", f.message);
                    }
                }
            }
        }
    }
    Ok(())
}
