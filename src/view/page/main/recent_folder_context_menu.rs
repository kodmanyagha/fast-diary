use druid::{widget::Controller, Env, Event, EventCtx, Menu, MenuItem, Widget};

use crate::{
    modal::{app_state::AppState, state::pending_confirmation::PendingConfirmation},
    utils::localization::text,
};

/// Opens a menu that offers to remove the right clicked row from the recent folders. The list
/// selects the row under any mouse button, so the clicked folder is the selected one.
pub struct RecentFolderContextMenu;

impl<W: Widget<AppState>> Controller<AppState, W> for RecentFolderContextMenu {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        app_state: &mut AppState,
        env: &Env,
    ) {
        child.event(ctx, event, app_state, env);

        match event {
            Event::MouseUp(mouse) if mouse.button.is_right() && ctx.is_hot() => {
                if let Some(folder_path) = app_state.selected_recent_folder().cloned() {
                    ctx.show_context_menu(build_recent_folder_menu(folder_path), mouse.window_pos);
                }
            }
            _ => {}
        }
    }
}

/// Builds the context menu of the recent folder `folder_path`.
fn build_recent_folder_menu(folder_path: String) -> Menu<AppState> {
    Menu::empty().entry(
        MenuItem::new(text("menu-recent-folder-remove")).on_activate(
            move |_ctx, app_state: &mut AppState, _env| {
                app_state.pending_confirmation =
                    Some(PendingConfirmation::RemoveRecentFolder(folder_path.clone()));
            },
        ),
    )
}
