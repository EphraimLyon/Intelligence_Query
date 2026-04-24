use axum::{
    extract::{State, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use sqlx::{PgPool, Row};

use crate::models::{Profile, CreateProfile};
use crate::filters::Filters;
use crate::utils::now;

const MAX_LIMIT: i64 = 100;
const DEFAULT_LIMIT: i64 = 10;

const VALID_SORT_COLUMNS: &[&str] = &[
    "age", "name", "country_name", "gender",
    "created_at", "gender_probability", "country_probability",
];

#[derive(Debug, Deserialize, Default)]
pub struct NlpQuery {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

// ================= REUSABLE QUERY ENGINE =================
// This ensures both standard search and NLP return the same data format.
async fn execute_filtered_query(pool: &PgPool, filters: Filters) -> Result<serde_json::Value, sqlx::Error> {
    // Strict Pagination Clamping
    let limit = filters.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
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

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // --- 1. Total Count for Pagination Envelope ---
    let count_query = format!("SELECT COUNT(*) FROM profiles {}", where_clause);
    let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query);
    for b in &bindings {
        count_builder = count_builder.bind(b);
    }
    let total = count_builder.fetch_one(pool).await.unwrap_or(0);

    // --- 2. Data Retrieval ---
    let sort_col = filters.sort_by.unwrap_or_else(|| "created_at".to_string());
    let sort_order = if filters.order.as_deref().unwrap_or("desc").to_lowercase() == "asc" { "ASC" } else { "DESC" };

    let data_query = format!(
        "SELECT id, name, gender, gender_probability, age, age_group, country_id, country_name, country_probability, created_at 
         FROM profiles {} ORDER BY {} {} LIMIT ${} OFFSET ${}",
        where_clause, sort_col, sort_order, bindings.len() + 1, bindings.len() + 2
    );

    let mut data_builder = sqlx::query_as::<_, Profile>(&data_query);
    for b in &bindings {
        data_builder = data_builder.bind(b);
    }
    
    let rows = data_builder.bind(limit).bind(offset).fetch_all(pool).await?;
    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    Ok(json!({
        "status": "success",
        "data": rows,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "total_pages": total_pages
        }
    }))
}

// ================= HANDLERS =================

pub async fn get_profiles(
    State(pool): State<PgPool>,
    Query(filters): Query<Filters>,
) -> impl IntoResponse {
    // Validate sort column to prevent SQL injection and pass validation tests
    if let Some(ref col) = filters.sort_by {
        if !VALID_SORT_COLUMNS.iter().any(|&s| s == col) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"status": "error", "message": "Invalid sort_by value"})),
            ).into_response();
        }
    }

    match execute_filtered_query(&pool, filters).await {
        Ok(res) => Json(res).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"status": "error", "message": e.to_string()})),
        ).into_response(),
    }
}

pub async fn natural_language_query(
    State(pool): State<PgPool>,
    Query(params): Query<NlpQuery>,
) -> impl IntoResponse {
    let q_text = params.q.clone().unwrap_or_default();

    match crate::nlp::parse(&q_text) {
        Some(mut filters) => {
            // Apply pagination from URL params over the NLP-parsed filters
            filters.page = params.page;
            filters.limit = params.limit;

            match execute_filtered_query(&pool, filters).await {
                Ok(res) => Json(res).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"status": "error", "message": e.to_string()})),
                ).into_response(),
            }
        }
        None => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error", 
                "message": "Could not interpret query"
            })),
        ).into_response(),
    }
}

pub async fn create_profile(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateProfile>,
) -> impl IntoResponse {
    let id = Uuid::now_v7().to_string();

    let res = sqlx::query(
        r#"
        INSERT INTO profiles
        (id, name, gender, gender_probability, age, age_group, country_id, country_name, country_probability, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
    .bind(&id)
    .bind(&payload.name)
    .bind(&payload.gender)
    .bind(payload.gender_probability)
    .bind(payload.age)
    .bind(&payload.age_group)
    .bind(&payload.country_id)
    .bind(&payload.country_name)
    .bind(payload.country_probability)
    .bind(now())
    .execute(&pool)
    .await;

    match res {
        Ok(_) => (StatusCode::CREATED, Json(json!({ "status": "created", "id": id }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "status": "error", "message": e.to_string() }))
        ).into_response(),
    }
}