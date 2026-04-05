// src/user/model.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserType {
    Individual,
    Youth,
    Student,
    Researcher,
    Innovator,
    Organization,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub first_name: String,
    pub last_name: String,
    pub username: String,
    pub date_of_birth: Option<DateTime<Utc>>,
    pub nationality: Option<String>,
    pub user_type: UserType,
    pub age: Option<u8>,
    pub institution: Option<String>,
    pub title: Option<String>,
    pub interests: Vec<String>,
    pub iq_score: Option<f64>,
    pub total_points: u32,
    pub iq_test_completed: bool,
    pub email_verified: bool,
    pub profile_completed: bool,
    pub profile_img: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub user_type: UserType,
    // New flow
    pub full_name: Option<String>,
    pub role: Option<String>,
    pub institution: Option<String>,
    // Old flow
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub date_of_birth: Option<String>,
    // Optional for both
    pub nationality: Option<String>,
    pub education_level: Option<String>,
    pub year_level: Option<String>,
    pub area_of_study: Option<String>,
    pub profile_image: Option<String>,
    pub iq_score: Option<f64>,
    pub quiz_points: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub username: String,
    pub user_type: UserType,
    pub age: Option<u8>,
    pub email_verified: bool,
    pub profile_completed: bool,
}
