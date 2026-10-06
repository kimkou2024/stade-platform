//! Authentication: Argon2id hashing [M §10.2], JWT with expiry [M §7.1.3],
//! and an `AuthUser` extractor that guards protected routes.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AppError::Internal)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub token_type: String, // "access" | "refresh"
    pub exp: i64,
    pub iat: i64,
}

pub fn issue_token(
    secret: &str,
    user_id: Uuid,
    role: &str,
    token_type: &str,
    ttl_secs: i64,
) -> AppResult<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id,
        role: role.to_string(),
        token_type: token_type.to_string(),
        iat: now,
        exp: now + ttl_secs,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AppError::from)
}

pub fn verify_token(secret: &str, token: &str) -> AppResult<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(data.claims)
}

/// Authenticated user, extracted from the `Authorization: Bearer <jwt>` header.
/// Reject non-access tokens so refresh tokens can't be used as credentials.
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = header.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
        let claims = verify_token(&state.cfg.jwt_secret, token)?;
        if claims.token_type != "access" {
            return Err(AppError::Unauthorized);
        }
        Ok(AuthUser { user_id: claims.sub, role: claims.role })
    }
}

impl AuthUser {
    /// Coarse role gate. Fine-grained RBAC (role_permission) is layered later.
    pub fn require_staff(&self) -> AppResult<()> {
        match self.role.as_str() {
            "admin" | "operator" | "gate_agent" => Ok(()),
            _ => Err(AppError::Unauthorized),
        }
    }
}
