use sqlx::PgPool;
use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize)]
struct SeedProfile {
    pub name: String,
    pub gender: String,
    pub gender_probability: f64,
    pub age: i32,
    pub age_group: String,
    pub country_id: String,
    pub country_name: String,
    pub country_probability: f64,
}

#[derive(Deserialize)]
struct SeedData {
    pub profiles: Vec<SeedProfile>,
}

pub async fn seed_db(pool: &PgPool) {
    if let Ok(data) = std::fs::read_to_string("seed.json") {
        if let Ok(parsed) = serde_json::from_str::<SeedData>(&data) {
            for p in parsed.profiles {
                let _ = sqlx::query(
                    r#"
                    INSERT INTO profiles
                    (id,name,gender,gender_probability,age,age_group,country_id,country_name,country_probability,created_at)
                    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                    ON CONFLICT DO NOTHING
                    "#
                )
                .bind(Uuid::now_v7().to_string())
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
        }
    }

    println!("Seeding safe");
}