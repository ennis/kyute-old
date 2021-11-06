use kyute::{
    application, composable,
    widget::{Axis, Button, Flex, Text},
    BoxConstraints, Cache, CacheInvalidationToken, Data, Environment, Event, LayoutCtx,
    Measurements, PaintCtx, Rect, Widget, WidgetDelegate, Window,
};
use kyute_shell::{platform::Platform, winit::window::WindowBuilder};
use std::sync::Arc;

#[composable(uncached)]
fn root() -> Widget<Window> {
    window()
}

#[composable(uncached)]
fn window() -> Widget<Window> {
    todo!()
}

#[composable(uncached)]
fn vbox() -> Widget<Flex> {
    todo!()
}

#[derive(Clone,Data)]
struct AppState {
    items: Arc<Vec<u32>>,
    value: f64,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            items: Arc::new(vec![]),
            value: 0.0,
        }
    }
}

#[composable]
fn ui_function() -> (Widget, CacheInvalidationToken) {
    Cache::with_state(
        || AppState::default(),
        move |app_state| {
            eprintln!("recomputing ui_function");
            let add_item_button = Button::new("Add Item");

            let mut items_row = vec![];
            items_row.push(Widget::new(add_item_button).into());
            for item in app_state.items.iter() {
                let label = Text::new(format!("{}", item));
                items_row.push(Widget::new(label).into());
            }

            let invalidation_token = Cache::get_invalidation_token();
            let widget = Widget::new(Flex::new(Axis::Vertical, items_row)).into();
            (widget, invalidation_token)
        },
    )
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

    let mut ui_cache = Cache::new();

    eprintln!("running ui_function...");
    let (widget1, invalidation_token) = ui_cache.run(ui_function);
    eprintln!("running ui_function...");
    let (widget2, invalidation_token) = ui_cache.run(ui_function);
    assert!(widget1.same(&widget2));
    ui_cache.invalidate(invalidation_token);
    eprintln!("running ui_function...");
    let (widget3, invalidation_token) = ui_cache.run(ui_function);
    assert!(!widget3.same(&widget2));

    Platform::shutdown();
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
/*#[composable]
fn item_gui(item: &mut Item) -> Widget {
    // don't modify state in closure, instead, just mark the call to `on_click` as dirty.
    Button::new("change_name").on_click(|| item.name = "Hello".into());

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
*/
