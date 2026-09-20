use druid::Data;
use serde::{Deserialize, Serialize};

/// What the right side of the diary page shows for the opened diary.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Data, Serialize, Deserialize)]
pub enum EditorMode {
    #[default]
    Edit,
    Split,
    Preview,
}

impl EditorMode {
    /// Returns the mode that follows this one when cycling through the modes.
    pub fn next(&self) -> Self {
        match self {
            Self::Edit => Self::Split,
            Self::Split => Self::Preview,
            Self::Preview => Self::Edit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modes_are_cycled_in_order() {
        assert_eq!(EditorMode::Edit.next(), EditorMode::Split);
        assert_eq!(EditorMode::Split.next(), EditorMode::Preview);
        assert_eq!(EditorMode::Preview.next(), EditorMode::Edit);
    }
}
