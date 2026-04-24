use serde::Deserialize;
use anyhow::Result;

// The value "gender, probability  and count" needed derived from Genderize External API.
#[derive(Deserialize)]
pub struct Genderize {
    pub gender: Option<String>,
    pub probability: f64,
    pub count: i64,
}

// The value "age" needed from Agify API.
#[derive(Deserialize)]
pub struct Agify {
    pub age: Option<i32>,
}

// The value "country array" needed from Nationalize External API. 
#[derive(Deserialize)]
pub struct Nationalize {
    pub country: Vec<Country>,
}

// The value "country_id and probability" needed from country array
#[derive(Deserialize)]
pub struct Country {
    pub country_id: String,
    pub probability: f64,
}

pub async fn fetch_all(name: &str) -> Result<(Genderize, Agify, Country)> {
    let gender: Genderize = reqwest::get(format!("https://api.genderize.io?name={}", name))
        .await?
        .json()
        .await?;

    if gender.gender.is_none() || gender.count == 0 {
        anyhow::bail!("Genderize");
    }

    let age: Agify = reqwest::get(format!("https://api.agify.io?name={}", name))
        .await?
        .json()
        .await?;

    if age.age.is_none() {
        anyhow::bail!("Agify");
    }

    let nat: Nationalize = reqwest::get(format!("https://api.nationalize.io?name={}", name))
        .await?
        .json()
        .await?;

    let country = nat
        .country
        .into_iter()
        .max_by(|a, b| a.probability.partial_cmp(&b.probability).unwrap());

    if country.is_none() {
        anyhow::bail!("Nationalize");
    }

    Ok((gender, age, country.unwrap()))
}

pub fn age_group(age: i32) -> String {
    match age {
        0..=12 => "child",
        13..=19 => "teenager",
        20..=59 => "adult",
        _ => "senior",
    }
    .to_string()
}