use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub async fn init_db(database_url: &str) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("DB connection failed");

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
        )
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}