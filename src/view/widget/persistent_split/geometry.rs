/// Where the bar of a horizontal split lies, for a split that is `total_width` wide with a bar
/// area of `bar_area` between the two panes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitGeometry {
    pub total_width: f64,
    pub bar_area: f64,
    pub min_first_width: f64,
    pub min_second_width: f64,
}

impl SplitGeometry {
    /// Returns the width of the first pane for the wanted `ratio`, kept between the minimum
    /// widths of both panes.
    pub fn first_width(&self, ratio: f64) -> f64 {
        let (lowest, highest) = self.first_width_limits();

        if self.available_width() <= f64::EPSILON {
            0.0
        } else {
            (self.available_width() * ratio)
                .round()
                .clamp(lowest, highest)
        }
    }

    pub fn second_width(&self, ratio: f64) -> f64 {
        (self.available_width() - self.first_width(ratio)).max(0.0)
    }

    /// Returns the start and the end of the bar area for the wanted `ratio`.
    pub fn bar_edges(&self, ratio: f64) -> (f64, f64) {
        let start = self.first_width(ratio);
        (start, start + self.bar_area)
    }

    pub fn bar_hit_test(&self, ratio: f64, x: f64) -> bool {
        let (start, end) = self.bar_edges(ratio);
        (start..=end).contains(&x)
    }

    /// Returns the ratio that gives the first pane `first_width`, as far as the minimum widths
    /// of the panes allow it.
    pub fn ratio_for_first_width(&self, first_width: f64) -> f64 {
        let (lowest, highest) = self.first_width_limits();

        if self.available_width() <= f64::EPSILON {
            0.5
        } else {
            first_width.clamp(lowest, highest) / self.available_width()
        }
    }

    fn available_width(&self) -> f64 {
        (self.total_width - self.bar_area).max(0.0)
    }

    fn first_width_limits(&self) -> (f64, f64) {
        let lowest = self.min_first_width;
        let highest = (self.available_width() - self.min_second_width).max(0.0);

        if lowest > highest {
            let middle = (lowest + highest) / 2.0;
            (middle, middle)
        } else {
            (lowest, highest)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEOMETRY: SplitGeometry = SplitGeometry {
        total_width: 406.0,
        bar_area: 6.0,
        min_first_width: 100.0,
        min_second_width: 150.0,
    };

    #[test]
    fn the_panes_share_the_width_without_the_bar_by_the_ratio() {
        assert_eq!(GEOMETRY.first_width(0.5), 200.0);
        assert_eq!(GEOMETRY.second_width(0.5), 200.0);
    }

    #[test]
    fn the_minimum_widths_of_both_panes_are_respected() {
        assert_eq!(GEOMETRY.first_width(0.05), 100.0);
        assert_eq!(GEOMETRY.first_width(0.95), 250.0);
        assert_eq!(GEOMETRY.second_width(0.95), 150.0);
    }

    #[test]
    fn a_narrow_split_shares_the_shortage_between_the_panes() {
        let narrow = SplitGeometry {
            total_width: 206.0,
            ..GEOMETRY
        };

        assert_eq!(narrow.first_width(0.5), 75.0);
        assert_eq!(narrow.second_width(0.5), 125.0);
    }

    #[test]
    fn the_bar_lies_right_after_the_first_pane() {
        assert_eq!(GEOMETRY.bar_edges(0.5), (200.0, 206.0));
        assert!(GEOMETRY.bar_hit_test(0.5, 203.0));
        assert!(!GEOMETRY.bar_hit_test(0.5, 150.0));
        assert!(!GEOMETRY.bar_hit_test(0.5, 250.0));
    }

    #[test]
    fn dragging_gives_the_ratio_of_the_wanted_first_width() {
        assert_eq!(GEOMETRY.ratio_for_first_width(200.0), 0.5);
        assert_eq!(GEOMETRY.ratio_for_first_width(0.0), 0.25);
        assert_eq!(GEOMETRY.ratio_for_first_width(1000.0), 0.625);
    }

    #[test]
    fn the_ratio_of_a_dragged_bar_gives_back_the_same_first_width() {
        let ratio = GEOMETRY.ratio_for_first_width(217.0);

        assert_eq!(GEOMETRY.first_width(ratio), 217.0);
    }

    #[test]
    fn a_split_without_room_gives_nothing_to_the_panes() {
        let empty = SplitGeometry {
            total_width: 4.0,
            ..GEOMETRY
        };

        assert_eq!(empty.first_width(0.5), 0.0);
        assert_eq!(empty.second_width(0.5), 0.0);
        assert_eq!(empty.ratio_for_first_width(10.0), 0.5);
    }
}
