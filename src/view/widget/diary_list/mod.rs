/*
use druid::{
    widget::{Label, List, Scroll},
    Data, LocalizedString, Widget, WidgetExt,
};

use crate::modal::app_data::AppData;

struct DiaryList<T> {
    list: List<T>,
}

impl<T: Data> DiaryList<T> {
    pub fn new<W: Widget<T> + 'static>(closure: impl Fn() -> W + 'static) -> Self {
        Self {
            list: List::new(closure),
        }
    }

    pub fn invoke_me(&self) {
        log::error!("DiaryList::invoke_me invoked");
    }
}

impl<T: Data> Widget<AppData> for DiaryList<T> {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut AppData,
        env: &druid::Env,
    ) {
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &AppData,
        env: &druid::Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &AppData,
        data: &AppData,
        env: &druid::Env,
    ) {
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &AppData,
        env: &druid::Env,
    ) -> druid::Size {
        self.list.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &AppData, env: &druid::Env) {}
}

*/
