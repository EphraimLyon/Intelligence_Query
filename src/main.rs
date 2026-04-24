mod db;
mod models;
mod handlers;
mod seed;
mod utils;

use axum::{
    Router,
    routing::{get, post},
};
use db::init_db;
use handlers::*;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = init_db(&database_url).await;

    seed::seed_db(&pool).await;

    let app = Router::new()
        // list / create
        .route("/api/profiles", get(get_profiles).post(create_profile))
        // âš  static routes MUST come before parameterised ones
        .route("/api/profiles/search", get(search_profiles))
        .route("/api/profiles/query",  get(natural_language_query))
        // parameterised routes
        .route("/api/profiles/{id}", get(get_profile).delete(delete_profile))
        .with_state(pool)
        .layer(CorsLayer::permissive());

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    println!("ðŸš€ Server running on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
