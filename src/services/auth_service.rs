use crate::error::ApiError;
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::Rng;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub async fn generate_nonce(
    redis: &mut ConnectionManager,
    address: &str,
) -> Result<String, ApiError> {
    let nonce: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let key = format!("nonce:{}", address);
    redis
        .set_ex::<_, _, ()>(&key, &nonce, 300)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(nonce)
}

pub async fn verify_and_issue_jwt(
    redis: &mut ConnectionManager,
    address: &str,
    signature_hex: &str,
    jwt_secret: &str,
    expiration_hours: u64,
) -> Result<String, ApiError> {
    let key = format!("nonce:{}", address);
    let nonce: Option<String> = redis
        .get(&key)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    let nonce = nonce.ok_or(ApiError::BadRequest("Nonce expired or not found".into()))?;

    // Decode hex signature
    let signature_bytes = hex_decode(signature_hex)
        .map_err(|_| ApiError::BadRequest("Invalid signature hex".into()))?;

    // Verify Ed25519 signature
    let valid = crate::utils::crypto::verify_stellar_signature(
        address,
        nonce.as_bytes(),
        &signature_bytes,
    )
    .map_err(|e| ApiError::BadRequest(format!("Signature verification failed: {}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized);
    }

    // Delete nonce
    redis
        .del::<_, ()>(&key)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;

    // Issue JWT
    let exp = chrono::Utc::now()
        + chrono::Duration::hours(expiration_hours as i64);
    let claims = Claims {
        sub: address.to_string(),
        exp: exp.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(e.into()))?;

    Ok(token)
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect()
}
