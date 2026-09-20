use std::fmt;

use base64::{engine::general_purpose::STANDARD, Engine};
use zeroize::Zeroizing;

use super::error::{VaultError, VaultResult};

pub struct FolderKey(Zeroizing<[u8; FolderKey::LENGTH]>);

impl FolderKey {
    pub const LENGTH: usize = 32;

    /// Generates a new random key from the operating system's secure random generator.
    pub fn generate() -> VaultResult<Self> {
        let mut bytes = Zeroizing::new([0_u8; Self::LENGTH]);
        getrandom::fill(&mut bytes[..]).map_err(|err| VaultError::Randomness(err.to_string()))?;
        Ok(Self(bytes))
    }

    /// Decodes a key from its base64 form, checking that it has the exact key length.
    pub fn from_base64(encoded: &str) -> VaultResult<Self> {
        let decoded = Zeroizing::new(STANDARD.decode(encoded).map_err(|_| {
            VaultError::InvalidKeyFile("The folder key is not valid base64.".into())
        })?);
        let bytes = <[u8; Self::LENGTH]>::try_from(decoded.as_slice())
            .map_err(|_| VaultError::InvalidKeyFile("The folder key has a wrong length.".into()))?;

        Ok(Self(Zeroizing::new(bytes)))
    }

    /// Returns the base64 form of the key, as stored inside the `.encrypted` file.
    pub fn to_base64(&self) -> Zeroizing<String> {
        Zeroizing::new(STANDARD.encode(self.0.as_slice()))
    }

    /// Returns the raw key bytes.
    pub fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}

impl fmt::Debug for FolderKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FolderKey(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_unique() -> anyhow::Result<()> {
        assert_ne!(
            FolderKey::generate()?.as_bytes(),
            FolderKey::generate()?.as_bytes()
        );
        Ok(())
    }

    #[test]
    fn base64_round_trip_preserves_key() -> anyhow::Result<()> {
        let key = FolderKey::generate()?;

        let restored = FolderKey::from_base64(&key.to_base64())?;

        assert_eq!(restored.as_bytes(), key.as_bytes());
        Ok(())
    }

    #[test]
    fn rejects_malformed_or_short_keys() {
        assert!(FolderKey::from_base64("not base64!!").is_err());
        assert!(FolderKey::from_base64(&STANDARD.encode([1_u8; 16])).is_err());
    }

    #[test]
    fn debug_output_hides_key_material() -> anyhow::Result<()> {
        assert_eq!(
            format!("{:?}", FolderKey::generate()?),
            "FolderKey(<redacted>)"
        );
        Ok(())
    }
}
