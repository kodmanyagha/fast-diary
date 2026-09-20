use druid::{Point, Rect, Size};

use crate::modal::state::calendar_month::{DAYS_IN_WEEK, MONTHS_SHOWN};

const WEEKDAY_HEADER_HEIGHT: f64 = 20.0;
const MONTH_TITLE_HEIGHT: f64 = 22.0;
const MONTH_GAP: f64 = 6.0;
const MIN_ROW_HEIGHT: f64 = 22.0;
const MAX_ROW_HEIGHT: f64 = 30.0;
const ROW_HEIGHT_TO_CELL_WIDTH_RATIO: f64 = 0.75;
const FALLBACK_WIDTH: f64 = 280.0;

/// Sizes and positions of the weekday header, the month titles and the day cells of a calendar
/// that is `width` wide and shows months of `month_rows` weeks one below the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridGeometry {
    width: f64,
    month_rows: [usize; MONTHS_SHOWN],
}

impl GridGeometry {
    pub fn new(width: f64, month_rows: [usize; MONTHS_SHOWN]) -> Self {
        Self {
            width: if width.is_finite() {
                width
            } else {
                FALLBACK_WIDTH
            },
            month_rows,
        }
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.month_top(MONTHS_SHOWN) - MONTH_GAP)
    }

    pub fn weekday_label_rect(&self, column: usize) -> Rect {
        let cell_width = self.cell_width();

        Rect::new(
            column as f64 * cell_width,
            0.0,
            (column + 1) as f64 * cell_width,
            WEEKDAY_HEADER_HEIGHT,
        )
    }

    pub fn month_title_rect(&self, month_index: usize) -> Rect {
        let top = self.month_top(month_index);

        Rect::new(0.0, top, self.width, top + MONTH_TITLE_HEIGHT)
    }

    pub fn cell_rect(&self, month_index: usize, cell_index: usize) -> Rect {
        let (row, column) = (cell_index / DAYS_IN_WEEK, cell_index % DAYS_IN_WEEK);
        let (cell_width, row_height) = (self.cell_width(), self.row_height());
        let top = self.month_top(month_index) + MONTH_TITLE_HEIGHT + row as f64 * row_height;

        Rect::new(
            column as f64 * cell_width,
            top,
            (column + 1) as f64 * cell_width,
            top + row_height,
        )
    }

    /// Returns the month and the index of the day cell that contain `point`.
    pub fn cell_at(&self, point: Point) -> Option<(usize, usize)> {
        if point.x < 0.0 {
            return None;
        }

        let column = (point.x / self.cell_width()).floor() as usize;

        (0..MONTHS_SHOWN).find_map(|month_index| {
            let cells_top = self.month_top(month_index) + MONTH_TITLE_HEIGHT;
            let row_position = (point.y - cells_top) / self.row_height();
            let is_in_the_cells = row_position >= 0.0
                && row_position < self.month_rows[month_index] as f64
                && column < DAYS_IN_WEEK;

            is_in_the_cells.then(|| {
                (
                    month_index,
                    row_position.floor() as usize * DAYS_IN_WEEK + column,
                )
            })
        })
    }

    fn month_top(&self, month_index: usize) -> f64 {
        WEEKDAY_HEADER_HEIGHT
            + self
                .month_rows
                .iter()
                .take(month_index)
                .map(|rows| MONTH_TITLE_HEIGHT + *rows as f64 * self.row_height() + MONTH_GAP)
                .sum::<f64>()
    }

    fn cell_width(&self) -> f64 {
        self.width / DAYS_IN_WEEK as f64
    }

    fn row_height(&self) -> f64 {
        (self.cell_width() * ROW_HEIGHT_TO_CELL_WIDTH_RATIO).clamp(MIN_ROW_HEIGHT, MAX_ROW_HEIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: [usize; MONTHS_SHOWN] = [5, 6, 4];

    #[test]
    fn the_month_blocks_lie_below_the_weekday_header_one_after_the_other() {
        let geometry = GridGeometry::new(280.0, ROWS);

        assert_eq!(geometry.month_title_rect(0).origin(), Point::new(0.0, 20.0));
        assert_eq!(geometry.cell_rect(0, 0).origin(), Point::new(0.0, 42.0));
        assert_eq!(geometry.cell_rect(0, 1).x0, geometry.cell_rect(0, 0).x1);
        assert_eq!(geometry.cell_rect(0, 7).y0, geometry.cell_rect(0, 0).y1);
        assert_eq!(
            geometry.month_title_rect(1).y0,
            geometry.cell_rect(0, 34).y1 + MONTH_GAP
        );
    }

    #[test]
    fn the_calendar_ends_with_the_last_row_of_the_last_month() {
        let geometry = GridGeometry::new(280.0, ROWS);

        assert_eq!(geometry.size().width, 280.0);
        assert_eq!(geometry.size().height, geometry.cell_rect(2, 27).y1);
    }

    #[test]
    fn every_cell_of_every_month_is_found_at_its_own_center() {
        let geometry = GridGeometry::new(313.0, ROWS);

        ROWS.iter().enumerate().for_each(|(month_index, rows)| {
            (0..rows * DAYS_IN_WEEK).for_each(|cell| {
                assert_eq!(
                    geometry.cell_at(geometry.cell_rect(month_index, cell).center()),
                    Some((month_index, cell))
                );
            });
        });
    }

    #[test]
    fn points_outside_the_day_cells_hit_nothing() {
        let geometry = GridGeometry::new(280.0, ROWS);
        let size = geometry.size();

        assert_eq!(geometry.cell_at(Point::new(10.0, 5.0)), None);
        assert_eq!(geometry.cell_at(Point::new(10.0, 30.0)), None);
        assert_eq!(geometry.cell_at(Point::new(-1.0, 60.0)), None);
        assert_eq!(geometry.cell_at(Point::new(size.width + 1.0, 60.0)), None);
        assert_eq!(geometry.cell_at(Point::new(10.0, size.height + 1.0)), None);
        assert_eq!(
            geometry.cell_at(Point::new(10.0, geometry.month_title_rect(1).center().y)),
            None
        );
    }

    #[test]
    fn a_month_with_fewer_weeks_takes_less_room() {
        let short = GridGeometry::new(280.0, [4, 4, 4]);
        let long = GridGeometry::new(280.0, [6, 6, 6]);

        assert!(short.size().height < long.size().height);
    }

    #[test]
    fn row_height_stays_within_its_limits() {
        assert_eq!(GridGeometry::new(70.0, ROWS).row_height(), MIN_ROW_HEIGHT);
        assert_eq!(GridGeometry::new(2000.0, ROWS).row_height(), MAX_ROW_HEIGHT);
    }

    #[test]
    fn an_unbounded_width_falls_back_to_a_fixed_width() {
        assert_eq!(
            GridGeometry::new(f64::INFINITY, ROWS).size().width,
            FALLBACK_WIDTH
        );
    }
}
