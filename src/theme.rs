use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub primary: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub muted: Color,
}

impl Theme {
    pub fn from_name(name: &str, no_color: bool) -> Self {
        if no_color || name == "mono" {
            return Self { primary: Color::White, success: Color::White, warning: Color::White, danger: Color::White, muted: Color::Gray };
        }
        match name {
            "amber" => Self { primary: Color::Yellow, success: Color::Green, warning: Color::Yellow, danger: Color::Red, muted: Color::DarkGray },
            _ => Self { primary: Color::Cyan, success: Color::Green, warning: Color::Yellow, danger: Color::LightRed, muted: Color::Gray }, // cyber
        }
    }
}
