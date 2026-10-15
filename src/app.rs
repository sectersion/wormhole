#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ctx {
    pub dry_run: bool,
    pub auto_yes: bool,
}
