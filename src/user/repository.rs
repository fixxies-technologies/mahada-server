use sqlx::PgPool;
use uuid::Uuid;

use super::model::User;

pub struct UserRepository;

impl UserRepository {
    pub async fn create(pool: &PgPool, user: &User) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (
                id, email, password_hash, full_name, first_name, last_name,
                username, date_of_birth, nationality, user_type,
                age, institution, title, interests, iq_score, total_points,
                iq_test_completed, email_verified, profile_completed
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16,
                $17, $18, $19
            )
            RETURNING id, email, password_hash, full_name, first_name, last_name,
                      username, date_of_birth, nationality, user_type as "user_type: _",
                      age, institution, title, interests, iq_score, total_points,
                      iq_test_completed, email_verified, profile_completed,
                      profile_img, created_at, updated_at
            "#,
            user.id,
            user.email,
            user.password_hash,
            user.full_name,
            user.first_name,
            user.last_name,
            user.username,
            user.date_of_birth,
            user.nationality,
            user.user_type as _,
            user.age.map(|a| a as i16),
            user.institution,
            user.title,
            &user.interests,
            user.iq_score,
            user.total_points as i32,
            user.iq_test_completed,
            user.email_verified,
            user.profile_completed,
        )
        .fetch_one(pool)
        .await
    }
}
