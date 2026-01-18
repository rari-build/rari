use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

use crate::state::{AppState, BuildStatus, Phase};

pub fn render_ui(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let title = Paragraph::new("Rari Binary Builder")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(title, chunks[0]);

    let progress = state.overall_progress();
    let elapsed = state.start_time.elapsed().as_secs();
    let elapsed_text = if elapsed >= 60 {
        let minutes = elapsed / 60;
        let seconds = elapsed % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", elapsed)
    };

    let phase_text = match state.phase {
        Phase::CheckingRust => "Checking Rust installation",
        Phase::InstallingTargets => "Installing targets",
        Phase::Building => "Building binaries",
        Phase::Complete => "Complete",
    };

    let progress_label = format!(
        "{} | {}/{} builds | {} elapsed",
        phase_text,
        state.success_count + state.failure_count,
        state.builds.len(),
        elapsed_text
    );

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Overall Progress"))
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
        .percent((progress * 100.0) as u16)
        .label(progress_label);
    frame.render_widget(gauge, chunks[1]);

    let build_items: Vec<ListItem> = state
        .builds
        .iter()
        .map(|build| {
            let (icon, color, status_text) = match &build.status {
                BuildStatus::Pending => ("WAIT", Color::Gray, "Pending".to_string()),
                BuildStatus::Installing => ("INST", Color::Yellow, "Installing target".to_string()),
                BuildStatus::Building => {
                    let progress_text = if let Some(total) = build.total_crates {
                        format!("Building ({}/{})", build.compiled_crates, total)
                    } else if build.compiled_crates > 0 {
                        format!("Building ({})", build.compiled_crates)
                    } else {
                        "Building".to_string()
                    };
                    ("BILD", Color::Blue, progress_text)
                }
                BuildStatus::Copying => ("COPY", Color::Cyan, "Copying".to_string()),
                BuildStatus::Validating => ("CHCK", Color::Magenta, "Validating".to_string()),
                BuildStatus::Success => {
                    let size = build.size_mb.map(|s| format!(" ({:.2} MB)", s)).unwrap_or_default();
                    ("DONE", Color::Green, format!("Success{}", size))
                }
                BuildStatus::Failed(err) => ("FAIL", Color::Red, format!("Failed: {}", err)),
            };

            let line = Line::from(vec![
                Span::raw(format!("[{}] ", icon)),
                Span::styled(
                    format!("{:15}", build.target.platform),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" -> "),
                Span::styled(status_text, Style::default().fg(color)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let builds_list =
        List::new(build_items).block(Block::default().borders(Borders::ALL).title("Build Status"));
    frame.render_widget(builds_list, chunks[2]);

    let log_items: Vec<ListItem> =
        state.logs.iter().rev().take(6).rev().map(|log| ListItem::new(log.as_str())).collect();

    let logs = List::new(log_items)
        .block(Block::default().borders(Borders::ALL).title("Activity Log"))
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(logs, chunks[3]);

    let help_text = if state.phase == Phase::Complete {
        "Press any key to exit"
    } else {
        "Press 'q' or ESC to cancel"
    };
    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[4]);
}
