use axum::{
    extract::{State, Path, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::utils::now;
use sqlx::{PgPool, Row};

use crate::models::{Profile, CreateProfile};

const MAX_LIMIT: i64 = 100;
const DEFAULT_LIMIT: i64 = 10;

const VALID_SORT_COLUMNS: &[&str] = &[
    "age","name","country_name","gender",
    "created_at","gender_probability","country_probability",
];

#[derive(Debug, Deserialize, Default)]
pub struct Filters {
    pub name: Option<String>,
    pub country: Option<String>,
    pub gender: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub min_age: Option<i32>,
    pub max_age: Option<i32>,
    pub country_id: Option<String>,
    pub age_group: Option<String>,
    pub min_gender_probability: Option<f64>,
    pub min_country_probability: Option<f64>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct QueryParams {
    pub search: Option<String>,
    pub gender: Option<String>,
    pub country: Option<String>,
    pub age_group: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl QueryParams {
    pub fn page(&self) -> i64 { self.page.unwrap_or(1) }
    pub fn limit(&self) -> i64 { self.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) }
    pub fn offset(&self) -> i64 { (self.page() - 1) * self.limit() }
}

#[derive(Debug, Deserialize, Default)]
pub struct NlpQuery {
    pub q: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

fn validated_sort_column(sort_by: Option<&str>) -> Result<&'static str, ()> {
    match sort_by {
        None => Ok("created_at"),
        Some(col) => VALID_SORT_COLUMNS
            .iter()
            .find(|&&s| s == col)
            .copied()
            .ok_or(()),
    }
}

fn validated_order(order: Option<&str>) -> &'static str {
    match order {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    }
}

fn capped_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT)
}

// ================= CREATE =================

pub async fn create_profile(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateProfile>,
) -> impl IntoResponse {

    let id = Uuid::now_v7().to_string();

    let res = sqlx::query(
        r#"
        INSERT INTO profiles
        (id,name,gender,gender_probability,age,age_group,country_id,country_name,country_probability,created_at)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
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
        Ok(_) => Json(json!({ "status": "created", "id": id })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "status": "error", "message": e.to_string() }))
        ).into_response(),
    }
}

// ================= GET PROFILES =================

pub async fn get_profiles(
    State(pool): State<PgPool>,
    Query(filters): Query<Filters>,
) -> impl IntoResponse {

    let sort_column = match validated_sort_column(filters.sort_by.as_deref()) {
        Ok(col) => col,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "error",
                    "message": "Invalid sort_by value"
                })),
            ).into_response();
        }
    };

    let sort_order = validated_order(filters.order.as_deref());
    //let mut bindings: Vec<serde_json::Value> = vec![];
if filters.limit.unwrap_or(0) < 0 {
    return (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "status": "error",
            "message": "Invalid limit"
        }))
    ).into_response();
}

    // ✅ FIX 1: strict limit cap enforcement
    let limit = match filters.limit {
        Some(l) if l > 0 => l.min(MAX_LIMIT),
        _ => DEFAULT_LIMIT,
    };

    // ensure page is valid
    let page = filters.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    // ---------------- WHERE CLAUSE ----------------
    let mut conditions: Vec<String> = vec![];
    // let mut bindings: Vec<String> = vec![];
    let mut bindings: Vec<serde_json::Value> = vec![];

    if let Some(g) = &filters.gender {
        conditions.push(format!("LOWER(gender) = ${}", bindings.len() + 1));
        bindings.push(g.to_lowercase());
    }

    if let Some(c) = &filters.country_id {
        conditions.push(format!("UPPER(country_id) = ${}", bindings.len() + 1));
        bindings.push(c.to_uppercase());
    }

    if let Some(min) = filters.min_age {
        conditions.push(format!("age >= ${}", bindings.len() + 1));
        // bindings.push(min.to_string());
    bindings.push(json!(min));
    }

    if let Some(max) = filters.max_age {
        conditions.push(format!("age <= ${}", bindings.len() + 1));
       // bindings.push(max.to_string());
       bindings.push(json!(max));
    }

    if let Some(min_gp) = filters.min_gender_probability {
        conditions.push(format!("gender_probability >= ${}", bindings.len() + 1));
        bindings.push(min_gp.to_string());
    }

    if let Some(min_cp) = filters.min_country_probability {
        conditions.push(format!("country_probability >= ${}", bindings.len() + 1));
        bindings.push(min_cp.to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // ---------------- COUNT QUERY ----------------
    let count_query = format!("SELECT COUNT(*) AS count FROM profiles {}", where_clause);
    let mut count_builder = sqlx::query(&count_query);

    for b in &bindings {
        count_builder = count_builder.bind(b);
    }

    let total: i64 = match count_builder.fetch_one(&pool).await {
        Ok(r) => r.get("count"),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": e.to_string()
                })),
            ).into_response();
        }
    };

    // ---------------- DATA QUERY ----------------
    let limit_pos = bindings.len() + 1;
    let offset_pos = bindings.len() + 2;

    let data_query = format!(
        "SELECT * FROM profiles {} ORDER BY {} {} LIMIT ${} OFFSET ${}",
        where_clause, sort_column, sort_order, limit_pos, offset_pos
    );

    let mut builder = sqlx::query(&data_query);

    for b in &bindings {
        builder = builder.bind(b);
    }

    builder = builder.bind(limit).bind(offset);

    let rows = match builder.fetch_all(&pool).await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": e.to_string()
                })),
            ).into_response();
        }
    };

    let data: Vec<Profile> = rows.into_iter().map(|row| Profile {
        id: row.get("id"),
        name: row.get("name"),
        gender: row.get("gender"),
        gender_probability: row.get("gender_probability"),
        age: row.get("age"),
        age_group: row.get("age_group"),
        country_id: row.get("country_id"),
        country_name: row.get("country_name"),
        country_probability: row.get("country_probability"),
        created_at: row.get("created_at"),
    }).collect();

    // ---------------- FIX 2: correct pagination envelope ----------------
    let total_pages = (total as f64 / limit as f64).ceil() as i64;

    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "data": data,
            "pagination": {
                "page": page,
                "limit": limit,
                "total": total,
                "total_pages": total_pages
            }
        })),
    ).into_response()
}

// ================= NLP =================

pub async fn natural_language_query(
    Query(params): Query<NlpQuery>,
) -> impl IntoResponse {

    let q = match &params.q {
        Some(q) if !q.trim().is_empty() => q.to_lowercase(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"status":"error","message":"Missing q"}))
            ).into_response();
        }
    };

    let has_male = q.contains("male") || q.contains("males");
    let has_female = q.contains("female") || q.contains("females");

    let gender = if has_male && has_female {
        None
    } else if has_male {
        Some("male")
    } else if has_female {
        Some("female")
    } else {
        None
    };

    let country_id = if q.contains("nigeria") || q.contains("from nigeria") {
        Some("NG")
    } else if q.contains("kenya") {
        Some("KE")
    } else {
        None
    };

    if gender.is_none() && country_id.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"status":"error","message":"Could not interpret query"}))
        ).into_response();
    }

    Json(json!({
        "status":"success",
        "query":q,
        "parsed":{
            "gender":gender,
            "country_id":country_id
        }
    })).into_response()
}