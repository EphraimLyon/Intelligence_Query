use serde::Deserialize;
use chrono::Utc;

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub search: Option<String>,
    pub gender: Option<String>,
    pub country: Option<String>,
    pub age_group: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl QueryParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(10).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

/// Returns current timestamp
pub fn now() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}