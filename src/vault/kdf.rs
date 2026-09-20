use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::error::{VaultError, VaultResult};

pub const KDF_ALGORITHM_NAME: &str = "argon2id";
const DERIVED_KEY_LENGTH: usize = 32;
const MAX_MEMORY_KIB: u32 = 1_048_576;
const MAX_ITERATIONS: u32 = 20;
const MAX_PARALLELISM: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl KdfParams {
    pub const RECOMMENDED: Self = Self {
        memory_kib: 65_536,
        iterations: 3,
        parallelism: 1,
    };

    /// Rejects parameters outside of sane limits, so a tampered `.encrypted` file cannot
    /// make the application allocate huge amounts of memory.
    fn ensure_within_limits(&self) -> VaultResult<()> {
        [
            (1..=MAX_MEMORY_KIB).contains(&self.memory_kib),
            (1..=MAX_ITERATIONS).contains(&self.iterations),
            (1..=MAX_PARALLELISM).contains(&self.parallelism),
        ]
        .iter()
        .all(|within_limit| *within_limit)
        .then_some(())
        .ok_or_else(|| {
            VaultError::InvalidKeyFile("The key derivation parameters are out of range.".into())
        })
    }
}

/// Derives a 32 byte key from `password` and `salt` using Argon2id.
pub fn derive_key(
    password: &str,
    salt: &[u8],
    params: &KdfParams,
) -> VaultResult<Zeroizing<[u8; DERIVED_KEY_LENGTH]>> {
    params.ensure_within_limits()?;

    let argon2_params = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(DERIVED_KEY_LENGTH),
    )
    .map_err(|err| VaultError::KeyDerivation(err.to_string()))?;

    let mut derived_key = Zeroizing::new([0_u8; DERIVED_KEY_LENGTH]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params)
        .hash_password_into(password.as_bytes(), salt, &mut derived_key[..])
        .map_err(|err| VaultError::KeyDerivation(err.to_string()))?;

    Ok(derived_key)
}

#[cfg(test)]
pub const TEST_PARAMS: KdfParams = KdfParams {
    memory_kib: 64,
    iterations: 1,
    parallelism: 1,
};

#[cfg(test)]
mod tests {
    use super::*;

    const SALT: [u8; 16] = [3; 16];

    #[test]
    fn same_inputs_derive_same_key() -> anyhow::Result<()> {
        assert_eq!(
            derive_key("secret", &SALT, &TEST_PARAMS)?,
            derive_key("secret", &SALT, &TEST_PARAMS)?
        );
        Ok(())
    }

    #[test]
    fn password_and_salt_both_change_the_key() -> anyhow::Result<()> {
        let base = derive_key("secret", &SALT, &TEST_PARAMS)?;

        assert_ne!(base, derive_key("Secret", &SALT, &TEST_PARAMS)?);
        assert_ne!(base, derive_key("secret", &[4; 16], &TEST_PARAMS)?);
        Ok(())
    }

    #[test]
    fn rejects_parameters_outside_of_limits() {
        let too_much_memory = KdfParams {
            memory_kib: MAX_MEMORY_KIB + 1,
            ..TEST_PARAMS
        };
        let no_iterations = KdfParams {
            iterations: 0,
            ..TEST_PARAMS
        };

        assert!(derive_key("secret", &SALT, &too_much_memory).is_err());
        assert!(derive_key("secret", &SALT, &no_iterations).is_err());
    }
}
