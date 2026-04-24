use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub gender_probability: f64,
    pub age: i32,
    pub age_group: String,
    pub country_id: String,
    pub country_name: String,
    pub country_probability: f64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateProfile {
    pub name: String,
    pub gender: String,
    pub gender_probability: f64,
    pub age: i32,
    pub age_group: String,
    pub country_id: String,
    pub country_name: String,
    pub country_probability: f64,
}