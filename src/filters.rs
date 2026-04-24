// filters.rs – kept for backward compatibility but main filtering logic
// now lives in handlers.rs (get_profiles / search_profiles).
// If you still import build_query elsewhere, use this PostgreSQL-compatible version.

use serde::Deserialize;

const MAX_LIMIT: i64 = 100;

#[derive(Debug, Deserialize, Default)]
pub struct Filters {
    pub gender: Option<String>,
    pub age_group: Option<String>,
    pub country_id: Option<String>,

    pub min_age: Option<i32>,
    pub max_age: Option<i32>,

    pub min_gender_probability: Option<f64>,
    pub min_country_probability: Option<f64>,

    pub sort_by: Option<String>,
    pub order: Option<String>,

    pub page: Option<i64>,
    pub limit: Option<i64>,
}

const VALID_SORT_COLUMNS: &[&str] = &[
    "age", "name", "country_name", "gender",
    "created_at", "gender_probability", "country_probability",
];

/// Returns `(sql_string, positional_params)` ready for PostgreSQL ($1, $2, …).
/// Returns `Err(String)` when `sort_by` is invalid.
pub fn build_query(filters: &Filters) -> Result<(String, Vec<String>), String> {
    let mut conditions: Vec<String> = vec![];
    let mut params: Vec<String> = vec![];

    macro_rules! push {
        ($cond:expr, $val:expr) => {{
            conditions.push(format!($cond, params.len() + 1));
            params.push($val);
        }};
    }

    if let Some(g) = &filters.gender {
        push!("LOWER(gender) = ${}", g.to_lowercase());
    }
    if let Some(c) = &filters.country_id {
        push!("UPPER(country_id) = ${}", c.to_uppercase());
    }
    if let Some(a) = &filters.age_group {
        push!("LOWER(age_group) = ${}", a.to_lowercase());
    }
    if let Some(min) = filters.min_age {
        push!("age >= ${}", min.to_string());
    }
    if let Some(max) = filters.max_age {
        push!("age <= ${}", max.to_string());
    }
    if let Some(min_gp) = filters.min_gender_probability {
        push!("gender_probability >= ${}", min_gp.to_string());
    }
    if let Some(min_cp) = filters.min_country_probability {
        push!("country_probability >= ${}", min_cp.to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // validate sort column
    let sort_col = filters.sort_by.as_deref().unwrap_or("created_at");
    if !VALID_SORT_COLUMNS.contains(&sort_col) {
        return Err(format!(
            "Invalid sort_by '{}'. Allowed: {}",
            sort_col,
            VALID_SORT_COLUMNS.join(", ")
        ));
    }

    let order = match filters.order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let limit = filters.limit.unwrap_or(10).min(MAX_LIMIT);
    let page = filters.page.unwrap_or(1);
    let offset = (page - 1) * limit;

    let limit_pos = params.len() + 1;
    let offset_pos = params.len() + 2;

    let query = format!(
        "SELECT * FROM profiles {} ORDER BY {} {} LIMIT ${} OFFSET ${}",
        where_clause, sort_col, order, limit_pos, offset_pos
    );

    params.push(limit.to_string());
    params.push(offset.to_string());

    Ok((query, params))
}
