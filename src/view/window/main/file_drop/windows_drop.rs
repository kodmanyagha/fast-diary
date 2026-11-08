use std::{
    cell::RefCell,
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    ptr,
    sync::{
        atomic::{AtomicIsize, Ordering},
        OnceLock,
    },
};

use druid::{Point, WindowHandle};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    UI::{
        Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, DragQueryPoint, HDROP},
        WindowsAndMessaging::{
            CallWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, GWLP_WNDPROC, WM_DROPFILES,
            WNDPROC,
        },
    },
};

const ALL_FILES_QUERY: u32 = u32::MAX;
const ACCEPT: i32 = 1;

static ORIGINAL_WINDOW_PROCEDURE: AtomicIsize = AtomicIsize::new(0);
static DELIVER_DROPPED_FILES: OnceLock<fn(Vec<PathBuf>, Option<Point>)> = OnceLock::new();

thread_local! {
    static DROP_WINDOW: RefCell<Option<WindowHandle>> = const { RefCell::new(None) };
}

/// Returns where in the client area the drop `drop` happened, converted from pixels into
/// druid's display points.
///
/// # Safety
/// `drop` has to be the handle of a drop that `DragFinish` has not released yet.
unsafe fn drop_point(drop: HDROP) -> Option<Point> {
    let mut point = POINT { x: 0, y: 0 };
    let is_in_client_area = unsafe { DragQueryPoint(drop, &mut point) } != 0;
    let scale = DROP_WINDOW.with(|window| {
        window
            .borrow()
            .as_ref()
            .and_then(|handle| handle.get_scale().ok())
    })?;
    is_in_client_area.then(|| {
        let (x, y) = scale.px_to_dp_xy(point.x, point.y);
        Point::new(x, y)
    })
}

/// Reads the path of every file in the drop `drop`.
///
/// # Safety
/// `drop` has to be the handle of a drop that `DragFinish` has not released yet.
unsafe fn dropped_paths(drop: HDROP) -> Vec<PathBuf> {
    let count = unsafe { DragQueryFileW(drop, ALL_FILES_QUERY, ptr::null_mut(), 0) };

    (0..count)
        .filter_map(|index| {
            let length = unsafe { DragQueryFileW(drop, index, ptr::null_mut(), 0) };
            let mut buffer = vec![0u16; usize::try_from(length).ok()? + 1];
            let copied = unsafe {
                DragQueryFileW(drop, index, buffer.as_mut_ptr(), length.saturating_add(1))
            };
            buffer
                .get(..usize::try_from(copied).ok()?)
                .filter(|name| !name.is_empty())
                .map(|name| PathBuf::from(OsString::from_wide(name)))
        })
        .collect()
}

/// Returns the window procedure that druid gave the window before files were accepted.
fn original_window_procedure() -> WNDPROC {
    let address = ORIGINAL_WINDOW_PROCEDURE.load(Ordering::Acquire);
    unsafe { std::mem::transmute::<isize, WNDPROC>(address) }
}

/// Answers `WM_DROPFILES` by handing the dropped paths over, and passes every other message on
/// to druid's own window procedure.
unsafe extern "system" fn drop_window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_DROPFILES {
        let drop = wparam as HDROP;
        let paths = unsafe { dropped_paths(drop) };
        let point = unsafe { drop_point(drop) };
        unsafe { DragFinish(drop) };
        if let Some(deliver) = DELIVER_DROPPED_FILES.get() {
            deliver(paths, point);
        }
        0
    } else {
        unsafe { CallWindowProcW(original_window_procedure(), window, message, wparam, lparam) }
    }
}

/// Makes the Win32 window of `handle` accept files dropped from Explorer and hands the paths
/// of every drop to `deliver`. The window procedure of druid is wrapped through
/// `SetWindowLongPtrW`, which needs nothing beyond `user32`.
pub fn accept_dropped_files(handle: &WindowHandle, deliver: fn(Vec<PathBuf>, Option<Point>)) {
    let RawWindowHandle::Win32(win32) = handle.raw_window_handle() else {
        return;
    };
    let window = win32.hwnd as HWND;
    if window.is_null() || DELIVER_DROPPED_FILES.set(deliver).is_err() {
        return;
    }

    let original = unsafe { GetWindowLongPtrW(window, GWLP_WNDPROC) };
    if original == 0 {
        tracing::warn!("Could not read the window procedure to accept dropped files.");
        return;
    }
    ORIGINAL_WINDOW_PROCEDURE.store(original, Ordering::Release);
    DROP_WINDOW.with(|window| window.replace(Some(handle.clone())));

    let replacement = drop_window_procedure as unsafe extern "system" fn(_, _, _, _) -> _;
    let replaced =
        unsafe { SetWindowLongPtrW(window, GWLP_WNDPROC, replacement as usize as isize) };
    if replaced == 0 {
        tracing::warn!("Could not take over the window procedure to accept dropped files.");
        return;
    }
    unsafe { DragAcceptFiles(window, ACCEPT) };
}
