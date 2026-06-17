use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use validator::Validate;

/// Custom validator for Stellar addresses
/// Stellar addresses are 56 characters long and start with 'G'
pub fn validate_stellar_address(address: &str) -> Result<(), validator::ValidationError> {
    if address.len() != 56 || !address.starts_with('G') {
        return Err(validator::ValidationError::new("invalid_stellar_address"));
    }
    Ok(())
}

/// Custom validator for timestamps - must be greater than current Unix timestamp (in seconds)
/// Allows a small grace period (300 seconds / 5 minutes) for clock skew
pub fn validate_future_timestamp(timestamp: i64) -> Result<(), validator::ValidationError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    
    // Allow 5 minutes grace period for clock skew
    if timestamp <= now - 300 {
        return Err(validator::ValidationError::new("future_timestamp"));
    }
    Ok(())
}

/// Extractor that validates the deserialized JSON data
/// Returns a 400 Bad Request with field-level validation errors if validation fails
pub struct ValidJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|err| {
                let error_msg = format!("Invalid JSON: {}", err);
                let body = Json(json!({
                    "error": {
                        "code": StatusCode::BAD_REQUEST.as_u16(),
                        "message": error_msg,
                    }
                }));
                (StatusCode::BAD_REQUEST, body).into_response()
            })?;

        value.validate().map_err(|err| {
            let error_msg = format!("Validation error: {}", err);
            let body = Json(json!({
                "error": {
                    "code": StatusCode::BAD_REQUEST.as_u16(),
                    "message": error_msg,
                    "details": err.field_errors()
                        .iter()
                        .map(|(field, errors)| {
                            let error_msgs: Vec<String> = errors
                                .iter()
                                .map(|e| e.message.as_ref()
                                    .map(|m| m.to_string())
                                    .unwrap_or_else(|| e.code.to_string()))
                                .collect();
                            (field.to_string(), error_msgs)
                        })
                        .collect::<Vec<_>>()
                }
            }));
            (StatusCode::BAD_REQUEST, body).into_response()
        })?;

        Ok(ValidJson(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_stellar_address_valid() {
        let valid_address = "GBBD47UZQ5SDELZ52UBNRSUYMK76GRBTQDTOOQKY6NZJWWQNFE4H6ZQR";
        assert!(validate_stellar_address(valid_address).is_ok());
    }

    #[test]
    fn test_validate_stellar_address_invalid_length() {
        let invalid_address = "GBBD47UZQ5SDELZ52UBNRSUYMK76GRBTQDTOOQKY6NZJWWQNFE4H";
        assert!(validate_stellar_address(invalid_address).is_err());
    }

    #[test]
    fn test_validate_stellar_address_invalid_prefix() {
        let invalid_address = "ABBD47UZQ5SDELZ52UBNRSUYMK76GRBTQDTOOQKY6NZJWWQNFE4H6ZQR";
        assert!(validate_stellar_address(invalid_address).is_err());
    }

    #[test]
    fn test_validate_future_timestamp() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let future = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() + 86400) as i64; // 1 day in the future
        assert!(validate_future_timestamp(future).is_ok());
    }
}
