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
    let pool = init_db().await;

    seed::seed_db(&pool).await;

    let app = Router::new()
        .route("/api/profiles", get(get_profiles).post(create_profile))
        .route("/api/profiles/{id}", get(get_profile).delete(delete_profile))
        .route("/api/profiles/search", get(search_profiles))
        .with_state(pool)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("🚀 Server running on port 3000");

    axum::serve(listener, app).await.unwrap();
}