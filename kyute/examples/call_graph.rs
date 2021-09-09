use kyute::{composable, widget::Flex, Context, Data, Event, Measurements, Widget, WidgetDelegate, BoxConstraints, Environment, Rect, LayoutCtx, Layout, PaintCtx};
use kyute_shell::platform::Platform;
use std::sync::Arc;
use kyute::widget::Axis;

struct EventCtx;

struct Window;
impl WidgetDelegate for Window {
    fn layout(&self, ctx: &mut kyute::LayoutCtx, constraints: &BoxConstraints, env: &Environment) -> kyute::Layout {
        todo!()
    }

    fn paint(&self, ctx: &mut kyute::PaintCtx, bounds: Rect, env: &Environment) {
        todo!()
    }
}

#[composable(uncached)]
fn root() -> Widget<Window> {
    window()
}

#[composable(uncached)]
fn window() -> Widget<Window> {
    vbox();
    Widget::new(Window)
}

#[composable(uncached)]
fn vbox() -> Widget<Flex> {
    let mut vbox = Flex::new(Axis::Vertical);
    vbox.push(button("hello".into()));
    vbox.push(button("world".into()));
    Widget::new(vbox)
}

#[derive(Copy, Clone, Debug)]
struct Button;

impl WidgetDelegate for Button {
    fn layout(&self, ctx: &mut LayoutCtx, constraints: &BoxConstraints, env: &Environment) -> Layout {
        todo!()
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        todo!()
    }
}

impl Data for Button {
    fn same(&self, other: &Self) -> bool {
        true
    }
}

#[composable(uncached)]
fn button(label: Arc<str>) -> Widget<Button> {
    // a state entry is created within Context::cache, so this will be added as a dependency of the cache entry
    //let hovered = Context::cache((), |_| false);
    Widget::new(Button)
}

fn main() {
    let platform = Platform::new();

    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        //.with_level(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        //.with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    root();
}
