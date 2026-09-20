use druid::{
    commands, AppDelegate, DelegateCtx, Env, Monitor, Point, Screen, WindowHandle, WindowId,
};

use crate::{config::window_settings::WindowGeometry, modal::app_state::AppState};

/// Looks after the main window: it moves the window to a fallback position when the remembered
/// position lies outside of every connected monitor, and it closes the other windows, such as the
/// about window, together with the main window so that the application ends with it.
pub struct MainWindowDelegate {
    requested: Option<WindowGeometry>,
    fallback_position: Point,
    main_window: Option<WindowId>,
}

impl MainWindowDelegate {
    pub fn new(requested: Option<WindowGeometry>, fallback_position: Point) -> Self {
        Self {
            requested,
            fallback_position,
            main_window: None,
        }
    }

    fn keep_on_a_monitor(&self, handle: &WindowHandle) {
        let monitors = Screen::get_monitors()
            .iter()
            .map(Monitor::virtual_rect)
            .collect::<Vec<_>>();

        if self
            .requested
            .is_some_and(|requested| !requested.is_visible_on(&monitors))
        {
            handle.set_position(self.fallback_position);
        }
    }
}

impl AppDelegate<AppState> for MainWindowDelegate {
    fn window_added(
        &mut self,
        id: WindowId,
        handle: WindowHandle,
        _data: &mut AppState,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        if self.main_window.is_none() {
            self.main_window = Some(id);
            self.keep_on_a_monitor(&handle);
        }
    }

    fn window_removed(
        &mut self,
        id: WindowId,
        _data: &mut AppState,
        _env: &Env,
        ctx: &mut DelegateCtx,
    ) {
        if self.main_window == Some(id) {
            ctx.submit_command(commands::CLOSE_ALL_WINDOWS);
        }
    }
}
