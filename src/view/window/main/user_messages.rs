use std::error::Error;

use super::folder_command_error::FolderCommandError;
use crate::{
    modal::state::language::Language,
    utils::localization::{translate, translate_with},
    vault::error::VaultError,
};

/// Returns the translated message that tells the user what went wrong. Known failures get their
/// own sentence, anything else is reported as unexpected together with its technical details.
pub fn error_message(language: Language, error: &anyhow::Error) -> String {
    error
        .chain()
        .find_map(|cause| known_error_message(language, cause))
        .unwrap_or_else(|| {
            translate_with(
                language,
                "error-unexpected",
                &[("details", format!("{error:#}"))],
            )
        })
}

/// Returns the translated message of `cause` when it is a failure the user can act upon.
fn known_error_message(language: Language, cause: &(dyn Error + 'static)) -> Option<String> {
    cause
        .downcast_ref::<FolderCommandError>()
        .map(|error| folder_command_error_message(language, error))
        .or_else(|| {
            cause
                .downcast_ref::<VaultError>()
                .and_then(|error| vault_error_message(language, error))
        })
}

fn folder_command_error_message(language: Language, error: &FolderCommandError) -> String {
    match error {
        FolderCommandError::NoFolderSelected => translate(language, "error-noFolderSelected"),
        FolderCommandError::DiariesNotConverted(_) => {
            translate(language, "error-diariesNotConverted")
        }
        FolderCommandError::SomeDiariesNotConverted { failed } => translate_with(
            language,
            "error-someDiariesNotConverted",
            &[("failed", failed.to_string())],
        ),
    }
}

fn vault_error_message(language: Language, error: &VaultError) -> Option<String> {
    match error {
        VaultError::PasswordTooShort { minimum } => Some(translate_with(
            language,
            "error-passwordTooShort",
            &[("minimum", minimum.to_string())],
        )),
        VaultError::PasswordMismatch => Some(translate(language, "error-passwordMismatch")),
        VaultError::WrongPassword => Some(translate(language, "error-wrongPassword")),
        VaultError::AlreadyEncrypted => Some(translate(language, "error-alreadyEncrypted")),
        VaultError::InvalidKeyFile(_) => Some(translate(language, "error-keyFileDamaged")),
        VaultError::CorruptedContent => Some(translate(language, "error-diaryCorrupted")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;

    #[test]
    fn a_wrong_password_is_reported_in_the_chosen_language() {
        let error = anyhow::Error::from(VaultError::WrongPassword);

        assert_eq!(error_message(Language::Turkish, &error), "Yanlış şifre.");
        assert_eq!(error_message(Language::English, &error), "Wrong password.");
    }

    #[test]
    fn numbers_of_the_failure_are_put_into_the_message() {
        let too_short = anyhow::Error::from(VaultError::PasswordTooShort { minimum: 6 });
        let not_converted =
            anyhow::Error::from(FolderCommandError::SomeDiariesNotConverted { failed: 3 });

        assert!(error_message(Language::Turkish, &too_short).contains('6'));
        assert!(error_message(Language::Turkish, &not_converted).contains('3'));
    }

    #[test]
    fn the_outermost_known_failure_explains_the_error() {
        let error = anyhow::Error::from(FolderCommandError::DiariesNotConverted(
            VaultError::VerificationFailed,
        ));

        assert_eq!(
            error_message(Language::English, &error),
            translate(Language::English, "error-diariesNotConverted")
        );
    }

    #[test]
    fn a_known_failure_is_found_behind_added_context() {
        let error = Err::<(), _>(VaultError::PasswordMismatch)
            .context("while encrypting")
            .err()
            .map(|error| error_message(Language::English, &error));

        assert_eq!(
            error,
            Some(translate(Language::English, "error-passwordMismatch"))
        );
    }

    #[test]
    fn an_unknown_failure_is_reported_as_unexpected_with_its_details() {
        let error = anyhow::anyhow!("disk is on fire");
        let message = error_message(Language::Turkish, &error);

        assert!(message.starts_with("Beklenmeyen bir hata oluştu"));
        assert!(message.contains("disk is on fire"));
    }
}
