use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Gauge, List, ListItem, Padding, Paragraph, Row, Sparkline, Table,
        Tabs, Wrap,
    },
};
use crate::{app::Ctx, cli::Cli, modules::registry, theme::Theme};

struct ModState {
    checked: bool,
    status: Status,
    findings: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Pending,
    Scanning,
    Ok,
    Warn,
    Fail,
}

impl Status {
    fn label(self) -> (&'static str, usize) {
        match self {
            Status::Pending => ("○ PENDING", 0),
            Status::Scanning => ("◌ SCAN…", 1),
            Status::Ok => ("● SECURE", 2),
            Status::Warn => ("▲ WARN", 3),
            Status::Fail => ("✖ CRITICAL", 4),
        }
    }
}

fn status_style(s: Status, t: &Theme) -> Style {
    match s {
        Status::Ok => Style::default().fg(t.success).bold(),
        Status::Warn => Style::default().fg(t.warning).bold(),
        Status::Fail => Style::default().fg(t.danger).bold(),
        Status::Scanning => Style::default().fg(t.primary).bold(),
        Status::Pending => Style::default().fg(t.muted),
    }
}

pub fn run_tui(cli: &Cli) -> anyhow::Result<()> {
    let themes = ["cyber", "amber", "mono"];
    let mut theme_idx = themes.iter().position(|t| *t == cli.theme).unwrap_or(0);
    let mut theme = Theme::from_name(themes[theme_idx], cli.no_color);
    let mods = registry();
    let mut state: Vec<ModState> = mods
        .iter()
        .map(|_| ModState { checked: true, status: Status::Pending, findings: 0 })
        .collect();
    let mut selected: usize = 0;
    let mut tab: usize = 0;
    let tabs = ["◈ MODULES", "⚠ FINDINGS", "≡ AUDIT LOG", "? HELP"];
    let mut logs: Vec<String> = vec![
        "[init] wormhole v0.1.0 — CyberPatriot hardening console".into(),
        format!("[init] platform: {}  elevated: {}", std::env::consts::OS, crate::exec::is_elevated()),
        "[init] press ? for help, r to scan, space to toggle".into(),
    ];
    let mut dry_run = cli.dry_run;
    let mut show_help = false;
    let mut filter = String::new();
    let mut filtering = false;
    let mut progress: u16 = 0;
    let mut scan_tick: usize = 0;
    let mut spark: Vec<u64> = vec![3, 5, 2, 8, 6, 9, 4, 7, 5, 8, 10, 6];
    let mut terminal = ratatui::init();
    let layout_wide = cli.layout != "compact";

    let push = |logs: &mut Vec<String>, s: String| {
        logs.push(s);
        if logs.len() > 200 {
            logs.remove(0);
        }
    };

    loop {
        let cur_theme = theme.clone();
        terminal.draw(|f| {
            let area = f.area();
            // background frame
            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // header
                    Constraint::Length(3), // tabs + filter
                    Constraint::Min(8),    // main
                    Constraint::Length(7), // log + gauge
                    Constraint::Length(3), // footer
                ])
                .split(area);

            // ── header ──
            let elevated = crate::exec::is_elevated();
            let header = Paragraph::new(Line::from(vec![
                Span::styled(" ▓▒░ WORMHOLE ", Style::default().fg(cur_theme.primary).add_modifier(Modifier::BOLD | Modifier::REVERSED)),
                Span::raw("  CyberPatriot Hardening Console  "),
                Span::styled(format!("v0.1.0-{}", std::env::consts::OS), Style::default().fg(cur_theme.muted)),
                Span::raw("   "),
                Span::styled(
                    if elevated { " ▲ ELEVATED " } else { " ○ UNPRIVILEGED " },
                    if elevated {
                        Style::default().fg(cur_theme.success).bold()
                    } else {
                        Style::default().fg(cur_theme.danger).bold()
                    },
                ),
                Span::styled(
                    if dry_run { " DRY-RUN " } else { " LIVE " },
                    Style::default().fg(cur_theme.warning).bold(),
                ),
                Span::styled(format!(" {} ", themes[theme_idx]), Style::default().fg(cur_theme.muted)),
            ]))
            .block(Block::new().borders(Borders::ALL).border_set(symbols::border::DOUBLE).fg(cur_theme.primary));
            f.render_widget(header, root[0]);

            // ── tabs ──
            let tabrow = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(20), Constraint::Length(34)])
                .split(root[1]);
            f.render_widget(
                Tabs::new(tabs.to_vec())
                    .select(tab)
                    .style(Style::default().fg(cur_theme.muted))
                    .highlight_style(Style::default().fg(cur_theme.primary).bold())
                    .block(Block::new().borders(Borders::ALL).title(" views  [1-4] ")),
                tabrow[0],
            );
            f.render_widget(
                Paragraph::new(if filtering {
                    format!("❯ filter: {filter}▌")
                } else {
                    format!("❯ filter: {filter}   (/ to edit)")
                })
                .block(Block::new().borders(Borders::ALL).title(" filter ")),
                tabrow[1],
            );

            // ── main ──
            let main = if layout_wide {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
                    .split(root[2])
            } else {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                    .split(root[2])
            };

            // left: module table
            let rows: Vec<Row> = mods
                .iter()
                .enumerate()
                .filter(|(_, m)| filter.is_empty() || m.id().contains(filter.as_str()))
                .map(|(i, m)| {
                    let st = &state[i];
                    let (label, _) = st.status.label();
                    let plat = match m.platform() {
                        crate::modules::Platform::Any => "win+lin",
                        crate::modules::Platform::Windows => "win",
                        crate::modules::Platform::Linux => "lin",
                    };
                    let check = if st.checked { "[◉]" } else { "[ ]" };
                    let cursor = if i == selected { "▶" } else { " " };
                    let mut row = Row::new(vec![
                        Cell::from(format!("{cursor} {check}")),
                        Cell::from(m.id()),
                        Cell::from(plat),
                        Cell::from(Span::styled(label, status_style(st.status, &cur_theme))),
                        Cell::from(format!("{}", st.findings)),
                    ]);
                    if i == selected {
                        row = row.style(Style::default().bg(cur_theme.primary).fg(ratatui::style::Color::Black).bold());
                    } else if !m.supported() {
                        row = row.style(Style::default().fg(cur_theme.muted));
                    }
                    row
                })
                .collect();
            let widths = [Constraint::Length(8), Constraint::Min(14), Constraint::Length(9), Constraint::Length(13), Constraint::Length(6)];
            f.render_widget(
                Table::new(rows, widths)
                    .header(Row::new(["", "MODULE", "PLAT", "STATUS", "FIND"]).bold().fg(cur_theme.primary))
                    .block(Block::new().borders(Borders::ALL).title(format!(" modules [{}/{}]  (space toggle, a all) ", state.iter().filter(|s| s.checked).count(), mods.len())).fg(cur_theme.primary)),
                main[0],
            );

            // right: detail + findings + sparkline
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(6), Constraint::Length(5), Constraint::Min(4)])
                .split(main[1]);
            let detail = mods.get(selected).map(|m| {
                let st = &state[selected];
                vec![
                    Line::from(vec![Span::styled(m.id(), Style::default().fg(cur_theme.primary).bold()), Span::raw(format!("  {:?}", m.platform()))]),
                    Line::from(Span::raw(m.description())),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("status: "),
                        Span::styled(st.status.label().0, status_style(st.status, &cur_theme)),
                        Span::raw(format!("   findings: {}   supported: {}", st.findings, m.supported())),
                    ]),
                ]
            }).unwrap_or_default();
            f.render_widget(
                Paragraph::new(detail).wrap(Wrap { trim: true }).block(
                    Block::new().borders(Borders::ALL).title(" ◆ detail ").fg(cur_theme.primary).padding(Padding::horizontal(1)),
                ),
                right[0],
            );
            f.render_widget(
                Sparkline::default().data(&spark)
                    .block(Block::new().borders(Borders::ALL).title(" score trend "))
                    .style(Style::default().fg(cur_theme.primary)),
                right[1],
            );
            // mini findings preview for selected module
            let ctx = Ctx { dry_run: true, auto_yes: false };
            let preview: Vec<ListItem> = mods.get(selected).and_then(|m| m.check(&ctx).ok()).map(|v| {
                v.into_iter().take(5).map(|x| ListItem::new(format!("[{:?}] {}", x.severity, x.message))).collect()
            }).unwrap_or_else(|| vec![ListItem::new("no cached findings — press r to scan")]);
            f.render_widget(
                List::new(preview).block(Block::new().borders(Borders::ALL).title(" ⚠ findings preview ")),
                right[2],
            );

            // ── log + gauge ──
            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
                .split(root[3]);
            let n = logs.len().saturating_sub(5);
            let logtext: Vec<Line> = logs[n..].iter().map(|l| {
                let style = if l.contains("CRIT") || l.contains("fail") {
                    Style::default().fg(cur_theme.danger)
                } else if l.contains("WARN") || l.contains("dry") {
                    Style::default().fg(cur_theme.warning)
                } else if l.contains("ok") || l.contains("fixed") || l.contains("SECURE") {
                    Style::default().fg(cur_theme.success)
                } else {
                    Style::default()
                };
                Line::styled(l.clone(), style)
            }).collect();
            f.render_widget(
                Paragraph::new(logtext).block(Block::new().borders(Borders::ALL).title(" ≡ live audit log ")),
                bottom[0],
            );
            f.render_widget(
                Gauge::default()
                    .block(Block::new().borders(Borders::ALL).title(" coverage "))
                    .gauge_style(Style::default().fg(cur_theme.success))
                    .percent(progress)
                    .label(format!("{progress}% hardened")),
                bottom[1],
            );

            // ── footer ──
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw(" j/k move · space toggle · a all · r scan · f fix · d dry-run · t theme · / filter · 1-4 views · ? help · q quit "),
                ]))
                .block(Block::new().borders(Borders::ALL).title(" keys ")),
                root[4],
            );

            // help modal
            if show_help {
                let popup = centered(area, 60, 14);
                f.render_widget(Clear, popup);
                f.render_widget(
                    Paragraph::new(vec![
                        Line::from(" WORMHOLE — keys ".bold()),
                        Line::from(""),
                        Line::from(" j/k,↑/↓  move      space toggle module"),
                        Line::from(" a         toggle all   r scan selected/all-checked"),
                        Line::from(" f         apply fixes  d toggle dry-run"),
                        Line::from(" t         cycle theme  / filter, Esc clear"),
                        Line::from(" 1-4       switch views (? shows this overlay)"),
                        Line::from(""),
                        Line::from("Enter: close this help".italic()),
                    ])
                    .block(Block::new().borders(Borders::ALL).title(" help ").fg(cur_theme.primary)),
                    popup,
                );
            }
        })?;

        if event::poll(std::time::Duration::from_millis(150))? {
            if let Event::Key(k) = event::read()? {
                if filtering {
                    match k.code {
                        KeyCode::Esc | KeyCode::Enter => filtering = false,
                        KeyCode::Backspace => {
                            filter.pop();
                        }
                        KeyCode::Char(c) => filter.push(c),
                        _ => {}
                    }
                    continue;
                }
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if show_help {
                            show_help = false;
                        } else {
                            break;
                        }
                    }
                    KeyCode::Char('?') => show_help = !show_help,
                    KeyCode::Enter => show_help = false,
                    KeyCode::Char('j') | KeyCode::Down => selected = (selected + 1).min(mods.len().saturating_sub(1)),
                    KeyCode::Char('k') | KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Char(' ') => {
                        if let Some(s) = state.get_mut(selected) {
                            s.checked = !s.checked;
                        }
                    }
                    KeyCode::Char('a') => {
                        let all = state.iter().all(|s| s.checked);
                        for s in state.iter_mut() {
                            s.checked = !all;
                        }
                    }
                    KeyCode::Char('d') => {
                        dry_run = !dry_run;
                        push(&mut logs, format!("[mode] dry-run = {dry_run}"));
                    }
                    KeyCode::Char('t') => {
                        theme_idx = (theme_idx + 1) % themes.len();
                        theme = Theme::from_name(themes[theme_idx], cli.no_color);
                    }
                    KeyCode::Char('/') => filtering = true,
                    KeyCode::Char(c @ '1'..='4') => tab = (c as usize) - ('1' as usize),
                    KeyCode::Char('r') => {
                        // scan: mark checked as scanning then resolve
                        for (i, s) in state.iter_mut().enumerate() {
                            if s.checked && mods[i].supported() {
                                s.status = Status::Scanning;
                            }
                        }
                        scan_tick = 3; // resolve after a few frames
                        push(&mut logs, format!("[scan] started (dry_run={dry_run})…"));
                    }
                    KeyCode::Char('f') => {
                        let ctx = Ctx { dry_run, auto_yes: true };
                        for (i, m) in mods.iter().enumerate() {
                            if !state[i].checked || !m.supported() {
                                continue;
                            }
                            if let Ok(findings) = m.check(&ctx) {
                                for fd in findings.iter().filter(|x| x.fixable) {
                                    if dry_run {
                                        push(&mut logs, format!("[dry-run] would fix {}: {}", m.id(), fd.message));
                                    } else if m.fix(&ctx, &fd.id).is_ok() {
                                        push(&mut logs, format!("[ok] fixed {}: {}", m.id(), fd.message));
                                    } else {
                                        push(&mut logs, format!("[fail] {}: {}", m.id(), fd.message));
                                    }
                                }
                            }
                        }
                        progress = progress.saturating_add(10).min(100);
                    }
                    _ => {
                        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
                            break;
                        }
                    }
                }
            }
        }
        // resolve fake scan animation
        if scan_tick > 0 {
            scan_tick -= 1;
            if scan_tick == 0 {
                let ctx = Ctx { dry_run: true, auto_yes: false };
                let mut crit = 0;
                for (i, m) in mods.iter().enumerate() {
                    if state[i].status != Status::Scanning {
                        continue;
                    }
                    match m.check(&ctx) {
                        Ok(fd) => {
                            state[i].findings = fd.len();
                            state[i].status = if fd.is_empty() {
                                Status::Ok
                            } else if fd.iter().any(|x| matches!(x.severity, crate::modules::Severity::Critical)) {
                                crit += 1;
                                Status::Fail
                            } else {
                                Status::Warn
                            };
                            push(&mut logs, format!("[scan] {}: {} finding(s) → {}", m.id(), fd.len(), state[i].status.label().0));
                        }
                        Err(e) => {
                            state[i].status = Status::Fail;
                            push(&mut logs, format!("[fail] {}: {e:#}", m.id()));
                        }
                    }
                }
                spark.push(crit as u64);
                if spark.len() > 30 {
                    spark.remove(0);
                }
                let done = state.iter().filter(|s| s.status == Status::Ok).count();
                progress = ((done as f32 / mods.len() as f32) * 100.0) as u16;
            }
        }
    }
    ratatui::restore();
    Ok(())
}

fn centered(area: Rect, w_pct: u16, h: u16) -> Rect {
    let w = area.width * w_pct / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}
