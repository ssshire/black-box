use std::fmt;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

const DEFAULT_DATABASE_URL: &str = "postgres://chatuser:devpassword@localhost:5432/chatapp";

pub async fn connect() -> Result<PgPool, sqlx::Error> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
}

#[derive(Debug, Clone)]
pub struct AuthedUser {
    pub user_id: i32,
    pub username: String,
}

#[derive(Debug)]
pub enum AuthError {
    UsernameTaken,
    InvalidCredentials,
    Db(sqlx::Error),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::UsernameTaken => write!(f, "username already taken"),
            AuthError::InvalidCredentials => write!(f, "invalid username or password"),
            AuthError::Db(e) => write!(f, "database error: {}", e),
        }
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        AuthError::Db(e)
    }
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::InvalidCredentials)
}

fn is_unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505"))
}

pub async fn register_user(
    pool: &PgPool,
    username: &str,
    email: &str,
    password: &str,
) -> Result<AuthedUser, AuthError> {
    let password_hash = hash_password(password)?;

    let row = sqlx::query_as::<_, (i32,)>(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING user_id",
    )
    .bind(username)
    .bind(email)
    .bind(&password_hash)
    .fetch_one(pool)
    .await
    .map_err(|e| if is_unique_violation(&e) { AuthError::UsernameTaken } else { AuthError::Db(e) })?;

    Ok(AuthedUser {
        user_id: row.0,
        username: username.to_string(),
    })
}

pub async fn login_user(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<AuthedUser, AuthError> {
    let row = sqlx::query_as::<_, (i32, String)>(
        "SELECT user_id, password_hash FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?
    .ok_or(AuthError::InvalidCredentials)?;

    let (user_id, password_hash) = row;
    let parsed_hash = PasswordHash::new(&password_hash).map_err(|_| AuthError::InvalidCredentials)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidCredentials)?;

    Ok(AuthedUser {
        user_id,
        username: username.to_string(),
    })
}

pub async fn create_session(
    pool: &PgPool,
    name: &str,
    created_by: i32,
    max_size: i32,
) -> Result<i32, sqlx::Error> {
    let row = sqlx::query_as::<_, (i32,)>(
        "INSERT INTO sessions (session_name, created_by, max_size) VALUES ($1, $2, $3) RETURNING session_id",
    )
    .bind(name)
    .bind(created_by)
    .bind(max_size)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub async fn insert_message(
    pool: &PgPool,
    session_id: i32,
    user_id: i32,
    content: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO messages (session_id, user_id, content) VALUES ($1, $2, $3)")
        .bind(session_id)
        .bind(user_id)
        .bind(content)
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::password_hash::PasswordVerifier;

    #[test]
    fn hash_password_round_trips_with_argon2_verify() {
        let hash = hash_password("correct horse battery staple").unwrap();
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(Argon2::default()
            .verify_password(b"correct horse battery staple", &parsed)
            .is_ok());
        assert!(Argon2::default()
            .verify_password(b"wrong password", &parsed)
            .is_err());
    }
}