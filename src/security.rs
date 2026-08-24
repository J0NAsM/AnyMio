use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use thiserror::Error;

const MEMORY_KIB: u32 = 19_456;
const ITERATIONS: u32 = 3;
const PARALLELISM: u32 = 1;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("invalid password hash")]
    InvalidHash,
    #[error("password hashing failed")]
    Hashing,
}

fn argon2() -> Result<Argon2<'static>, PasswordError> {
    let params = Params::new(MEMORY_KIB, ITERATIONS, PARALLELISM, Some(32))
        .map_err(|_| PasswordError::Hashing)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

pub fn hash_unattended_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    argon2()?
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::Hashing)
}

pub fn verify_unattended_password(password: &str, encoded: &str) -> Result<bool, PasswordError> {
    let hash = PasswordHash::new(encoded).map_err(|_| PasswordError::InvalidHash)?;
    Ok(argon2()?
        .verify_password(password.as_bytes(), &hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passwords_are_salted_and_verifiable() {
        let hash = hash_unattended_password("correct horse battery staple").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_unattended_password("correct horse battery staple", &hash).unwrap());
        assert!(!verify_unattended_password("nope", &hash).unwrap());
    }
}
