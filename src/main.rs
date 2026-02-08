mod monitor;
mod ui;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use std::io;
use std::fs;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use monitor::Website;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Chargement de websites.json...");
    let file_content = fs::read_to_string("websites.json")
        .expect("ERREUR: Impossible de lire websites.json");

    let mut loaded_websites: Vec<Website> = serde_json::from_str(&file_content)
        .expect("ERREUR: Le format JSON est incorrect !");

    for site in &mut loaded_websites {
        site.last_status = "En attente...".to_string();
    }

    let app_state = Arc::new(Mutex::new(loaded_websites));

    // --- LANCEMENT DU WORKER ---
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        loop {
            let targets: Vec<(usize, String)> = {
                let sites = app_state_clone.lock().await;
                sites.iter().enumerate().map(|(i, s)| (i, s.url.clone())).collect()
            };

            for (index, url) in targets {
                let result = monitor::check_website(&url).await;

                let mut sites = app_state_clone.lock().await;
                if let Some(site) = sites.get_mut(index) {
                    match result {
                        Ok((msg, latency)) => {
                            site.last_status = msg;
                            site.history.push(latency);
                            if site.history.len() > 50 { site.history.remove(0); }
                        }
                        Err(e) => {
                            site.last_status = format!("ERREUR : {}", e);
                            site.history.push(0);
                            if site.history.len() > 50 { site.history.remove(0); }
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });


    // --- CONFIGURATION DE L'INTERFACE ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut selected_index = 0;


    // --- BOUCLE PRINCIPALE ---
    loop {
        let current_sites = app_state.lock().await;

        terminal.draw(|f| {
            ui::draw(f, &current_sites, selected_index);
        })?;

        drop(current_sites);

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,

                        KeyCode::Down => {
                            let current_sites = app_state.lock().await;
                            if selected_index < current_sites.len() - 1 {
                                selected_index += 1;
                            } else {
                                selected_index = 0;
                            }
                        }

                        KeyCode::Up => {
                            if selected_index > 0 {
                                selected_index -= 1;
                            } else {
                                let current_sites = app_state.lock().await;
                                selected_index = current_sites.len() - 1;
                            }
                        }

                        _ => {}
                    }
                }
    }

    // --- NETTOYAGE ---
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}