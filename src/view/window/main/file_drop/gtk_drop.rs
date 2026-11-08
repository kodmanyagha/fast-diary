use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use druid::{Point, Scalable, WindowHandle};
use gtk::{
    gdk::DragAction,
    prelude::{BinExt, Cast, ContainerExt, GtkWindowExt, ObjectExt, WidgetExt, WidgetExtManual},
    ApplicationWindow, DestDefaults, DrawingArea, TargetEntry, TargetFlags, Widget, WindowType,
};

use crate::clipboard::path_text::paths_in_text;

const URI_LIST_TARGET: &str = "text/uri-list";
const URI_LIST_INFO: u32 = 0;

static IS_DROP_TARGET_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Returns the top-level application windows that GTK knows of. druid keeps its GTK window to
/// itself, but it lives in this process, so GTK lists it; menus show up as popups and are left
/// out.
fn application_windows() -> Vec<ApplicationWindow> {
    gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|widget| widget.downcast::<ApplicationWindow>().ok())
        .filter(|window| window.window_type() == WindowType::Toplevel)
        .collect()
}

/// Returns the drawing area that druid paints the content of `window` into, below its menu.
fn content_area_of(window: &ApplicationWindow) -> Option<Widget> {
    window
        .child()?
        .downcast::<gtk::Container>()
        .ok()?
        .children()
        .into_iter()
        .find(|child| child.is::<DrawingArea>())
}

/// Converts the point (`x`, `y`) of `window` into druid's window coordinates, which start at
/// the top left corner of the content area and are measured in display points.
fn druid_window_point(
    window: &ApplicationWindow,
    handle: &WindowHandle,
    x: i32,
    y: i32,
) -> Option<Point> {
    let content_area = content_area_of(window)?;
    let (content_x, content_y) = window.translate_coordinates(&content_area, x, y)?;
    let scale = handle.get_scale().ok()?;
    Some(Point::new(f64::from(content_x), f64::from(content_y)).to_dp(scale))
}

/// Makes the GTK window of druid's main window accept files dropped from other apps, and hands
/// the paths of every drop to `deliver`. It has to run when the main window has just been
/// added, while it is the only application window, and does its work only once.
pub fn accept_dropped_files(handle: &WindowHandle, deliver: fn(Vec<PathBuf>, Option<Point>)) {
    if IS_DROP_TARGET_INSTALLED.swap(true, Ordering::AcqRel) {
        return;
    }
    let targets = [TargetEntry::new(
        URI_LIST_TARGET,
        TargetFlags::OTHER_APP,
        URI_LIST_INFO,
    )];
    let windows = application_windows();
    if windows.is_empty() {
        tracing::warn!("No GTK window was found to accept dropped files.");
    }

    windows.iter().for_each(|window| {
        window.drag_dest_set(DestDefaults::ALL, &targets, DragAction::COPY);
        let handle = handle.clone();
        window.connect_drag_data_received(move |window, _, x, y, selection, _, _| {
            let uris = selection
                .uris()
                .iter()
                .map(|uri| uri.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            deliver(
                paths_in_text(&uris, None).unwrap_or_default(),
                druid_window_point(window, &handle, x, y),
            );
        });
    });
}
