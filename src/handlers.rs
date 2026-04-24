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

// ── constants ────────────────────────────────────────────────────────────────

const MAX_LIMIT: i64 = 100;
const DEFAULT_LIMIT: i64 = 10;

const VALID_SORT_COLUMNS: &[&str] = &[
    "age", "name", "country_name", "gender",
    "created_at", "gender_probability", "country_probability",
];

// ── query-param structs ───────────────────────────────────────────────────────

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

// ── helpers ───────────────────────────────────────────────────────────────────

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

// ── CREATE ────────────────────────────────────────────────────────────────────

pub async fn create_profile(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateProfile>,
) -> Json<serde_json::Value> {
    let id = Uuid::now_v7().to_string();

    sqlx::query(
        r#"
        INSERT INTO profiles
        (id, name, gender, gender_probability, age, age_group, country_id, country_name, country_probability, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
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

    Json(json!({ "status": "created", "id": id }))
}

// ── GET LIST (with filtering, sorting, pagination) ────────────────────────────

pub async fn get_profiles(
    State(pool): State<PgPool>,
    Query(filters): Query<Filters>,
) -> impl IntoResponse {
    // --- validate sort_by ---
    let sort_column = match validated_sort_column(filters.sort_by.as_deref()) {
        Ok(col) => col,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "error",
                    "message": format!(
                        "Invalid sort_by value '{}'. Must be one of: {}",
                        filters.sort_by.as_deref().unwrap_or(""),
                        VALID_SORT_COLUMNS.join(", ")
                    )
                })),
            )
                .into_response();
        }
    };

    let sort_order = validated_order(filters.order.as_deref());
    let limit = capped_limit(filters.limit);
    let page = filters.page.unwrap_or(1);
    let offset = (page - 1) * limit;

    // --- build WHERE clause ---
    let mut conditions: Vec<String> = vec![];
    let mut bindings: Vec<String> = vec![];

    if let Some(name) = &filters.name {
        conditions.push(format!("name ILIKE ${}", bindings.len() + 1));
        bindings.push(format!("%{}%", name));
    }

    if let Some(country) = &filters.country {
        conditions.push(format!("LOWER(country_name) = ${}", bindings.len() + 1));
        bindings.push(country.to_lowercase());
    }

    if let Some(country_id) = &filters.country_id {
        conditions.push(format!("UPPER(country_id) = ${}", bindings.len() + 1));
        bindings.push(country_id.to_uppercase());
    }

    if let Some(gender) = &filters.gender {
        conditions.push(format!("LOWER(gender) = ${}", bindings.len() + 1));
        bindings.push(gender.to_lowercase());
    }

    if let Some(age_group) = &filters.age_group {
        conditions.push(format!("LOWER(age_group) = ${}", bindings.len() + 1));
        bindings.push(age_group.to_lowercase());
    }

    if let Some(min_age) = filters.min_age {
        conditions.push(format!("age >= ${}", bindings.len() + 1));
        bindings.push(min_age.to_string());
    }

    if let Some(max_age) = filters.max_age {
        conditions.push(format!("age <= ${}", bindings.len() + 1));
        bindings.push(max_age.to_string());
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

    // --- count ---
    let count_query = format!("SELECT COUNT(*) AS count FROM profiles {}", where_clause);
    let mut count_builder = sqlx::query(&count_query);
    for b in &bindings {
        count_builder = count_builder.bind(b);
    }
    let total: i64 = count_builder
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("count");

    // --- data ---
    let limit_pos = bindings.len() + 1;
    let offset_pos = bindings.len() + 2;
    let data_query = format!(
        "SELECT * FROM profiles {} ORDER BY {} {} LIMIT ${} OFFSET ${}",
        where_clause, sort_column, sort_order, limit_pos, offset_pos
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

    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "count": total,
            "page": page,
            "limit": limit,
            "data": data
        })),
    )
        .into_response()
}

// ── GET ONE ───────────────────────────────────────────────────────────────────

pub async fn get_profile(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let row: Option<Profile> =
        sqlx::query_as::<_, Profile>("SELECT * FROM profiles WHERE id = $1")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .unwrap();

    Json(json!({ "status": "success", "data": row }))
}

// ── DELETE ────────────────────────────────────────────────────────────────────

pub async fn delete_profile(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    sqlx::query("DELETE FROM profiles WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    Json(json!({ "status": "deleted" }))
}

// ── SEARCH ────────────────────────────────────────────────────────────────────

pub async fn search_profiles(
    State(pool): State<PgPool>,
    Query(params): Query<QueryParams>,
) -> Json<serde_json::Value> {
    let limit = params.limit();
    let page = params.page();
    let offset = params.offset();

    let mut conditions: Vec<String> = vec![];
    let mut bindings: Vec<String> = vec![];

    if let Some(search) = &params.search {
        conditions.push(format!("name ILIKE ${}", bindings.len() + 1));
        bindings.push(format!("%{}%", search));
    }

    if let Some(gender) = &params.gender {
        conditions.push(format!("LOWER(gender) = ${}", bindings.len() + 1));
        bindings.push(gender.to_lowercase());
    }

    if let Some(country) = &params.country {
        conditions.push(format!("UPPER(country_id) = ${}", bindings.len() + 1));
        bindings.push(country.to_uppercase());
    }

    if let Some(age_group) = &params.age_group {
        conditions.push(format!("LOWER(age_group) = ${}", bindings.len() + 1));
        bindings.push(age_group.to_lowercase());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_query = format!("SELECT COUNT(*) AS count FROM profiles {}", where_clause);
    let mut count_builder = sqlx::query(&count_query);
    for b in &bindings {
        count_builder = count_builder.bind(b);
    }
    let total: i64 = count_builder
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("count");

    let limit_pos = bindings.len() + 1;
    let offset_pos = bindings.len() + 2;
    let query = format!(
        "SELECT * FROM profiles {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause, limit_pos, offset_pos
    );

    let mut query_builder = sqlx::query(&query);
    for b in &bindings {
        query_builder = query_builder.bind(b);
    }
    query_builder = query_builder.bind(limit).bind(offset);

    let rows = query_builder.fetch_all(&pool).await.unwrap();

    let results: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<String, _>("id"),
                "name": r.get::<String, _>("name"),
                "gender": r.get::<String, _>("gender"),
                "age": r.get::<i32, _>("age"),
                "country_id": r.get::<String, _>("country_id"),
                "country_name": r.get::<String, _>("country_name"),
            })
        })
        .collect();

    Json(json!({
        "status": "success",
        "count": total,
        "page": page,
        "limit": limit,
        "data": results
    }))
}

// ── NATURAL LANGUAGE QUERY ────────────────────────────────────────────────────
//
// Parses free-text queries like "young males", "females above 30",
// "people from Nigeria", "adult males from Kenya", "Male and female teenagers above 17"
// into structured filters and delegates to the DB.

pub async fn natural_language_query(
    State(pool): State<PgPool>,
    Query(params): Query<NlpQuery>,
) -> impl IntoResponse {
    let q = match &params.q {
        Some(q) if !q.trim().is_empty() => q.trim().to_lowercase(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "error",
                    "message": "Missing or empty query parameter 'q'"
                })),
            )
                .into_response();
        }
    };

    // ── parse gender ─────────────────────────────────────────────────────────
    // "male and female" / "both" → no gender filter
    let gender: Option<String> = {
        let has_male = q.contains("male") || q.contains("man") || q.contains("men") || q.contains("boy");
        let has_female = q.contains("female") || q.contains("woman") || q.contains("women") || q.contains("girl");

        if has_male && has_female {
            None // both genders requested
        } else if has_male {
            Some("male".to_string())
        } else if has_female {
            Some("female".to_string())
        } else {
            None
        }
    };

    // ── parse age / age_group ─────────────────────────────────────────────────
    // explicit "above N" / "over N" / "under N" / "below N" / "between N and M"
    let mut min_age: Option<i32> = None;
    let mut max_age: Option<i32> = None;
    let mut age_group: Option<String> = None;

    // "above N" / "over N"
    if let Some(n) = extract_age_after(&q, &["above", "over", "older than", "greater than"]) {
        min_age = Some(n);
    }
    // "under N" / "below N" / "younger than N"
    if let Some(n) = extract_age_after(&q, &["under", "below", "younger than", "less than"]) {
        max_age = Some(n);
    }
    // "between N and M"
    if let Some((lo, hi)) = extract_between_ages(&q) {
        min_age = Some(lo);
        max_age = Some(hi);
    }

    // keyword-based age_group / age range (only when no explicit age found)
    if min_age.is_none() && max_age.is_none() {
        if q.contains("teenager") || q.contains("teen") || q.contains("adolescent") {
            age_group = Some("teenager".to_string());
        } else if q.contains("young adult") {
            age_group = Some("young_adult".to_string());
        } else if q.contains("young") || q.contains("youth") {
            // "young" → 18-35 range
            min_age = Some(18);
            max_age = Some(35);
        } else if q.contains("senior") || q.contains("elder") || q.contains("old") {
            age_group = Some("senior".to_string());
        } else if q.contains("adult") {
            age_group = Some("adult".to_string());
        } else if q.contains("child") || q.contains("kid") {
            age_group = Some("child".to_string());
        }
    }

    // if teenager keyword AND "above N" is set, combine them
    // e.g. "teenagers above 17" → age_group=teenager AND min_age=17
    if q.contains("teenager") || q.contains("teen") {
        age_group = Some("teenager".to_string());
        // min_age stays if set
    }

    // ── parse country ─────────────────────────────────────────────────────────
    let country_id: Option<String> = extract_country(&q);

    // ── build SQL ─────────────────────────────────────────────────────────────
    let mut conditions: Vec<String> = vec![];
    let mut bindings: Vec<String> = vec![];

    if let Some(g) = &gender {
        conditions.push(format!("LOWER(gender) = ${}", bindings.len() + 1));
        bindings.push(g.clone());
    }
    if let Some(ag) = &age_group {
        conditions.push(format!("LOWER(age_group) = ${}", bindings.len() + 1));
        bindings.push(ag.clone());
    }
    if let Some(mn) = min_age {
        conditions.push(format!("age >= ${}", bindings.len() + 1));
        bindings.push(mn.to_string());
    }
    if let Some(mx) = max_age {
        conditions.push(format!("age <= ${}", bindings.len() + 1));
        bindings.push(mx.to_string());
    }
    if let Some(cid) = &country_id {
        conditions.push(format!("UPPER(country_id) = ${}", bindings.len() + 1));
        bindings.push(cid.to_uppercase());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let limit = capped_limit(params.limit);
    let page = params.page.unwrap_or(1);
    let offset = (page - 1) * limit;

    // count
    let count_query = format!("SELECT COUNT(*) AS count FROM profiles {}", where_clause);
    let mut count_builder = sqlx::query(&count_query);
    for b in &bindings {
        count_builder = count_builder.bind(b);
    }
    let total: i64 = count_builder
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("count");

    // data
    let limit_pos = bindings.len() + 1;
    let offset_pos = bindings.len() + 2;
    let data_query = format!(
        "SELECT * FROM profiles {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause, limit_pos, offset_pos
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

    // reflect what was parsed back to the caller
    let parsed = json!({
        "gender": gender,
        "age_group": age_group,
        "min_age": min_age,
        "max_age": max_age,
        "country_id": country_id,
    });

    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "query": params.q,
            "parsed": parsed,
            "count": total,
            "page": page,
            "limit": limit,
            "data": data
        })),
    )
        .into_response()
}

// ── NLP helpers ───────────────────────────────────────────────────────────────

/// Extract an age number that follows any of the given keywords.
/// e.g. "above 30" → Some(30)
fn extract_age_after(q: &str, keywords: &[&str]) -> Option<i32> {
    for kw in keywords {
        if let Some(pos) = q.find(kw) {
            let rest = &q[pos + kw.len()..].trim_start();
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_str.parse::<i32>() {
                return Some(n);
            }
        }
    }
    None
}

/// Extract "between N and M" or "N to M" age ranges.
fn extract_between_ages(q: &str) -> Option<(i32, i32)> {
    // "between N and M"
    if let Some(pos) = q.find("between ") {
        let rest = &q[pos + 8..];
        let parts: Vec<&str> = rest.splitn(3, |c: char| !c.is_ascii_digit()).collect();
        // just use regex-free scan: grab first two numbers
        let nums: Vec<i32> = rest
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .take(2)
            .collect();
        let _ = parts; // suppress warning
        if nums.len() == 2 {
            return Some((nums[0], nums[1]));
        }
    }
    None
}

/// Map common country names / demonyms to ISO-3166 alpha-2 codes.
fn extract_country(q: &str) -> Option<String> {
    let map: &[(&str, &str)] = &[
        ("nigeria", "NG"),
        ("nigerian", "NG"),
        ("kenya", "KE"),
        ("kenyan", "KE"),
        ("ghana", "GH"),
        ("ghanaian", "GH"),
        ("south africa", "ZA"),
        ("south african", "ZA"),
        ("ethiopia", "ET"),
        ("ethiopian", "ET"),
        ("egypt", "EG"),
        ("egyptian", "EG"),
        ("tanzania", "TZ"),
        ("tanzanian", "TZ"),
        ("uganda", "UG"),
        ("ugandan", "UG"),
        ("rwanda", "RW"),
        ("rwandan", "RW"),
        ("cameroon", "CM"),
        ("cameroonian", "CM"),
        ("senegal", "SN"),
        ("senegalese", "SN"),
        ("ivory coast", "CI"),
        ("ivorian", "CI"),
        ("morocco", "MA"),
        ("moroccan", "MA"),
        ("algeria", "DZ"),
        ("algerian", "DZ"),
        ("angola", "AO"),
        ("angolan", "AO"),
        ("mozambique", "MZ"),
        ("mozambican", "MZ"),
        ("zambia", "ZM"),
        ("zambian", "ZM"),
        ("zimbabwe", "ZW"),
        ("zimbabwean", "ZW"),
        ("malawi", "MW"),
        ("malawian", "MW"),
        ("botswana", "BW"),
        ("botswanan", "BW"),
        ("namibia", "NA"),
        ("namibian", "NA"),
        ("united states", "US"),
        ("usa", "US"),
        ("american", "US"),
        ("united kingdom", "GB"),
        ("uk", "GB"),
        ("british", "GB"),
        ("canada", "CA"),
        ("canadian", "CA"),
        ("australia", "AU"),
        ("australian", "AU"),
        ("india", "IN"),
        ("indian", "IN"),
        ("china", "CN"),
        ("chinese", "CN"),
        ("brazil", "BR"),
        ("brazilian", "BR"),
        ("france", "FR"),
        ("french", "FR"),
        ("germany", "DE"),
        ("german", "DE"),
        ("japan", "JP"),
        ("japanese", "JP"),
    ];

    for (name, code) in map {
        if q.contains(name) {
            return Some(code.to_string());
        }
    }
    None
}
