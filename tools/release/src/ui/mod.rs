use crate::app::App;
use crate::package::ReleaseUnit;
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
        .release_units
        .iter()
        .enumerate()
        .map(|(idx, unit)| {
            let prefix = if idx == app.selected_package_idx { "> " } else { "  " };
            let content = format!("{}{} (v{})", prefix, unit.name(), unit.current_version());
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

pub fn render_version_selection(frame: &mut Frame, app: &App, unit: &ReleaseUnit) {
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

    let mut info_lines = vec![
        Line::from(vec![
            Span::raw("Package: "),
            Span::styled(
                unit.name(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Current Version: "),
            Span::styled(unit.current_version(), Style::default().fg(Color::Yellow)),
        ]),
    ];

    if let ReleaseUnit::Group(group) = unit {
        info_lines.push(Line::from(vec![
            Span::raw("Packages: "),
            Span::styled(
                format!("{} binaries", group.packages.len()),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    let info = Paragraph::new(info_lines)
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
            let content = format!("{}{}", prefix, vt.label(unit.current_version()));
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

pub fn render_custom_version(frame: &mut Frame, app: &App, unit: &ReleaseUnit, input: &str) {
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
                unit.name(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Current Version: "),
            Span::styled(unit.current_version(), Style::default().fg(Color::Yellow)),
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

pub fn render_otp_input(frame: &mut Frame, app: &App, unit: &ReleaseUnit, input: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Package: "),
            Span::styled(
                unit.name(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Current Version: "),
            Span::styled(unit.current_version(), Style::default().fg(Color::Yellow)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Package Info"));
    frame.render_widget(info, chunks[0]);

    let otp_info = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "npm requires a one-time password (OTP) for publishing.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("Please enter the 6-digit code from your authenticator app."),
    ])
    .block(Block::default().borders(Borders::ALL).title("OTP Required"));
    frame.render_widget(otp_info, chunks[1]);

    let masked = "*".repeat(input.len());
    let underscores = if input.len() < 6 { "_".repeat(6 - input.len()) } else { String::new() };
    let display = format!("{}{}", masked, underscores);
    let input_widget = Paragraph::new(display)
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Enter OTP (6 digits)"));
    frame.render_widget(input_widget, chunks[2]);

    if let Some(error) = &app.error_message {
        let error_widget = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Error"));
        frame.render_widget(error_widget, chunks[3]);
    } else {
        let help = Paragraph::new("Enter 6 digits  Enter: Confirm  Esc: Back")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[3]);
    }
}

pub fn render_publishing(frame: &mut Frame, app: &App, unit: &ReleaseUnit, version: &str) {
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
        format!("[DRY RUN] Publishing {}@{}", unit.name(), version)
    } else {
        format!("Publishing {}@{}", unit.name(), version)
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

pub fn render_post_publish(frame: &mut Frame, app: &App, has_more_packages: bool) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(5), Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let title = Paragraph::new("Package Released Successfully!")
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let mut lines = vec![Line::from("")];

    for pkg in &app.released_packages {
        lines.push(Line::from(vec![
            Span::styled("  ✓ ", Style::default().fg(Color::Green)),
            Span::styled(&pkg.name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("@"),
            Span::styled(&pkg.version, Style::default().fg(Color::Yellow)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "What would you like to do next?",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if has_more_packages {
        lines.push(Line::from(vec![
            Span::styled("  C", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(": Continue releasing more packages"),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("  F", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(": Finish - Push tags and open GitHub releases"),
    ]));

    let message = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title("Next Steps"));
    frame.render_widget(message, chunks[1]);

    let help_text = if has_more_packages {
        "C: Continue  F/Enter: Finish  Esc/q: Quit"
    } else {
        "F/Enter: Finish  Esc/q: Quit"
    };

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}

pub fn render_post_release(
    frame: &mut Frame,
    app: &App,
    released: &[crate::package::ReleasedPackage],
    step: &crate::app::PostReleaseStep,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(5), Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let title_text =
        if app.dry_run { "[DRY RUN] Post-Release Actions" } else { "Post-Release Actions" };
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Release Complete"));
    frame.render_widget(title, chunks[0]);

    match step {
        crate::app::PostReleaseStep::Pushing | crate::app::PostReleaseStep::PushComplete => {
            let mut lines: Vec<Line> = vec![Line::from("")];
            for msg in &app.post_release_messages {
                lines.push(Line::from(msg.as_str()));
            }
            if *step == crate::app::PostReleaseStep::Pushing {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Please wait...",
                    Style::default().fg(Color::Yellow),
                )));
            }
            let message = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Pushing to Git"));
            frame.render_widget(message, chunks[1]);
        }
        crate::app::PostReleaseStep::PromptGitHub => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Create GitHub Releases?",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];

            for pkg in released {
                lines.push(Line::from(format!("  * {}@{}", pkg.name, pkg.version)));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Open GitHub release pages in browser?",
                Style::default().fg(Color::Yellow),
            )));

            let message = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("GitHub Releases"));
            frame.render_widget(message, chunks[1]);

            let help = Paragraph::new("Y: Open in browser  N/Enter: Skip")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(help, chunks[2]);
        }
        crate::app::PostReleaseStep::OpeningGitHub | crate::app::PostReleaseStep::Done => {
            let mut lines: Vec<Line> = vec![Line::from("")];
            for msg in &app.post_release_messages {
                lines.push(Line::from(msg.as_str()));
            }
            let message = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("GitHub Releases"));
            frame.render_widget(message, chunks[1]);

            let help = if *step == crate::app::PostReleaseStep::Done {
                Paragraph::new("Press Enter to continue")
            } else {
                Paragraph::new("Please wait...")
            }
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(help, chunks[2]);
        }
    }

    if *step != crate::app::PostReleaseStep::PromptGitHub {
        let help = Paragraph::new("")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[2]);
    }
}

pub fn render_complete(
    frame: &mut Frame,
    released: &[crate::package::ReleasedPackage],
    dry_run: bool,
) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let success_text = if dry_run {
        "[DRY RUN] Would have released these packages:"
    } else {
        "✨ All packages released successfully! ✨"
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
        lines.push(Line::from(format!("  * {}@{}", pkg.name, pkg.version)));
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
