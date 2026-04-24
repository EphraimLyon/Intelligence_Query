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