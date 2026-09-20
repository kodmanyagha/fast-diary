use druid::{commands, widget::LabelText, Env, Menu, MenuItem, SysMods, WindowId};

use crate::{
    consts::druid_selector,
    modal::{
        app_state::{AppState, OpenFilePurpose},
        state::{
            app_pages::AppPages, diary_view_mode::DiaryViewMode, editor_mode::EditorMode,
            language::Language,
        },
    },
    utils::localization::text,
    view::page::main::main_page::folder_dialog_options,
};

fn is_main_page(app_state: &AppState) -> bool {
    app_state.page == AppPages::Main
}

fn is_diary_page(app_state: &AppState) -> bool {
    app_state.page == AppPages::Diary
}

fn has_opened_diary(app_state: &AppState) -> bool {
    is_diary_page(app_state) && app_state.current_diary.is_selected
}

fn menu_title(key: &'static str) -> LabelText<AppState> {
    text(key)
}

fn build_file_menu() -> Menu<AppState> {
    Menu::new(menu_title("menu-file"))
        .entry(
            MenuItem::new(menu_title("menu-file-new-diary"))
                .command(druid_selector::CREATE_NEW_DIARY)
                .hotkey(SysMods::Cmd, "n")
                .enabled_if(|app_state, _env| is_diary_page(app_state)),
        )
        .entry(
            MenuItem::new(menu_title("menu-file-open-folder"))
                .on_activate(|ctx, app_state, _env| {
                    app_state.open_file_purpose = OpenFilePurpose::DiaryPath;
                    ctx.submit_command(commands::SHOW_OPEN_PANEL.with(folder_dialog_options()));
                })
                .hotkey(SysMods::Cmd, "o")
                .enabled_if(|app_state, _env| is_main_page(app_state)),
        )
        .entry(
            MenuItem::new(menu_title("menu-file-save"))
                .command(druid_selector::DIARY_SAVE_CURRENT)
                .hotkey(SysMods::Cmd, "s")
                .enabled_if(|app_state, _env| has_opened_diary(app_state)),
        )
        .separator()
        .entry(
            MenuItem::new(menu_title("menu-file-close-folder"))
                .command(druid_selector::FOLDER_LOCK)
                .enabled_if(|app_state, _env| is_diary_page(app_state)),
        )
        .entry(
            MenuItem::new(menu_title("menu-file-settings"))
                .on_activate(|_ctx, app_state, _env| {
                    app_state.status_message.clear();
                    app_state.page = AppPages::Settings;
                })
                .enabled_if(|app_state, _env| is_diary_page(app_state)),
        )
        .separator()
        .entry(
            MenuItem::new(menu_title("menu-file-quit"))
                .command(commands::CLOSE_WINDOW)
                .hotkey(SysMods::Cmd, "q"),
        )
}

fn build_edit_menu() -> Menu<AppState> {
    let edit_item = |key: &'static str, command: druid::Selector| {
        MenuItem::new(menu_title(key))
            .command(command)
            .enabled_if(|app_state, _env| has_opened_diary(app_state))
    };

    Menu::new(menu_title("menu-edit"))
        .entry(edit_item("menu-edit-cut", commands::CUT))
        .entry(edit_item("menu-edit-copy", commands::COPY))
        .entry(edit_item("menu-edit-paste", commands::PASTE))
        .separator()
        .entry(edit_item("menu-edit-select-all", commands::SELECT_ALL))
}

fn diary_view_mode_item(key: &'static str, mode: DiaryViewMode) -> MenuItem<AppState> {
    MenuItem::new(menu_title(key))
        .on_activate(move |ctx, app_state, _env| {
            app_state.diary_view_mode = mode;
            ctx.submit_command(druid_selector::SETTINGS_SAVE);
        })
        .selected_if(move |app_state, _env| app_state.diary_view_mode == mode)
        .enabled_if(|app_state, _env| is_diary_page(app_state))
}

fn editor_mode_item(key: &'static str, mode: EditorMode) -> MenuItem<AppState> {
    MenuItem::new(menu_title(key))
        .on_activate(move |ctx, app_state, _env| {
            app_state.editor_mode = mode;
            ctx.submit_command(druid_selector::SETTINGS_SAVE);
        })
        .selected_if(move |app_state, _env| app_state.editor_mode == mode)
        .enabled_if(|app_state, _env| is_diary_page(app_state))
}

fn language_item(title: impl Into<LabelText<AppState>>, language: Language) -> MenuItem<AppState> {
    MenuItem::new(title)
        .on_activate(move |ctx, app_state, _env| {
            app_state.language = language;
            ctx.submit_command(druid_selector::SETTINGS_SAVE);
        })
        .selected_if(move |app_state, _env| app_state.language == language)
}

fn build_language_menu() -> Menu<AppState> {
    Menu::new(menu_title("menu-view-language"))
        .entry(language_item(
            menu_title("menu-view-language-system"),
            Language::System,
        ))
        .entry(language_item("English", Language::English))
        .entry(language_item("Türkçe", Language::Turkish))
}

fn build_view_menu() -> Menu<AppState> {
    Menu::new(menu_title("menu-view"))
        .entry(diary_view_mode_item("menu-view-list", DiaryViewMode::List))
        .entry(diary_view_mode_item(
            "menu-view-calendar",
            DiaryViewMode::Calendar,
        ))
        .separator()
        .entry(editor_mode_item("menu-view-editor", EditorMode::Edit))
        .entry(editor_mode_item("menu-view-split", EditorMode::Split))
        .entry(editor_mode_item("menu-view-preview", EditorMode::Preview))
        .separator()
        .entry(build_language_menu())
}

fn build_help_menu() -> Menu<AppState> {
    Menu::new(menu_title("menu-help"))
        .entry(MenuItem::new(menu_title("menu-help-about")).command(druid_selector::APP_SHOW_ABOUT))
}

/// Builds the menu bar of the main window.
pub fn build_main_menu(
    _window: Option<WindowId>,
    _app_state: &AppState,
    _env: &Env,
) -> Menu<AppState> {
    #[cfg(target_os = "macos")]
    let menu_bar = Menu::empty().entry(druid::platform_menus::mac::application::default());
    #[cfg(not(target_os = "macos"))]
    let menu_bar = Menu::empty();

    menu_bar
        .entry(build_file_menu())
        .entry(build_edit_menu())
        .entry(build_view_menu())
        .entry(build_help_menu())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modal::state::diary_list_item::DiaryListItem;

    fn state_on(page: AppPages, opened_diary: bool) -> AppState {
        let mut app_state = AppState::new();
        app_state.page = page;
        if opened_diary {
            app_state.current_diary = DiaryListItem::new().into();
        }
        app_state
    }

    #[test]
    fn folder_items_are_only_offered_on_the_page_they_belong_to() {
        assert!(is_main_page(&state_on(AppPages::Main, false)));
        assert!(!is_main_page(&state_on(AppPages::Diary, false)));
        assert!(is_diary_page(&state_on(AppPages::Diary, false)));
        assert!(!is_diary_page(&state_on(AppPages::Settings, false)));
    }

    #[test]
    fn editing_items_need_an_opened_diary_on_the_diary_page() {
        assert!(has_opened_diary(&state_on(AppPages::Diary, true)));
        assert!(!has_opened_diary(&state_on(AppPages::Diary, false)));
        assert!(!has_opened_diary(&state_on(AppPages::Main, true)));
    }
}
