mod monitor;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use std::io;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem},
    layout::{Layout, Constraint, Direction},
    style::{Style, Color},
    Terminal,
};

use monitor::Website;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. CRÉATION DES DONNÉES
    let websites = vec![
        Website { name: "Google".to_string(), url: "https://www.google.com".to_string(), last_status: "En attente...".to_string() },
        Website { name: "GitHub".to_string(), url: "https://github.com".to_string(), last_status: "En attente...".to_string() },
        Website { name: "Localhost".to_string(), url: "http://localhost:8080".to_string(), last_status: "En attente...".to_string() }, // Test error
    ];

    // Liste dans un Arc<Mutex<>> partageable
    let app_state = Arc::new(Mutex::new(websites));

    // 2. LANCEMENT DU "WORKER"
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        loop {
            let mut sites = app_state_clone.lock().await;

            // Lancement d'une vérification
            for site in sites.iter_mut() {
                match monitor::check_website(&site.url).await {
                    Ok(msg) => site.last_status = msg,
                    Err(e) => site.last_status = format!("ERREUR : {}", e),
                }
            }
            // On relâche le verrou
            drop(sites);

            // Stop 5s
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // 3. LANCEMENT DE L'INTERFACE
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        // A. On récupère les données
        let current_sites = app_state.lock().await;

        // On transforme sites en "ListItem" + couleur
        let items: Vec<ListItem> = current_sites
            .iter()
            .map(|site| {
                // 1. On décide de la couleur
                let style = if site.last_status.contains("SUCCÈS") {
                    Style::default().fg(Color::Green)
                } else if site.last_status.contains("En attente") {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Red)
                };

                // 2. On crée l'élément et on lui applique le style
                ListItem::new(format!("{} -> {}", site.name, site.last_status))
                    .style(style)
            })
            .collect();

        // B. On draw
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.size());

            let list = List::new(items)
                .block(Block::default().title(" Monitoring RustWatch ").borders(Borders::ALL));

            f.render_widget(list, chunks[0]);
        })?;

        // On relâche le verrou
        drop(current_sites);

        // C. Gestion du clavier
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
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