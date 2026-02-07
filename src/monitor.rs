use reqwest;
use anyhow::Result;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Website {
    pub url: String,
    pub name: String,
    #[serde(default)]
    pub last_status: String,
}

// Fonction asynchrone qui vérifie un site
pub async fn check_website(url: &str) -> Result<String> {

    // On crée un client avec un timeout de 5 secondes
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    // On lance la requête
    let response = client.get(url).send().await?;

    // On vérifie le code (200, 404, etc.)
    if response.status().is_success() {
        Ok(format!("SUCCÈS : Le site {} répond en 200 OK", url))
    } else {
        Ok(format!("ATTENTION : Le site {} a renvoyé le code {}", url, response.status()))
    }
}