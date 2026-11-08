use std::time::{Duration, Instant};

use druid::text::Selection;

/// The most edits that can be undone; older ones are forgotten.
const MAX_UNDO_STEPS: usize = 500;

/// Edits of the same kind that follow each other within this pause are undone together.
const TYPING_PAUSE: Duration = Duration::from_millis(1000);

/// The text of the editor together with its selection at one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorSnapshot {
    pub text: String,
    pub selection: Selection,
}

impl EditorSnapshot {
    pub fn new(text: impl Into<String>, selection: Selection) -> Self {
        Self {
            text: text.into(),
            selection,
        }
    }
}

/// What an edit did to the text, used to decide which edits are undone together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    TypedWordCharacter,
    TypedWhitespace,
    DeletedCharacter,
    Other,
}

impl EditKind {
    /// Tells what turned `before` into `after` by looking at the part between their common
    /// beginning and their common ending.
    pub fn between(before: &str, after: &str) -> Self {
        let prefix_length = common_prefix_length(before, after);
        let suffix_length = common_suffix_length(&before[prefix_length..], &after[prefix_length..]);
        let removed = &before[prefix_length..before.len() - suffix_length];
        let inserted = &after[prefix_length..after.len() - suffix_length];

        match (single_char(removed), single_char(inserted)) {
            (None, Some(typed)) if removed.is_empty() && typed.is_whitespace() => {
                Self::TypedWhitespace
            }
            (None, Some(_)) if removed.is_empty() => Self::TypedWordCharacter,
            (Some(_), None) if inserted.is_empty() => Self::DeletedCharacter,
            _ => Self::Other,
        }
    }

    fn can_join(self, next: Self) -> bool {
        self == next && self != Self::Other
    }
}

/// Returns the byte length of the longest beginning `a` and `b` share, on a character boundary.
fn common_prefix_length(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .take_while(|((_, a_char), b_char)| a_char == b_char)
        .last()
        .map_or(0, |((index, shared), _)| index + shared.len_utf8())
}

/// Returns the byte length of the longest ending `a` and `b` share, on a character boundary.
fn common_suffix_length(a: &str, b: &str) -> usize {
    a.chars()
        .rev()
        .zip(b.chars().rev())
        .take_while(|(a_char, b_char)| a_char == b_char)
        .map(|(shared, _)| shared.len_utf8())
        .sum()
}

/// Returns the only character of `text`, or `None` when it has none or more than one.
fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    chars.next().filter(|_| chars.next().is_none())
}

/// The last recorded edit, remembered so that the next one can be joined to it.
#[derive(Debug, Clone, Copy)]
struct LastEdit {
    kind: EditKind,
    at: Instant,
}

/// The undo and redo steps of an editor. Each step is the snapshot to go back to.
#[derive(Debug, Default)]
pub struct UndoHistory {
    undo_steps: Vec<EditorSnapshot>,
    redo_steps: Vec<EditorSnapshot>,
    last_edit: Option<LastEdit>,
}

impl UndoHistory {
    /// Records that the editor left the state `before` through an edit of `kind` made at `now`.
    /// The edit is joined to the previous step when it continues the same kind of typing.
    pub fn record(&mut self, before: EditorSnapshot, kind: EditKind, now: Instant) {
        let joins_last_step = self.last_edit.is_some_and(|last| {
            last.kind.can_join(kind) && now.saturating_duration_since(last.at) <= TYPING_PAUSE
        });

        if !joins_last_step {
            self.undo_steps.push(before);
            let overflow = self.undo_steps.len().saturating_sub(MAX_UNDO_STEPS);
            self.undo_steps.drain(..overflow);
        }

        self.redo_steps.clear();
        self.last_edit = Some(LastEdit { kind, at: now });
    }

    /// Steps back from `current`, returning the snapshot to show, or `None` when there is
    /// nothing to undo.
    pub fn undo(&mut self, current: EditorSnapshot) -> Option<EditorSnapshot> {
        self.last_edit = None;
        self.undo_steps
            .pop()
            .inspect(|_| self.redo_steps.push(current))
    }

    /// Steps forward again from `current`, returning the snapshot to show, or `None` when there
    /// is nothing to redo.
    pub fn redo(&mut self, current: EditorSnapshot) -> Option<EditorSnapshot> {
        self.last_edit = None;
        self.redo_steps
            .pop()
            .inspect(|_| self.undo_steps.push(current))
    }

    /// Forgets every step, for example when another diary is opened.
    pub fn clear(&mut self) {
        self.undo_steps.clear();
        self.redo_steps.clear();
        self.last_edit = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &str) -> EditorSnapshot {
        EditorSnapshot::new(text, Selection::caret(text.len()))
    }

    fn type_text(history: &mut UndoHistory, start: &str, typed: &str, now: Instant) -> String {
        typed.chars().fold(start.to_string(), |text, typed_char| {
            let after = format!("{text}{typed_char}");
            history.record(snapshot(&text), EditKind::between(&text, &after), now);
            after
        })
    }

    #[test]
    fn edits_are_told_apart_by_what_changed() {
        assert_eq!(EditKind::between("ab", "abc"), EditKind::TypedWordCharacter);
        assert_eq!(EditKind::between("ac", "abc"), EditKind::TypedWordCharacter);
        assert_eq!(EditKind::between("ab", "ab "), EditKind::TypedWhitespace);
        assert_eq!(EditKind::between("ab", "ab\n"), EditKind::TypedWhitespace);
        assert_eq!(EditKind::between("abc", "ac"), EditKind::DeletedCharacter);
        assert_eq!(EditKind::between("ab", "abcd"), EditKind::Other);
        assert_eq!(EditKind::between("abc", "axc"), EditKind::Other);
        assert_eq!(EditKind::between("abcd", "ad"), EditKind::Other);
    }

    #[test]
    fn edits_of_multi_byte_characters_are_measured_on_character_boundaries() {
        assert_eq!(EditKind::between("ğü", "ğşü"), EditKind::TypedWordCharacter);
        assert_eq!(EditKind::between("ğğ", "ğ"), EditKind::DeletedCharacter);
        assert_eq!(EditKind::between("aa", "aaa"), EditKind::TypedWordCharacter);
    }

    #[test]
    fn typing_is_undone_word_by_word() {
        let mut history = UndoHistory::default();
        let now = Instant::now();
        let text = type_text(&mut history, "", "hello world", now);

        let undone_word = history.undo(snapshot(&text));
        assert_eq!(undone_word, Some(snapshot("hello ")));
        let undone_space = history.undo(snapshot("hello "));
        assert_eq!(undone_space, Some(snapshot("hello")));
        let undone_first_word = history.undo(snapshot("hello"));
        assert_eq!(undone_first_word, Some(snapshot("")));
        assert_eq!(history.undo(snapshot("")), None);
    }

    #[test]
    fn a_pause_in_typing_starts_a_new_step() {
        let mut history = UndoHistory::default();
        let start = Instant::now();
        let text = type_text(&mut history, "", "ab", start);
        let text = type_text(&mut history, &text, "cd", start + TYPING_PAUSE * 2);

        assert_eq!(history.undo(snapshot(&text)), Some(snapshot("ab")));
        assert_eq!(history.undo(snapshot("ab")), Some(snapshot("")));
    }

    #[test]
    fn redo_brings_back_what_undo_took_and_a_new_edit_drops_it() {
        let mut history = UndoHistory::default();
        let now = Instant::now();
        history.record(snapshot(""), EditKind::Other, now);

        assert_eq!(history.undo(snapshot("pasted")), Some(snapshot("")));
        assert_eq!(history.redo(snapshot("")), Some(snapshot("pasted")));
        assert_eq!(history.undo(snapshot("pasted")), Some(snapshot("")));

        history.record(snapshot(""), EditKind::Other, now);
        assert_eq!(history.redo(snapshot("x")), None);
    }

    #[test]
    fn typing_after_an_undo_starts_a_new_step() {
        let mut history = UndoHistory::default();
        let now = Instant::now();
        let text = type_text(&mut history, "", "ab", now);
        let undone = history.undo(snapshot(&text));
        assert_eq!(undone, Some(snapshot("")));

        let text = type_text(&mut history, "", "c", now);
        assert_eq!(history.undo(snapshot(&text)), Some(snapshot("")));
    }

    #[test]
    fn the_oldest_steps_are_forgotten_beyond_the_limit() {
        let mut history = UndoHistory::default();
        let now = Instant::now();
        (0..=MAX_UNDO_STEPS)
            .for_each(|step| history.record(snapshot(&step.to_string()), EditKind::Other, now));

        let undone = std::iter::successors(history.undo(snapshot("last")), |shown| {
            history.undo(shown.clone())
        })
        .count();
        assert_eq!(undone, MAX_UNDO_STEPS);
    }

    #[test]
    fn clearing_forgets_every_step() {
        let mut history = UndoHistory::default();
        history.record(snapshot(""), EditKind::Other, Instant::now());
        history.clear();

        assert_eq!(history.undo(snapshot("x")), None);
        assert_eq!(history.redo(snapshot("x")), None);
    }
}
