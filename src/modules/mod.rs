pub mod firewall;
pub mod user_audit;

use crate::app::Ctx;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Linux,
    Any,
}

impl Platform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "linux" => Self::Linux,
            _ => Self::Linux,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Info,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub message: String,
    pub fixable: bool,
}

pub trait Module: Send + Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn platform(&self) -> Platform;
    fn supported(&self) -> bool {
        matches!(self.platform(), Platform::Any) || self.platform() == Platform::current()
    }
    fn check(&self, ctx: &Ctx) -> anyhow::Result<Vec<Finding>>;
    fn fix(&self, ctx: &Ctx, finding_id: &str) -> anyhow::Result<()>;
}

pub fn registry() -> Vec<Box<dyn Module>> {
    vec![
        Box::new(firewall::Firewall),
        Box::new(user_audit::UserAudit),
    ]
}
