use chrono::Utc;
use uuid::Uuid;

use crate::user::model::User;

use super::model::{SignupRequest, UserResponse, UserType};
use super::repository::UserRepository;
use sqlx::PgPool;

#[derive(Debug)]
pub enum SignupError {
    EmailAlreadyExists,
    InvalidAge,
    DatabaseError(sqlx::Error),
    HashError,
}

impl From<sqlx::Error> for SignupError {
    fn from(e: sqlx::Error) -> Self {
        SignupError::DatabaseError(e)
    }
}

pub struct UserService;

impl UserService {
    pub async fn signup(pool: &PgPool, req: SignupRequest) -> Result<UserResponse, SignupError> {
        // Check email is not taken
        if UserRepository::find_by_email(pool, &req.email)
            .await?
            .is_some()
        {
            return Err(SignupError::EmailAlreadyExists);
        }

        // Resolve name fields from either flow
        let full_name = req.full_name.clone().unwrap_or_else(|| {
            format!(
                "{} {}",
                req.first_name.as_deref().unwrap_or(""),
                req.last_name.as_deref().unwrap_or("")
            )
            .trim()
            .to_string()
        });
        let first_name = req.first_name.clone().unwrap_or_else(|| {
            full_name
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
        });
        let last_name = req.last_name.clone().unwrap_or_else(|| {
            full_name
                .split_whitespace()
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ")
        });

        // Calculate age from date_of_birth string if provided
        let (date_of_birth, age) = if let Some(dob_str) = &req.date_of_birth {
            let dob = dob_str
                .parse::<chrono::NaiveDate>()
                .map_err(|_| SignupError::InvalidAge)?;
            let today = Utc::now().date_naive();
            let mut age = today.year() - dob.year();
            if today.ordinal() < dob.ordinal() {
                age -= 1;
            }
            if age < 13 || age > 120 {
                return Err(SignupError::InvalidAge);
            }
            let dob_utc = dob.and_hms_opt(0, 0, 0).unwrap().and_utc();
            (Some(dob_utc), Some(age as u8))
        } else {
            (None, None)
        };

        // Generate unique username
        let base = format!("{}{}", first_name.to_lowercase(), last_name.to_lowercase())
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        let username = Self::generate_unique_username(pool, &base).await?;

        // Hash password
        let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
            .map_err(|_| SignupError::HashError)?;

        let params = User {
            id: Uuid::new_v6(),
            email: req.email,
            password_hash,
            full_name,
            first_name,
            last_name,
            username,
            date_of_birth,
            nationality: req.nationality,
            user_type: req.user_type,
            age,
            institution: req.institution.or_else(|| {
                match (&req.education_level, &req.year_level) {
                    (Some(e), Some(y)) => Some(format!("{} - {}", e, y)),
                    _ => None,
                }
            }),
            title: req.role.or(req.education_level),
            interests: req.area_of_study.map(|s| vec![s]).unwrap_or_default(),
            iq_score: req.iq_score,
            total_points: req.quiz_points.unwrap_or(0),
            iq_test_completed: req.iq_score.is_some(),
            email_verified: true,
            profile_completed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let user = UserRepository::create(pool, &params).await?;

        Ok(UserResponse {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            username: user.username,
            user_type: user.user_type,
            age: user.age.map(|a| a as u8),
            email_verified: user.email_verified,
            profile_completed: user.profile_completed,
        })
    }

    async fn generate_unique_username(pool: &PgPool, base: &str) -> Result<String, SignupError> {
        let mut username = base.to_string();
        let mut counter = 1u32;
        loop {
            if UserRepository::find_by_username(pool, &username)
                .await?
                .is_none()
            {
                return Ok(username);
            }
            username = format!("{}{}", base, counter);
            counter += 1;
        }
    }
}
