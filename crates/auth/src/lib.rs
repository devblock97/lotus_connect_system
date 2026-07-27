use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use errors::{AppError, Result};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub token_type: TokenType,
}

/// Hashes a password using Argon2id with a randomly generated salt
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| AppError::Internal(format!("Password hashing failed: {}", err)))?
        .to_string();
    Ok(password_hash)
}

/// Verifies a password against an Argon2id hash. Returns true if match, false otherwise.
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|err| {
        AppError::Internal(format!("Invalid password hash format: {}", err))
    })?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generates a JWT token for a given user ID, type, secret, and expiration
pub fn generate_token(
    user_id: Uuid,
    token_type: TokenType,
    secret: &str,
    expiration_value: i64,
) -> Result<(String, i64)> {
    let now = Utc::now();
    let duration = match token_type {
        TokenType::Access => chrono::Duration::try_minutes(expiration_value)
            .ok_or_else(|| AppError::Internal("Invalid duration format".to_string()))?,
        TokenType::Refresh => chrono::Duration::try_days(expiration_value)
            .ok_or_else(|| AppError::Internal("Invalid duration format".to_string()))?,
    };
    let exp_time = now + duration;
    let exp = exp_time.timestamp();
    let iat = now.timestamp();

    let claims = Claims {
        sub: user_id,
        exp,
        iat,
        token_type,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(format!("Failed to generate token: {}", err)))?;

    Ok((token, exp))
}

/// Verifies a JWT token using the configured secret key
pub fn verify_token(token: &str, secret: &str) -> Result<Claims> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|err| AppError::Authentication(format!("Invalid or expired token: {}", err)))?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "super_secure_password_123";
        let hash = hash_password(password).unwrap();
        
        // Assert password matches hash
        assert!(verify_password(password, &hash).unwrap());

        // Assert random wrong password does not match
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_jwt_generation_and_verification() {
        let user_id = Uuid::now_v7();
        let secret = "my_super_secret_jwt_sign_key_for_testing";
        
        // Generate access token
        let (token, _exp) = generate_token(user_id, TokenType::Access, secret, 15).unwrap();

        // Verify token
        let claims = verify_token(&token, secret).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.token_type, TokenType::Access);
    }
}
