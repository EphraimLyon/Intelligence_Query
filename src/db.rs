use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub async fn init_db(database_url: &str) -> PgPool {
    // 1. Optimized Pool Settings
    let pool = PgPoolOptions::new()
        .max_connections(20) // Increased from 5 to handle concurrent test load
        .acquire_timeout(Duration::from_secs(3)) // Fail fast if DB is jammed
        .idle_timeout(Duration::from_secs(60))
        .connect(database_url)
        .await
        .expect("DB connection failed");

    // 2. Schema with Performance Indexes
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            gender TEXT,
            gender_probability FLOAT8,
            age INTEGER,
            age_group TEXT,
            country_id TEXT,
            country_name TEXT,
            country_probability FLOAT8,
            created_at TEXT,
            UNIQUE(name, age, country_id)
        );

        -- Performance Indexes to prevent timeouts during complex filtering
        CREATE INDEX IF NOT EXISTS idx_profiles_gender ON profiles(gender);
        CREATE INDEX IF NOT EXISTS idx_profiles_country ON profiles(country_id);
        CREATE INDEX IF NOT EXISTS idx_profiles_age ON profiles(age);
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to initialize database schema");

    pool
}