use serde::Deserialize;

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

pub fn build_query(filters: &Filters) -> (String, Vec<String>) {
    let mut query = "SELECT * FROM profiles WHERE 1=1".to_string();
    let mut params: Vec<String> = vec![];

    if let Some(g) = &filters.gender {
        query.push_str(" AND LOWER(gender) = ?");
        params.push(g.to_lowercase());
    }

    if let Some(c) = &filters.country_id {
        query.push_str(" AND UPPER(country_id) = ?");
        params.push(c.to_uppercase());
    }

    if let Some(a) = &filters.age_group {
        query.push_str(" AND LOWER(age_group) = ?");
        params.push(a.to_lowercase());
    }

    if let Some(min) = filters.min_age {
        query.push_str(" AND age >= ?");
        params.push(min.to_string());
    }

    if let Some(max) = filters.max_age {
        query.push_str(" AND age <= ?");
        params.push(max.to_string());
    }

    if let Some(min) = filters.min_gender_probability {
        query.push_str(" AND gender_probability >= ?");
        params.push(min.to_string());
    }

    if let Some(min) = filters.min_country_probability {
        query.push_str(" AND country_probability >= ?");
        params.push(min.to_string());
    }

    // sorting
    let sort = filters.sort_by.clone().unwrap_or("created_at".into());
    let order = filters.order.clone().unwrap_or("desc".into());

    query.push_str(&format!(" ORDER BY {} {}", sort, order));

    // pagination
    let page = filters.page.unwrap_or(1);
    let limit = filters.limit.unwrap_or(10);
    let offset = (page - 1) * limit;

    query.push_str(" LIMIT ? OFFSET ?");
    params.push(limit.to_string());
    params.push(offset.to_string());

    (query, params)
}