#[cfg(target_os = "linux")]
mod gtk_drop;
#[cfg(windows)]
mod windows_drop;

use std::path::PathBuf;

use druid::{Point, Target, WindowHandle};

use crate::{consts::druid_selector, utils::event_sink::get_event_sink};

/// Files dropped onto the window, together with the point they were dropped at in window
/// coordinates, when the platform tells it.
#[derive(Debug, Clone, PartialEq)]
pub struct DroppedFiles {
    pub paths: Vec<PathBuf>,
    pub window_point: Option<Point>,
}

/// Hands the files dropped onto the window to the app, which inserts their images into the
/// opened diary where they were dropped, just as if they had been pasted there.
fn deliver_dropped_files(paths: Vec<PathBuf>, window_point: Option<Point>) {
    if paths.is_empty() {
        return;
    }
    let delivery = get_event_sink().map(|sink| {
        sink.submit_command(
            druid_selector::FILES_DROPPED,
            DroppedFiles {
                paths,
                window_point,
            },
            Target::Global,
        )
    });
    if let Some(Err(err)) = delivery {
        tracing::error!("Could not deliver the dropped files: {err}");
    }
}

/// Lets files be dragged from the file manager and dropped onto the window of `handle`. druid
/// has no drag and drop of its own, so this reaches the native window directly, which is done
/// for GTK on Linux and for Windows; elsewhere dropping stays unsupported.
pub fn accept_dropped_files(handle: &WindowHandle) {
    #[cfg(target_os = "linux")]
    gtk_drop::accept_dropped_files(handle, deliver_dropped_files);
    #[cfg(windows)]
    windows_drop::accept_dropped_files(handle, deliver_dropped_files);
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = (handle, deliver_dropped_files);
    }
}
