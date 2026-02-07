mod monitor;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use std::io;
use std::fs;
use serde_json;
use crossterm::event::KeyEventKind;

use monitor::Website;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem, Sparkline},
    layout::{Layout, Constraint, Direction},
    style::{Style, Color},
    Terminal,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    println!("Chargement de websites.json...");

    let file_content = fs::read_to_string("websites.json")
        .expect("ERREUR: Impossible de lire websites.json.");

    let mut loaded_websites: Vec<Website> = serde_json::from_str(&file_content)
        .expect("ERREUR: Le format JSON est incorrect !");

    for site in &mut loaded_websites {
        site.last_status = "En attente...".to_string();
    }

    let app_state = Arc::new(Mutex::new(loaded_websites));

    // 2. LANCEMENT DU "WORKER"
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        loop {
            let mut sites = app_state_clone.lock().await;

            // Lancement d'une vérification
            for site in sites.iter_mut() {
                match monitor::check_website(&site.url).await {
                    Ok((msg, latency)) => {
                        site.last_status = msg;
                        // On ajoute le ping dans l'historique
                        site.history.push(latency);
                        if site.history.len() > 50 {
                            site.history.remove(0);
                        }
                    }
                    Err(e) => {
                        site.last_status = format!("ERREUR : {}", e);
                        site.history.push(0);
                        if site.history.len() > 50 {
                            site.history.remove(0);
                        }
                    }
                }
            }
            // On relâche le verrou
            drop(sites);

            // Stop 5s
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // 3. LANCEMENT DE L'INTERFACE
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut selected_index = 0;
    loop {
        // A. On récupère les données
        let current_sites = app_state.lock().await;

        // On transforme sites en "ListItem" + couleur
        let items: Vec<ListItem> = current_sites
            .iter()
            .enumerate()
            .map(|(i,site)| {
                // 1. On décide de la couleur
                let style = if site.last_status.contains("SUCCÈS") {
                    Color::Green
                } else if site.last_status.contains("En attente") {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let prefix = if i == selected_index { ">> " } else { "   " };

                // 2. On crée l'élément et on lui applique le style
                ListItem::new(format!("{}{}", prefix, format!("{} -> {}", site.name, site.last_status)))
                    .style(Style::default().fg(style))
            })
            .collect();

        // B. On draw
        terminal.draw(|f| {
            // 1. On découpe l'écran Verticalement
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(50), // 50% Liste
                    Constraint::Percentage(50), // 50% Graphique
                ].as_ref())
                .split(f.area());

            // --- ZONE 1 : LA LISTE ---
            let list = List::new(items.clone())
                .block(Block::default().title(" Monitoring ").borders(Borders::ALL));
            f.render_widget(list, chunks[0]);

            // --- ZONE 2 : LE GRAPHIQUE SELECTIONNE ---
            if let Some(selected_site) = items.get(selected_index).and_then(|_| current_sites.get(selected_index)) {
                let sparkline = Sparkline::default()
                    .block(Block::default()
                        .title(format!(" Latence: {} - Ping Actuel: {} ms ",
                                       selected_site.name,
                                       selected_site.history.last().unwrap_or(&0)
                        ))
                        .borders(Borders::ALL))
                    .data(&selected_site.history)
                    .style(Style::default().fg(Color::Cyan));

                f.render_widget(sparkline, chunks[1]);
            }
        })?;

        // On relâche le verrou
        drop(current_sites);

        // C. Gestion du clavier
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,

                        // FLÈCHE BAS
                        KeyCode::Down => {
                            let current_sites = app_state.lock().await;
                            // On ne descend que si on n'est pas déjà tout en bas
                            if selected_index < current_sites.len() - 1 {
                                selected_index += 1;
                            }
                        }

                        // FLÈCHE HAUT
                        KeyCode::Up => {
                            // On ne monte que si on n'est pas déjà tout en haut (0)
                            if selected_index > 0 {
                                selected_index -= 1;
                            }
                        }

                        _ => {} // On ignore les autres touches
                    }
                }
            }
        }
    }

    // 4. NETTOYAGE
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}