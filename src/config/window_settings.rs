use druid::{Data, Point, Rect, Size, WindowState};
use serde::{Deserialize, Serialize};

pub const MIN_WINDOW_SIZE: Size = Size::new(800.0, 600.0);
pub const DEFAULT_WINDOW_SIZE: Size = Size::new(800.0, 600.0);
pub const FALLBACK_NORMAL_SIZE: Size = Size::new(1000.0, 700.0);

const LARGEST_WINDOW_LENGTH: f64 = 16_384.0;
const MIN_VISIBLE_SIZE: Size = Size::new(100.0, 50.0);

/// Position and size of a window that is neither maximized nor minimized.
#[derive(Debug, Clone, Copy, PartialEq, Data, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl WindowGeometry {
    pub fn new(position: Point, size: Size) -> Self {
        Self {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        }
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Tells whether enough of the window lies on one of `monitors` to grab and move it. Without
    /// any known monitor the window is assumed to be visible.
    pub fn is_visible_on(&self, monitors: &[Rect]) -> bool {
        let window = Rect::from_origin_size(self.origin(), self.size());

        monitors.is_empty()
            || monitors.iter().any(|monitor| {
                let overlap = monitor.intersect(window);
                overlap.width() >= MIN_VISIBLE_SIZE.width
                    && overlap.height() >= MIN_VISIBLE_SIZE.height
            })
    }

    fn is_finite(&self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|value| value.is_finite())
    }
}

/// How the window has to be created so that it looks like it did when it was closed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaunchPlan {
    pub size: Size,
    pub position: Option<Point>,
    pub maximized: bool,
}

/// The remembered state of the main window. `normal` holds the last geometry the window had while
/// it was neither maximized nor minimized, so that leaving the maximized state returns to it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Data, Serialize, Deserialize)]
pub struct WindowSettings {
    #[serde(default)]
    pub normal: Option<WindowGeometry>,
    #[serde(default)]
    pub maximized: bool,
}

impl WindowSettings {
    /// Works out how to create the window. When the window was closed maximized but its normal
    /// geometry is unknown, it gets [`FALLBACK_NORMAL_SIZE`] to return to.
    pub fn launch_plan(&self) -> LaunchPlan {
        let known_normal = self.normal.filter(WindowGeometry::is_finite);
        let size_without_history = if self.maximized {
            FALLBACK_NORMAL_SIZE
        } else {
            DEFAULT_WINDOW_SIZE
        };

        LaunchPlan {
            size: known_normal.map_or(size_without_history, |normal| {
                Self::clamp_size(normal.size())
            }),
            position: known_normal.map(|normal| normal.origin()),
            maximized: self.maximized,
        }
    }

    /// Returns the settings after the window was seen in `state` at `position` with `size`. A
    /// maximized window keeps the remembered normal geometry and a minimized one changes nothing.
    pub fn observe(&self, state: WindowState, position: Point, size: Size) -> Self {
        match state {
            WindowState::Minimized => *self,
            WindowState::Maximized => Self {
                maximized: true,
                ..*self
            },
            WindowState::Restored => Self {
                normal: Some(WindowGeometry::new(position, size)),
                maximized: false,
            },
        }
    }

    fn clamp_size(size: Size) -> Size {
        Size::new(
            size.width
                .clamp(MIN_WINDOW_SIZE.width, LARGEST_WINDOW_LENGTH),
            size.height
                .clamp(MIN_WINDOW_SIZE.height, LARGEST_WINDOW_LENGTH),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(x: f64, y: f64, width: f64, height: f64) -> WindowGeometry {
        WindowGeometry::new(Point::new(x, y), Size::new(width, height))
    }

    fn settings(normal: Option<WindowGeometry>, maximized: bool) -> WindowSettings {
        WindowSettings { normal, maximized }
    }

    #[test]
    fn first_launch_uses_the_default_size_and_lets_the_system_place_it() {
        let plan = WindowSettings::default().launch_plan();

        assert_eq!(plan.size, DEFAULT_WINDOW_SIZE);
        assert_eq!(plan.position, None);
        assert!(!plan.maximized);
    }

    #[test]
    fn the_remembered_normal_geometry_is_restored() {
        let plan = settings(Some(geometry(40.0, 50.0, 1200.0, 900.0)), false).launch_plan();

        assert_eq!(plan.size, Size::new(1200.0, 900.0));
        assert_eq!(plan.position, Some(Point::new(40.0, 50.0)));
    }

    #[test]
    fn a_window_closed_maximized_opens_maximized_and_keeps_its_normal_geometry() {
        let plan = settings(Some(geometry(40.0, 50.0, 1200.0, 900.0)), true).launch_plan();

        assert!(plan.maximized);
        assert_eq!(plan.size, Size::new(1200.0, 900.0));
    }

    #[test]
    fn a_maximized_window_without_a_normal_geometry_returns_to_the_fallback_size() {
        let plan = settings(None, true).launch_plan();

        assert!(plan.maximized);
        assert_eq!(plan.size, FALLBACK_NORMAL_SIZE);
        assert_eq!(plan.position, None);
    }

    #[test]
    fn an_unusable_normal_geometry_counts_as_unknown() {
        let broken = settings(Some(geometry(f64::NAN, 0.0, 900.0, 700.0)), true).launch_plan();

        assert_eq!(broken.size, FALLBACK_NORMAL_SIZE);
        assert_eq!(broken.position, None);
    }

    #[test]
    fn remembered_sizes_are_kept_within_sensible_limits() {
        let tiny = settings(Some(geometry(0.0, 0.0, 10.0, 10.0)), false).launch_plan();
        let huge = settings(Some(geometry(0.0, 0.0, 1e9, 1e9)), false).launch_plan();

        assert_eq!(tiny.size, MIN_WINDOW_SIZE);
        assert_eq!(
            huge.size,
            Size::new(LARGEST_WINDOW_LENGTH, LARGEST_WINDOW_LENGTH)
        );
    }

    #[test]
    fn a_restored_window_becomes_the_normal_geometry() {
        let observed = WindowSettings::default().observe(
            WindowState::Restored,
            Point::new(10.0, 20.0),
            Size::new(900.0, 700.0),
        );

        assert_eq!(
            observed,
            settings(Some(geometry(10.0, 20.0, 900.0, 700.0)), false)
        );
    }

    #[test]
    fn a_maximized_window_keeps_the_last_normal_geometry() {
        let before = settings(Some(geometry(10.0, 20.0, 900.0, 700.0)), false);

        let observed = before.observe(
            WindowState::Maximized,
            Point::ZERO,
            Size::new(1920.0, 1080.0),
        );

        assert_eq!(observed, settings(before.normal, true));
    }

    #[test]
    fn leaving_the_maximized_state_records_the_new_normal_geometry() {
        let before = settings(Some(geometry(10.0, 20.0, 900.0, 700.0)), true);

        let observed = before.observe(
            WindowState::Restored,
            Point::new(30.0, 40.0),
            Size::new(1000.0, 800.0),
        );

        assert_eq!(
            observed,
            settings(Some(geometry(30.0, 40.0, 1000.0, 800.0)), false)
        );
    }

    #[test]
    fn a_minimized_window_changes_nothing() {
        let before = settings(Some(geometry(10.0, 20.0, 900.0, 700.0)), true);

        let observed = before.observe(WindowState::Minimized, Point::ZERO, Size::ZERO);

        assert_eq!(observed, before);
    }

    #[test]
    fn a_window_is_visible_when_enough_of_it_is_on_a_monitor() {
        let monitors = [Rect::new(0.0, 0.0, 1920.0, 1080.0)];

        assert!(geometry(100.0, 100.0, 800.0, 600.0).is_visible_on(&monitors));
        assert!(geometry(1800.0, 500.0, 800.0, 600.0).is_visible_on(&monitors));
    }

    #[test]
    fn a_window_that_is_off_every_monitor_is_not_visible() {
        let monitors = [Rect::new(0.0, 0.0, 1920.0, 1080.0)];

        assert!(!geometry(3000.0, 100.0, 800.0, 600.0).is_visible_on(&monitors));
        assert!(!geometry(1900.0, 500.0, 800.0, 600.0).is_visible_on(&monitors));
        assert!(!geometry(100.0, 1070.0, 800.0, 600.0).is_visible_on(&monitors));
    }

    #[test]
    fn any_window_is_visible_when_no_monitor_is_known() {
        assert!(geometry(9999.0, 9999.0, 800.0, 600.0).is_visible_on(&[]));
    }

    #[test]
    fn a_window_on_a_second_monitor_is_visible() {
        let monitors = [
            Rect::new(0.0, 0.0, 1920.0, 1080.0),
            Rect::new(1920.0, 0.0, 3840.0, 1080.0),
        ];

        assert!(geometry(2500.0, 100.0, 800.0, 600.0).is_visible_on(&monitors));
    }
}
