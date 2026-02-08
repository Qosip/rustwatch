use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Sparkline},
    Frame,
};

use crate::monitor::Website;

// Fonction principale de dessin
pub fn draw(f: &mut Frame, websites: &[Website], selected_index: usize) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ].as_ref())
        .split(f.area());

    let items: Vec<ListItem> = websites
        .iter()
        .enumerate()
        .map(|(i, site)| {
            // Logique des couleurs
            let color = if site.last_status.contains("SUCCÈS") {
                Color::Green
            } else if site.last_status.contains("En attente") {
                Color::Yellow
            } else {
                Color::Red
            };

            let prefix = if i == selected_index { ">> " } else { "   " };

            ListItem::new(format!("{}{} -> {}", prefix, site.name, site.last_status))
                .style(Style::default().fg(color))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(" Monitoring RustWatch ").borders(Borders::ALL));
    f.render_widget(list, chunks[0]);

    if let Some(selected_site) = websites.get(selected_index) {
        let current_ping = selected_site.history.last().unwrap_or(&0);

        let sparkline = Sparkline::default()
            .block(Block::default()
                .title(format!(" Latence: {} - Ping Actuel: {} ms ", selected_site.name, current_ping))
                .borders(Borders::ALL))
            .data(&selected_site.history)
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(sparkline, chunks[1]);
    }
}