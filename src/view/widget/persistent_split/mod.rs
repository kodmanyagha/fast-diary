pub mod geometry;

use druid::{
    theme, BoxConstraints, Cursor, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, PaintCtx, Point, Rect, RenderContext, Size, UpdateCtx, Widget, WidgetPod,
};

use crate::consts::druid_selector;

use geometry::SplitGeometry;

const DEFAULT_BAR_SIZE: f64 = 3.0;
const MIN_BAR_AREA: f64 = 6.0;

/// Two panes side by side that are divided by a draggable bar. The position of the bar is a
/// fraction of the width that is read from and written to the data, and the settings are saved
/// whenever the bar is released.
pub struct PersistentSplit<T, L> {
    first: WidgetPod<T, Box<dyn Widget<T>>>,
    second: WidgetPod<T, Box<dyn Widget<T>>>,
    ratio_lens: L,
    bar_size: f64,
    min_first_width: f64,
    min_second_width: f64,
    grab_offset: f64,
    is_bar_hovered: bool,
}

impl<T: Data, L: Lens<T, f64>> PersistentSplit<T, L> {
    pub fn columns(
        first: impl Widget<T> + 'static,
        second: impl Widget<T> + 'static,
        ratio_lens: L,
    ) -> Self {
        Self {
            first: WidgetPod::new(Box::new(first)),
            second: WidgetPod::new(Box::new(second)),
            ratio_lens,
            bar_size: DEFAULT_BAR_SIZE,
            min_first_width: 0.0,
            min_second_width: 0.0,
            grab_offset: 0.0,
            is_bar_hovered: false,
        }
    }

    pub fn bar_size(mut self, bar_size: f64) -> Self {
        self.bar_size = bar_size;
        self
    }

    pub fn min_widths(mut self, first: f64, second: f64) -> Self {
        self.min_first_width = first;
        self.min_second_width = second;
        self
    }

    fn geometry(&self, total_width: f64) -> SplitGeometry {
        SplitGeometry {
            total_width,
            bar_area: self.bar_size.max(MIN_BAR_AREA),
            min_first_width: self.min_first_width,
            min_second_width: self.min_second_width,
        }
    }

    fn ratio(&self, data: &T) -> f64 {
        self.ratio_lens.with(data, |ratio| *ratio)
    }

    fn set_hovered(&mut self, ctx: &mut EventCtx, is_hovered: bool) {
        if is_hovered != self.is_bar_hovered {
            self.is_bar_hovered = is_hovered;
            if is_hovered {
                ctx.set_cursor(&Cursor::ResizeLeftRight);
            } else {
                ctx.clear_cursor();
            }
        }
    }

    fn handle_bar_event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T) {
        let geometry = self.geometry(ctx.size().width);
        let ratio = self.ratio(data);

        match event {
            Event::MouseDown(mouse)
                if mouse.button.is_left() && geometry.bar_hit_test(ratio, mouse.pos.x) =>
            {
                ctx.set_handled();
                ctx.set_active(true);
                self.grab_offset = mouse.pos.x - geometry.first_width(ratio);
                self.set_hovered(ctx, true);
            }
            Event::MouseUp(mouse) if mouse.button.is_left() && ctx.is_active() => {
                ctx.set_handled();
                ctx.set_active(false);
                let is_hovered = ctx.is_hot() && geometry.bar_hit_test(ratio, mouse.pos.x);
                self.set_hovered(ctx, is_hovered);
                ctx.submit_command(druid_selector::SETTINGS_SAVE);
            }
            Event::MouseMove(mouse) if ctx.is_active() => {
                let dragged_ratio = geometry.ratio_for_first_width(mouse.pos.x - self.grab_offset);
                self.ratio_lens
                    .with_mut(data, |ratio| *ratio = dragged_ratio);
                ctx.request_layout();
            }
            Event::MouseMove(mouse) => {
                let is_hovered = ctx.is_hot() && geometry.bar_hit_test(ratio, mouse.pos.x);
                self.set_hovered(ctx, is_hovered);
            }
            _ => {}
        }
    }
}

impl<T: Data, L: Lens<T, f64>> Widget<T> for PersistentSplit<T, L> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if self.first.is_active() {
            self.first.event(ctx, event, data, env);
            if ctx.is_handled() {
                return;
            }
        }
        if self.second.is_active() {
            self.second.event(ctx, event, data, env);
            if ctx.is_handled() {
                return;
            }
        }

        self.handle_bar_event(ctx, event, data);

        if !self.first.is_active() {
            self.first.event(ctx, event, data, env);
        }
        if !self.second.is_active() {
            self.second.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.first.lifecycle(ctx, event, data, env);
        self.second.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if self.ratio(old_data) != self.ratio(data) {
            ctx.request_layout();
        }

        self.first.update(ctx, data, env);
        self.second.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = bc.max();
        let geometry = self.geometry(size.width);
        let first_width = geometry.first_width(self.ratio(data));
        let second_width = geometry.second_width(self.ratio(data));

        let first_bc = BoxConstraints::new(
            Size::new(first_width, bc.min().height),
            Size::new(first_width, bc.max().height),
        );
        let second_bc = BoxConstraints::new(
            Size::new(second_width, bc.min().height),
            Size::new(second_width, bc.max().height),
        );

        let first_size = self.first.layout(ctx, &first_bc, data, env);
        let second_size = self.second.layout(ctx, &second_bc, data, env);

        self.first.set_origin(ctx, Point::ORIGIN);
        self.second
            .set_origin(ctx, Point::new(first_size.width + geometry.bar_area, 0.0));

        let paint_rect = self.first.paint_rect().union(self.second.paint_rect());
        ctx.set_paint_insets(paint_rect - size.to_rect());

        Size::new(size.width, first_size.height.max(second_size.height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let size = ctx.size();
        let geometry = self.geometry(size.width);
        let (bar_start, _) = geometry.bar_edges(self.ratio(data));
        let inset = (geometry.bar_area - self.bar_size) / 2.0;
        let bar = Rect::new(
            bar_start + inset,
            0.0,
            bar_start + inset + self.bar_size,
            size.height,
        );

        ctx.fill(bar, &env.get(theme::BORDER_LIGHT));
        self.first.paint(ctx, data, env);
        self.second.paint(ctx, data, env);
    }
}
