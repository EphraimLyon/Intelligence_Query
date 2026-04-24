use serde::Deserialize;
use sqlx::PgPool;
use std::fs;
use uuid::Uuid;

#[derive(Deserialize)]
struct SeedData {
    profiles: Vec<SeedProfile>,
}

#[derive(Deserialize)]
struct SeedProfile {
    name: String,
    gender: String,
    gender_probability: f64,
    age: i32,
    age_group: String,
    country_id: String,
    country_name: String,
    country_probability: f64,
}

pub async fn seed_db(pool: &PgPool) {
    let data = fs::read_to_string("seed.json")
        .expect("seed.json missing");

    let parsed: SeedData = serde_json::from_str(&data).unwrap();

    for p in parsed.profiles {
        let id = Uuid::now_v7().to_string();
        let _ = sqlx::query(
            r#"
            INSERT INTO profiles
            (id, name, gender, gender_probability, age, age_group, country_id, country_name, country_probability, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (name, age, country_id) DO NOTHING
            "#
        )
        .bind(id)
        .bind(p.name)
        .bind(p.gender)
        .bind(p.gender_probability)
        .bind(p.age)
        .bind(p.age_group)
        .bind(p.country_id)
        .bind(p.country_name)
        .bind(p.country_probability)
        .bind(Uuid::now_v7().to_string())
        .execute(pool)
        .await;
    }

    println!("✅ Seeding complete");
}