use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use stellar_strkey::Strkey;

pub fn verify_stellar_signature(
    address: &str,
    message: &[u8],
    signature_bytes: &[u8],
) -> Result<bool, anyhow::Error> {
    let str_key = Strkey::from_string(address)
        .map_err(|e| anyhow::anyhow!("Invalid Stellar address: {:?}", e))?;

    let public_key_bytes = match str_key {
        Strkey::PublicKeyEd25519(pk) => pk.0,
        _ => return Err(anyhow::anyhow!("Not an Ed25519 public key address")),
    };

    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)?;
    let signature = Signature::from_slice(signature_bytes)?;

    Ok(verifying_key.verify(message, &signature).is_ok())
}
