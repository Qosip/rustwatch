use anyhow::Result;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Website {
    pub url: String,
    pub name: String,
    #[serde(default)]
    pub last_status: String,
    #[serde(skip, default)]
    pub history: Vec<u64>,
}

// Fonction asynchrone qui vérifie un site : on renvoie le Message (String) et la Latence (u64)
pub async fn check_website(url: &str) -> Result<(String, u64)> {

    // On crée un client avec un timeout de 5 secondes
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()?;

    let start = Instant::now();

    // On lance la requête
    let response = client.get(url).send().await?;
    let duration = start.elapsed();
    let latency_ms = duration.as_millis() as u64;

    // On vérifie le code (200, 404, etc.)
    if response.status().is_success() {
        Ok((
            format!("SUCCÈS : {} ({} ms)", response.status(), latency_ms),
            latency_ms
        ))
    } else {
        Ok((
            format!("ATTENTION : Code {}", response.status()),
            0
        ))
    }
}