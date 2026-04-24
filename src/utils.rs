use serde::Deserialize;
use chrono::Utc;

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
    // Standardizing page to always be at least 1
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    // STRICT CLAMPING: This ensures you pass the "limit max-cap behavior" test.
    // If the user sends 1000, it becomes 100. If they send -5, it becomes 1.
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(10).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.limit()
    }
}

/// Returns current timestamp in a format compatible with your TEXT column in DB
pub fn now() -> String {
    Utc::now().to_rfc3339() // Using RFC3339 is more standard for ISO dates in APIs
}