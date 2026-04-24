use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub async fn init_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite://database.db")
        .await
        .expect("DB connection failed");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            gender TEXT,
            gender_probability REAL,
            age INTEGER,
            age_group TEXT,
            country_id TEXT,
            country_name TEXT,
            country_probability REAL,
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