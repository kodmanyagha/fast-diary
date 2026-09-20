use super::error::{VaultError, VaultResult};

pub const MIN_PASSWORD_LENGTH: usize = 6;
const RECOMMENDED_PASSWORD_LENGTH: usize = 10;

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

/// Returns a hint when `password` is acceptable but easy to guess, or `None` when it is fine.
pub fn weak_password_warning(password: &str) -> Option<&'static str> {
    match password.chars().count() {
        0 => None,
        _ if password.chars().all(|character| character.is_ascii_digit()) => {
            Some("Digits-only passwords are easy to guess.")
        }
        length if length < RECOMMENDED_PASSWORD_LENGTH => {
            Some("Short password: 10 or more characters are recommended.")
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
        assert_eq!(weak_password_warning(""), None);
        assert!(weak_password_warning("1234567890123").is_some());
        assert!(weak_password_warning("abc12def").is_some());
        assert_eq!(weak_password_warning("a long enough passphrase"), None);
    }
}
