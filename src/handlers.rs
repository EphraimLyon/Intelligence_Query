use axum::{
    extract::{State, Path, Query},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::utils::{QueryParams, now};
use sqlx::{SqlitePool, Row};

use crate::models::{Profile, CreateProfile};

#[derive(Deserialize)]
pub struct Filters {
    pub name: Option<String>,
    pub country: Option<String>,
    pub gender: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub min_age: Option<i32>,
    pub max_age: Option<i32>,
    pub country_id: Option<String>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
}

pub async fn create_profile(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CreateProfile>,
) -> Json<serde_json::Value> {

    // ✅ UUIDv7
    let id = Uuid::now_v7().to_string();

    sqlx::query(
        r#"
        INSERT INTO profiles
        (id, name, gender, gender_probability, age, age_group, country_id, country_name, country_probability, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .await
    .unwrap();

    Json(json!({
        "status": "created",
        "id": id
    }))
}

pub async fn get_profiles(
    State(pool): State<SqlitePool>,
    Query(filters): Query<Filters>,
) -> Json<serde_json::Value> {

    // ✅ Pagination defaults
    let page = filters.page.unwrap_or(1);
    let limit = filters.limit.unwrap_or(10);
    let offset = (page - 1) * limit;

    let mut conditions = vec![];
    let mut bindings: Vec<String> = vec![];

    // ✅ Safe filters
    if let Some(name) = &filters.name {
        conditions.push("name LIKE ?");
        bindings.push(format!("%{}%", name));
    }

    if let Some(country) = &filters.country {
        conditions.push("country_name = ?");
        bindings.push(country.clone());
    }

    // ✅ country_id filter
    if let Some(country_id) = &filters.country_id {
        conditions.push("country_id = ?");
        bindings.push(country_id.clone());
    }

    // ✅ min_age filter
    if let Some(min_age) = filters.min_age {
        conditions.push("age >= ?");
        bindings.push(min_age.to_string());
    }

    // ✅ max_age filter
    if let Some(max_age) = filters.max_age {
        conditions.push("age <= ?");
        bindings.push(max_age.to_string());
    }

    if let Some(gender) = &filters.gender {
        conditions.push("gender = ?");
        bindings.push(gender.clone());
    }

    let where_clause = if conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // ✅ TOTAL COUNT (important for pagination)
    let count_query = format!("SELECT COUNT(*) as count FROM profiles {}", where_clause);
    let mut count_builder = sqlx::query(&count_query);

    for b in &bindings {
        count_builder = count_builder.bind(b);
    }

    let total: i64 = count_builder
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("count");

    // ✅ Safe sort_by — whitelist only, never interpolate user input directly
    let sort_column = match filters.sort_by.as_deref() {
        Some("age") => "age",
        Some("name") => "name",
        Some("country_name") => "country_name",
        Some("gender") => "gender",
        _ => "created_at",
    };

    // ✅ Safe order direction
    let sort_order = match filters.order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    // ✅ MAIN QUERY with LIMIT + OFFSET
    let data_query = format!(
        "SELECT * FROM profiles {} ORDER BY {} {} LIMIT ? OFFSET ?",
        where_clause, sort_column, sort_order
    );

    let mut query_builder = sqlx::query(&data_query);

    for b in &bindings {
        query_builder = query_builder.bind(b);
    }

    query_builder = query_builder.bind(limit).bind(offset);

    let rows = query_builder.fetch_all(&pool).await.unwrap();

    let data: Vec<Profile> = rows
        .into_iter()
        .map(|row| Profile {
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
        })
        .collect();

    Json(json!({
        "status": "success",
        "count": total,
        "page": page,
        "limit": limit,
        "data": data
    }))
}

pub async fn get_profile(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {

    let row: Option<Profile> =
        sqlx::query_as::<_, Profile>("SELECT * FROM profiles WHERE id = ?")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .unwrap();

    Json(json!({
        "status": "success",
        "data": row
    }))
}

pub async fn delete_profile(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {

    sqlx::query("DELETE FROM profiles WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    Json(json!({
        "status": "deleted"
    }))
}

pub async fn search_profiles(
    State(pool): State<SqlitePool>,
    Query(params): Query<QueryParams>,
) -> Json<serde_json::Value> {

    let mut conditions = vec![];
    let mut bindings: Vec<String> = vec![];

    if let Some(search) = &params.search {
        conditions.push("name LIKE ?");
        bindings.push(format!("%{}%", search));
    }

    if let Some(gender) = &params.gender {
        conditions.push("gender = ?");
        bindings.push(gender.clone());
    }

    if let Some(country) = &params.country {
        conditions.push("country_id = ?");
        bindings.push(country.clone());
    }

    if let Some(age_group) = &params.age_group {
        conditions.push("age_group = ?");
        bindings.push(age_group.clone());
    }

    let where_clause = if conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_query = format!("SELECT COUNT(*) as count FROM profiles {}", where_clause);
    let mut count_builder = sqlx::query(&count_query);

    for b in &bindings {
        count_builder = count_builder.bind(b);
    }

    let total: i64 = count_builder
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("count");

    let query = format!(
        "SELECT * FROM profiles {} LIMIT ? OFFSET ?",
        where_clause
    );

    let mut query_builder = sqlx::query(&query);

    for b in &bindings {
        query_builder = query_builder.bind(b);
    }

    query_builder = query_builder
        .bind(params.limit())
        .bind(params.offset());

    let rows = query_builder.fetch_all(&pool).await.unwrap();

    let results: Vec<serde_json::Value> = rows.iter().map(|r| {
        json!({
            "id": r.get::<String, _>("id"),
            "name": r.get::<String, _>("name"),
            "gender": r.get::<String, _>("gender"),
            "age": r.get::<i32, _>("age"),
            "country_id": r.get::<String, _>("country_id"),
            "country_name": r.get::<String, _>("country_name"),
        })
    }).collect();

    Json(json!({
        "status": "success",
        "count": total,
        "page": params.page(),
        "limit": params.limit(),
        "data": results
    }))
}