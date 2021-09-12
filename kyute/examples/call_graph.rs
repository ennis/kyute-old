use kyute::{
    composable,
    widget::{Axis, Button, Flex},
    BoxConstraints, Context, Data, Environment, Event, Layout, LayoutCtx, Measurements, PaintCtx,
    Rect, Widget, WidgetDelegate,
};
use kyute_shell::platform::Platform;
use std::sync::Arc;

struct EventCtx;

struct Window;
impl WidgetDelegate for Window {
    fn layout(
        &mut self,
        ctx: &mut kyute::LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> kyute::Layout {
        todo!()
    }

    fn paint(&self, ctx: &mut kyute::PaintCtx, layout: Layout, env: &Environment) {
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

#[composable(uncached)]
fn button(label: Arc<str>) -> Widget<Button> {
    // a state entry is created within Context::cache, so this will be added as a dependency of the cache entry
    //let hovered = Context::cache((), |_| false);
    Widget::new(Button::new(label.to_string()))
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

/*List::new(|| {
    Flex::row()
        .with_child(
            Label::new(|(_, item): &(Vector<u32>, u32), _env: &_| {
                format!("List item #{}", item)
            })
        )
        .with_child(
            Button::new("Delete")
                .on_click(|_ctx, (shared, item): &mut (Vector<u32>, u32), _env| {
                    shared.retain(|v| v != item);
                })
        )
}))
.vertical()
.lens(lens::Id.map(
    |d: &AppData| (d.right.clone(), d.right.clone()),
    |d: &mut AppData, x: (Vector<u32>, Vector<u32>)| {
        d.right = x.0
    },
)*/

// issue: how do you write a composable function that focuses "down" on some state but retains
// the ability to modify it?
// what about arbitrarily deep tree data structures?
#[composable]
fn item_gui(item: &mut Item) -> Widget {
    // don't modify state in closure, instead, just mark the call to `on_click` as dirty.
    Button::new("change_name")
        .on_click(|| item.name = "Hello".into());

    // .on_click is actually:
    // #[composable] fn on_click() -> bool { }
    // which is cached
    // in the end, the root state entry will be marked as a dependency of the revision of the button
    item.clone()
}

#[composable]
fn gui() -> Widget {
    // parent cache entry now depends on state
    let mut items = Context::state(|| Vec::new());

    // this creates a new vbox every time...
    let mut vbox = Flex::new(Axis::Vertical);

    for item in items.iter_mut() {
        // ... but this call is cached
        Context::use_id(item.unique_id, || {
            let widget = item_gui(item);
            vbox.push(widget);
        });
    }

    Widget::new(vbox).into()
}