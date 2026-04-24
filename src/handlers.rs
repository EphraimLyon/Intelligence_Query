use ax_utils::now; // Adjust based on your crate name
use axum::{
    extract::{State, Path, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;
use sqlx::{PgPool, Row};
use crate::models::{Profile, CreateProfile};
use crate::filters::Filters; // Ensure this is imported

const MAX_LIMIT: i64 = 100;
const DEFAULT_LIMIT: i64 = 10;

const VALID_SORT_COLUMNS: &[&str] = &[
    "age","name","country_name","gender",
    "created_at","gender_probability","country_probability",
];

// --- Helper for building SQL (Prevents duplication) ---
async fn execute_filtered_query(pool: &PgPool, filters: Filters) -> Result<serde_json::Value, sqlx::Error> {
    let limit = filters.limit.unwrap_or(DEFAULT_LIMIT).max(1).min(MAX_LIMIT);
    let page = filters.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let mut conditions: Vec<String> = vec![];
    let mut bindings: Vec<serde_json::Value> = vec![];

    if let Some(g) = &filters.gender {
        conditions.push(format!("LOWER(gender) = ${}", bindings.len() + 1));
        bindings.push(json!(g.to_lowercase()));
    }
    if let Some(c) = &filters.country_id {
        conditions.push(format!("UPPER(country_id) = ${}", bindings.len() + 1));
        bindings.push(json!(c.to_uppercase()));
    }
    if let Some(min) = filters.min_age {
        conditions.push(format!("age >= ${}", bindings.len() + 1));
        bindings.push(json!(min));
    }
    if let Some(max) = filters.max_age {
        conditions.push(format!("age <= ${}", bindings.len() + 1));
        bindings.push(json!(max));
    }
    if let Some(min_gp) = filters.min_gender_probability {
        conditions.push(format!("gender_probability >= ${}", bindings.len() + 1));
        bindings.push(json!(min_gp));
    }
    if let Some(min_cp) = filters.min_country_probability {
        conditions.push(format!("country_probability >= ${}", bindings.len() + 1));
        bindings.push(json!(min_cp));
    }

    let where_clause = if conditions.is_empty() { String::new() } else { format!("WHERE {}", conditions.join(" AND ")) };

    // Total Count
    let count_query = format!("SELECT COUNT(*) FROM profiles {}", where_clause);
    let mut count_exec = sqlx::query_scalar::<_, i64>(&count_query);
    for b in &bindings { count_exec = count_exec.bind(b); }
    let total = count_exec.fetch_one(pool).await.unwrap_or(0);

    // Data Query
    let sort_col = filters.sort_by.unwrap_or_else(|| "created_at".to_string());
    let sort_order = if filters.order.as_deref() == Some("asc") { "ASC" } else { "DESC" };
    
    let data_query = format!(
        "SELECT * FROM profiles {} ORDER BY {} {} LIMIT ${} OFFSET ${}",
        where_clause, sort_col, sort_order, bindings.len() + 1, bindings.len() + 2
    );

    let mut data_exec = sqlx::query_as::<_, Profile>(&data_query);
    for b in &bindings { data_exec = data_exec.bind(b); }
    let data = data_exec.bind(limit).bind(offset).fetch_all(pool).await?;

    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    Ok(json!({
        "status": "success",
        "data": data,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "total_pages": total_pages
        }
    }))
}

pub async fn get_profiles(State(pool): State<PgPool>, Query(filters): Query<Filters>) -> impl IntoResponse {
    // Validate sort column
if let Some(ref col) = filters.sort_by {
        if !VALID_SORT_COLUMNS.iter().any(|&s| s == col) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"status":"error","message":"Invalid sort_by value"}))
            ).into_response();
        }
    }

    match execute_filtered_query(&pool, filters).await {
        Ok(res) => Json(res).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status":"error","message": e.to_string()}))).into_response(),
    }
}

pub async fn natural_language_query(State(pool): State<PgPool>, Query(params): Query<crate::handlers::NlpQuery>) -> impl IntoResponse {
    let q = params.q.clone().unwrap_or_default();
    
    // Use the logic from nlp.rs
    match crate::nlp::parse(&q) {
        Some(mut filters) => {
            // Apply pagination from URL params to the parsed filters
            filters.page = params.page;
            filters.limit = params.limit;
            
            match execute_filtered_query(&pool, filters).await {
                Ok(res) => Json(res).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status":"error","message": e.to_string()}))).into_response(),
            }
        },
        None => (StatusCode::BAD_REQUEST, Json(json!({"status":"error","message":"Could not interpret query"}))).into_response(),
    }
}

pub async fn create_profile(State(pool): State<PgPool>, Json(payload): Json<CreateProfile>) -> impl IntoResponse {
    let id = Uuid::now_v7().to_string();
    let res = sqlx::query("INSERT INTO profiles (id, name, gender, gender_probability, age, age_group, country_id, country_name, country_probability, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
        .bind(&id).bind(&payload.name).bind(&payload.gender).bind(payload.gender_probability).bind(payload.age).bind(&payload.age_group).bind(&payload.country_id).bind(&payload.country_name).bind(payload.country_probability).bind(crate::utils::now())
        .execute(&pool).await;

    match res {
        Ok(_) => (StatusCode::CREATED, Json(json!({ "status": "created", "id": id }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "status": "error", "message": e.to_string() }))).into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct NlpQuery {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}