use super::error::{VaultError, VaultResult};

pub const MIN_PASSWORD_LENGTH: usize = 6;
pub const RECOMMENDED_PASSWORD_LENGTH: usize = 10;

/// Why an acceptable password is still easy to guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordWeakness {
    DigitsOnly,
    ShorterThanRecommended,
}

/// Checks that a new password meets the minimum length and matches its confirmation.
pub fn validate_new_password(password: &str, confirmation: &str) -> VaultResult<()> {
    if password.chars().count() < MIN_PASSWORD_LENGTH {
        return Err(VaultError::PasswordTooShort {
            minimum: MIN_PASSWORD_LENGTH,
        });
    }

    (password == confirmation)
        .then_some(())
        .ok_or(VaultError::PasswordMismatch)
}

/// Returns why `password` is easy to guess, or `None` when it is empty or fine.
pub fn password_weakness(password: &str) -> Option<PasswordWeakness> {
    match password.chars().count() {
        0 => None,
        _ if password.chars().all(|character| character.is_ascii_digit()) => {
            Some(PasswordWeakness::DigitsOnly)
        }
        length if length < RECOMMENDED_PASSWORD_LENGTH => {
            Some(PasswordWeakness::ShorterThanRecommended)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_passwords() {
        assert!(matches!(
            validate_new_password("12345", "12345"),
            Err(VaultError::PasswordTooShort { minimum: 6 })
        ));
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert!(validate_new_password("ğüışöç", "ğüışöç").is_ok());
    }

    #[test]
    fn rejects_mismatching_confirmation() {
        assert!(matches!(
            validate_new_password("123456", "123457"),
            Err(VaultError::PasswordMismatch)
        ));
    }

    #[test]
    fn warns_about_digit_only_and_short_passwords() {
        assert_eq!(password_weakness(""), None);
        assert_eq!(
            password_weakness("1234567890123"),
            Some(PasswordWeakness::DigitsOnly)
        );
        assert_eq!(
            password_weakness("abc12def"),
            Some(PasswordWeakness::ShorterThanRecommended)
        );
        assert_eq!(password_weakness("a long enough passphrase"), None);
    }
}
