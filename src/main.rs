mod db;
mod models;
mod handlers;
mod seed;
mod utils;
mod services; // Ensure this is registered
mod nlp;      // Ensure this is registered

use axum::{
    Router,
    routing::{get, post, delete}, // Added delete for completeness
};
use db::init_db;
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // 1. Load environment variables
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // 2. Initialize database with the improved pool settings from our db.rs fix
    let pool = init_db(&database_url).await;

    // 3. Run seed logic
    seed::seed_db(&pool).await;

    // 4. Build the application
    let app = Router::new()
        // Standard collection routes
        .route("/api/profiles", get(handlers::get_profiles).post(handlers::create_profile))
        
        // Static specific search routes (MUST be above parameterized routes)
        .route("/api/profiles/search", get(handlers::get_profiles)) // Using get_profiles for search
        .route("/api/profiles/query", get(handlers::natural_language_query))
        
        // Parameterized routes - FIX: Use :id instead of {id} for Axum
        .route("/api/profiles/:id", get(handlers::get_profile).delete(handlers::delete_profile))
        
        // State and Middleware
        .with_state(pool)
        .layer(CorsLayer::permissive());

    // 5. Config server address
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port)
        .parse()
        .expect("Invalid address format");

    println!("🚀 Server running on {}", addr);

    // 6. Launch
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}