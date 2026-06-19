use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

/// Mint a signed HS256 JWT for `address` using `secret`.
/// The token is valid for one hour — enough for any test run.
pub fn mint_token(address: &str, secret: &str) -> String {
    let claims = Claims {
        sub: address.to_owned(),
        exp: 9999999999,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("test JWT encoding failed")
}
