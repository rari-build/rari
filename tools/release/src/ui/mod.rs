use crate::app::App;
use crate::package::Package;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

pub fn render_package_selection(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let title_text =
        if app.dry_run { "rari Release Manager [DRY RUN MODE]" } else { "rari Release Manager" };
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = app
        .packages
        .iter()
        .enumerate()
        .map(|(idx, pkg)| {
            let prefix = if idx == app.selected_package_idx { "> " } else { "  " };
            let content = format!("{}{} (v{})", prefix, pkg.name, pkg.current_version);
            ListItem::new(content).style(if idx == app.selected_package_idx {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
        })
        .collect();

    let list =
        List::new(items).block(Block::default().borders(Borders::ALL).title("Select Package"));
    frame.render_widget(list, chunks[1]);

    let help = Paragraph::new("Up/Down: Navigate  Enter: Select  q/Esc: Quit")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}

pub fn render_version_selection(frame: &mut Frame, app: &App, package: &Package) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Package: "),
            Span::styled(
                &package.name,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Current Version: "),
            Span::styled(&package.current_version, Style::default().fg(Color::Yellow)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Package Info"));
    frame.render_widget(info, chunks[0]);

    let commits: Vec<Line> =
        app.recent_commits.iter().take(5).map(|c| Line::from(format!("* {}", c))).collect();
    let commits_widget = Paragraph::new(commits)
        .block(Block::default().borders(Borders::ALL).title("Recent Commits"));
    frame.render_widget(commits_widget, chunks[1]);

    let items: Vec<ListItem> = app
        .version_types
        .iter()
        .enumerate()
        .map(|(idx, vt)| {
            let prefix = if idx == app.selected_version_idx { "> * " } else { "  o " };
            let content = format!("{}{}", prefix, vt.label(&package.current_version));
            ListItem::new(content).style(if idx == app.selected_version_idx {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
        })
        .collect();

    let list =
        List::new(items).block(Block::default().borders(Borders::ALL).title("Select Version"));
    frame.render_widget(list, chunks[2]);

    let help = Paragraph::new("Up/Down: Navigate  Enter: Confirm  Esc: Back")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[3]);
}

pub fn render_custom_version(frame: &mut Frame, app: &App, package: &Package, input: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Package: "),
            Span::styled(
                &package.name,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Current Version: "),
            Span::styled(&package.current_version, Style::default().fg(Color::Yellow)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Package Info"));
    frame.render_widget(info, chunks[0]);

    let input_widget = Paragraph::new(input)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Enter Custom Version"));
    frame.render_widget(input_widget, chunks[1]);

    if let Some(error) = &app.error_message {
        let error_widget = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title("Error"));
        frame.render_widget(error_widget, chunks[2]);
    } else {
        let help = Paragraph::new("Enter: Confirm  Esc: Back")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[2]);
    }
}

pub fn render_publishing(frame: &mut Frame, app: &App, package: &Package, version: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    let title_text = if app.dry_run {
        format!("[DRY RUN] Publishing {}@{}", package.name, version)
    } else {
        format!("Publishing {}@{}", package.name, version)
    };
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Green))
        .percent((app.publish_progress * 100.0) as u16);
    frame.render_widget(gauge, chunks[1]);

    let status_lines: Vec<Line> =
        app.status_messages.iter().map(|msg| Line::from(msg.as_str())).collect();
    let status =
        Paragraph::new(status_lines).block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(status, chunks[2]);

    let help = Paragraph::new("Please wait... (Esc to cancel)")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[3]);
}

pub fn render_complete(frame: &mut Frame, released: &[String], dry_run: bool) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let success_text = if dry_run {
        "[DRY RUN] Would have released these packages:"
    } else {
        "All packages released successfully!"
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            success_text,
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for pkg in released {
        lines.push(Line::from(format!("  * {}", pkg)));
    }

    let message = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(message, chunks[0]);

    let help = Paragraph::new("Press Enter or Esc to exit")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[1]);
}
