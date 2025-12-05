use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
struct Stats30d {
    #[serde(rename = "sandwichRate")]
    sandwich_rate: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Validator {
    #[serde(rename = "identityAccount")]
    identity_account: String,
    stats30d: Option<Stats30d>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ApiResponse {
    ok: bool,
    data: Vec<Validator>,
}

fn main() -> Result<()> {
    println!("MEV Blocklist Generator");
    println!("=======================\n");

    let api_url = "https://nextgen.mev-hub.snowgenesis.com/api/validators";

    println!("Fetching validator data from {}", api_url);

    // Create HTTP client
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .build()
        .context("Failed to create HTTP client")?;

    // Fetch validator data
    let response = client
        .get(api_url)
        .send()
        .context("Failed to fetch validator data")?;

    if !response.status().is_success() {
        anyhow::bail!("API request failed with status: {}", response.status());
    }

    let api_response: ApiResponse = response
        .json()
        .context("Failed to parse JSON response")?;

    if !api_response.ok {
        anyhow::bail!("API returned error response");
    }

    let validators = api_response.data;
    println!("Total validators fetched: {}\n", validators.len());

    // Filter validators with sandwich rate >= 1.0
    let threshold = 1.0;
    let mut blocklist: Vec<String> = validators
        .into_iter()
        .filter_map(|v| {
            if let Some(stats) = &v.stats30d {
                if stats.sandwich_rate >= threshold {
                    println!(
                        "Blocklisting: {} (sandwich rate: {:.2}%)",
                        v.identity_account, stats.sandwich_rate
                    );
                    return Some(v.identity_account);
                }
            }
            None
        })
        .collect();

    // Sort for consistency
    blocklist.sort();

    println!("\n{} validators meet the criteria (sandwich rate >= {}%)",
             blocklist.len(), threshold);

    // Write to blocklist.txt
    let output_path = "blocklist.txt";
    let content = blocklist.join("\n");

    fs::write(output_path, content)
        .context("Failed to write blocklist.txt")?;

    println!("\nBlocklist saved to: {}", output_path);
    println!("Total addresses in blocklist: {}", blocklist.len());

    Ok(())
}
