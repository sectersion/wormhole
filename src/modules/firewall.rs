use super::{Finding, Module, Platform, Severity};
use crate::app::Ctx;

pub struct Firewall;

impl Module for Firewall {
    fn id(&self) -> &'static str {
        "firewall"
    }
    fn description(&self) -> &'static str {
        "Ensure host firewall is enabled"
    }
    fn platform(&self) -> Platform {
        Platform::Any
    }
    fn check(&self, _ctx: &Ctx) -> anyhow::Result<Vec<Finding>> {
        // stub: real impl shells out to netsh / ufw
        Ok(vec![Finding {
            id: "fw-off".into(),
            severity: Severity::High,
            message: "Firewall status unknown (stub)".into(),
            fixable: true,
        }])
    }
    fn fix(&self, ctx: &Ctx, finding_id: &str) -> anyhow::Result<()> {
        anyhow::ensure!(finding_id == "fw-off", "unknown finding");
        if ctx.dry_run {
            return Ok(());
        }
        match Platform::current() {
            Platform::Windows => {
                crate::exec::cmd("netsh", &["advfirewall", "set", "allprofiles", "state", "on"])?;
            }
            _ => {
                crate::exec::cmd("ufw", &["--force", "enable"])?;
            }
        }
        Ok(())
    }
}
