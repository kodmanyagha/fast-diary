use druid::Data;

/// A removal from the recent folders that waits for the user to confirm it.
#[derive(Debug, Clone, PartialEq, Eq, Data)]
pub enum PendingConfirmation {
    RemoveRecentFolder(String),
    ClearRecentFolders,
}

impl PendingConfirmation {
    /// Returns the translation key of the question that asks the user to confirm.
    pub fn question_key(&self) -> &'static str {
        match self {
            Self::RemoveRecentFolder(_) => "page-login-confirmRemoveRecentFolder",
            Self::ClearRecentFolders => "page-login-confirmClearRecentFolders",
        }
    }

    /// Returns the folder this confirmation is about, when it is about a single folder.
    pub fn folder_path(&self) -> Option<&str> {
        match self {
            Self::RemoveRecentFolder(folder_path) => Some(folder_path),
            Self::ClearRecentFolders => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_confirmation_asks_its_own_question() {
        assert_ne!(
            PendingConfirmation::RemoveRecentFolder("/diary".into()).question_key(),
            PendingConfirmation::ClearRecentFolders.question_key()
        );
    }

    #[test]
    fn only_the_removal_of_one_folder_names_a_folder() {
        assert_eq!(
            PendingConfirmation::RemoveRecentFolder("/diary".into()).folder_path(),
            Some("/diary")
        );
        assert_eq!(PendingConfirmation::ClearRecentFolders.folder_path(), None);
    }
}
