use serde::Deserialize;
use anyhow::{Result, Context};
use futures::try_join; // You'll need the 'futures' crate in Cargo.toml

#[derive(Deserialize, Debug)]
pub struct Genderize {
    pub gender: Option<String>,
    pub probability: f64,
    pub count: i64,
}

#[derive(Deserialize, Debug)]
pub struct Agify {
    pub age: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct Nationalize {
    pub country: Vec<Country>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Country {
    pub country_id: String,
    pub probability: f64,
}

pub async fn fetch_all(name: &str) -> Result<(Genderize, Agify, Country)> {
    let client = reqwest::Client::new();
    let gender_url = format!("https://api.genderize.io?name={}", name);
    let age_url = format!("https://api.agify.io?name={}", name);
    let nat_url = format!("https://api.nationalize.io?name={}", name);

    // FIX 1: Run all 3 requests in PARALLEL using try_join!
    // This reduces latency from (A+B+C) to just the slowest single request.
    let (gender_res, age_res, nat_res) = try_join!(
        client.get(&gender_url).send(),
        client.get(&age_url).send(),
        client.get(&nat_url).send()
    ).context("Failed to contact external APIs")?;

    // Deserialize them
    let gender: Genderize = gender_res.json().await?;
    let age: Agify = age_res.json().await?;
    let nat: Nationalize = nat_res.json().await?;

    // FIX 2: Better Validation Logic
    if gender.gender.is_none() || gender.count == 0 {
        anyhow::bail!("Could not determine gender for the provided name");
    }

    let age_val = age.age.context("Agify returned no age")?;

    let best_country = nat
        .country
        .into_iter()
        .max_by(|a, b| a.probability.partial_cmp(&b.probability).unwrap_or(std::cmp::Ordering::Equal))
        .context("Nationalize returned no countries")?;

    Ok((gender, age, best_country))
}

pub fn age_group(age: i32) -> String {
    // Standardizing groups to match test expectations
    match age {
        0..=12 => "child",
        13..=19 => "teenager", // Matches your NLP logic
        20..=64 => "adult",
        _ => "senior",
    }
    .to_string()
}