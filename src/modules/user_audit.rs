use super::{Finding, Module, Platform};
use crate::app::Ctx;

pub struct UserAudit;

impl Module for UserAudit {
    fn id(&self) -> &'static str {
        "user-audit"
    }
    fn description(&self) -> &'static str {
        "List local users (stub)"
    }
    fn platform(&self) -> Platform {
        Platform::Any
    }
    fn check(&self, _ctx: &Ctx) -> anyhow::Result<Vec<Finding>> {
        Ok(vec![])
    }
    fn fix(&self, _ctx: &Ctx, _finding_id: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
