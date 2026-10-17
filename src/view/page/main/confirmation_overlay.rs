use druid::{
    theme,
    widget::{Button, CrossAxisAlignment, Either, Flex, Label, LineBreaking, SizedBox},
    Color, Widget, WidgetExt,
};

use crate::{
    consts::druid_selector,
    modal::app_state::AppState,
    utils::{
        display_text::shorten_in_the_middle,
        localization::{text, translate},
    },
};

const DIM_COLOR: Color = Color::rgba8(0, 0, 0, 140);
const CARD_WIDTH: f64 = 440.0;
const CARD_PADDING: f64 = 20.0;
const CARD_CORNER_RADIUS: f64 = 6.0;
const CARD_BORDER_WIDTH: f64 = 1.0;
const BUTTON_WIDTH: f64 = 110.0;
const MAX_FOLDER_PATH_CHARS: usize = 52;

/// Returns the translated question of the pending confirmation.
fn question_text(app_state: &AppState) -> String {
    app_state
        .pending_confirmation
        .as_ref()
        .map(|confirmation| translate(app_state.language, confirmation.question_key()))
        .unwrap_or_default()
}

/// Returns the shortened folder of the pending confirmation, or nothing when it is not about a
/// single folder.
fn folder_path_text(app_state: &AppState) -> String {
    app_state
        .pending_confirmation
        .as_ref()
        .and_then(|confirmation| confirmation.folder_path())
        .map(|folder_path| shorten_in_the_middle(folder_path, MAX_FOLDER_PATH_CHARS))
        .unwrap_or_default()
}

fn build_answer_buttons() -> impl Widget<AppState> {
    Flex::row()
        .with_flex_spacer(1.0)
        .with_child(
            Button::new(text("page-login-confirmCancel"))
                .on_click(|_ctx, app_state: &mut AppState, _env| {
                    app_state.pending_confirmation = None
                })
                .fix_width(BUTTON_WIDTH),
        )
        .with_default_spacer()
        .with_child(
            Button::new(text("page-login-confirmOk"))
                .on_click(|ctx, _app_state: &mut AppState, _env| {
                    ctx.submit_command(druid_selector::CONFIRMATION_ACCEPT)
                })
                .fix_width(BUTTON_WIDTH),
        )
}

/// Builds the card that holds the question, the folder it is about and the answer buttons.
fn build_question_card() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Label::dynamic(|app_state: &AppState, _env| question_text(app_state))
                .with_line_break_mode(LineBreaking::WordWrap),
        )
        .with_default_spacer()
        .with_child(
            Label::dynamic(|app_state: &AppState, _env| folder_path_text(app_state))
                .with_text_color(theme::PLACEHOLDER_COLOR),
        )
        .with_default_spacer()
        .with_child(build_answer_buttons())
        .padding(CARD_PADDING)
        .fix_width(CARD_WIDTH)
        .background(theme::WINDOW_BACKGROUND_COLOR)
        .border(theme::BORDER_LIGHT, CARD_BORDER_WIDTH)
        .rounded(CARD_CORNER_RADIUS)
}

/// Builds the layer that dims the whole page and asks the user to confirm the pending removal
/// from the recent folders. It takes no space and lets every click through while nothing waits
/// for a confirmation.
pub fn confirmation_overlay() -> impl Widget<AppState> {
    Either::new(
        |app_state: &AppState, _env| app_state.pending_confirmation.is_some(),
        build_question_card()
            .center()
            .background(DIM_COLOR)
            .expand(),
        SizedBox::empty(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modal::state::{language::Language, pending_confirmation::PendingConfirmation};

    fn english_state_waiting_for(confirmation: Option<PendingConfirmation>) -> AppState {
        let mut app_state = AppState::new();
        app_state.language = Language::English;
        app_state.pending_confirmation = confirmation;
        app_state
    }

    #[test]
    fn nothing_is_asked_without_a_pending_confirmation() {
        let app_state = english_state_waiting_for(None);

        assert_eq!(question_text(&app_state), "");
        assert_eq!(folder_path_text(&app_state), "");
    }

    #[test]
    fn removing_one_folder_asks_about_that_folder() {
        let app_state = english_state_waiting_for(Some(PendingConfirmation::RemoveRecentFolder(
            "/diary".to_string(),
        )));

        assert_eq!(
            question_text(&app_state),
            translate(Language::English, "page-login-confirmRemoveRecentFolder")
        );
        assert_eq!(folder_path_text(&app_state), "/diary");
    }

    #[test]
    fn clearing_the_list_names_no_folder() {
        let app_state = english_state_waiting_for(Some(PendingConfirmation::ClearRecentFolders));

        assert_eq!(
            question_text(&app_state),
            translate(Language::English, "page-login-confirmClearRecentFolders")
        );
        assert_eq!(folder_path_text(&app_state), "");
    }
}
