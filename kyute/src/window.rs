use crate::{
    core2::WidgetState, event::InputState, region::Region, BoxConstraints, Context, Environment,
    Event, EventCtx, LayoutCtx, LayoutItem, Measurements, Offset, PaintCtx, Rect, Size, Widget,
    WidgetDelegate,
};
use keyboard_types::KeyState;
use kyute_shell::{
    drawing::Color,
    platform::Platform,
    window::{PlatformWindow, WindowDrawContext},
    winit,
    winit::event::{DeviceId, VirtualKeyCode, WindowEvent},
};
use std::{sync::Arc, time::Instant};
use tracing::trace_span;
use crate::composable;
//use crate::context::State;
use kyute_shell::winit::window::WindowBuilder;

struct WindowState {
    window: Option<PlatformWindow>,
    window_builder: Option<WindowBuilder>,
    //pointer_grab: Option<WeakWidgetRef>,
    //hot: Option<WeakWidgetRef>,
    inputs: InputState,
    scale_factor: f64,
    invalid: Region,
    needs_layout: bool,
}

/// A window managed by kyute.
pub struct Window {
    //window_state: State<WindowState>,
    children: Vec<Widget>,
}

impl Window {
    #[composable(uncached)]
    pub fn new(
        window_builder: WindowBuilder,
        children: Vec<Widget>,
    ) -> Widget<Window> {

        // retained widget state: you need one to build a widget;
        // it's also how you respond to events
        let mut widget_state = Context::state(move || WidgetState::new());  // StateCell<WindowState>

        // create the window (called only once)
        /*let mut window_state = Context::state(move || {
            WindowState {
                window: None,
                window_builder: Some(window_builder),
                inputs: Default::default(),
                scale_factor: 0.0,
                invalid: Default::default(),
                needs_layout: false,
            }
        }); // StateCell<WindowState>*/

        // full mutable access to widget_state here: handle events, etc.
        // full mutable access to window_state here: update it or whatever

        /*let widget_state = widget_state.commit(); // convert to State<...>
        let window_state = window_state.commit();*/

        /*Widget::new(
            widget_state.into(),        // writes back the
            Window {
               // window_state: window_state.into(),
                children,
            },
        )*/

        todo!()
    }
}

impl WidgetDelegate for Window {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /*fn mount(&self, app_ctx: &mut AppCtx) {

    }*/

    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> LayoutItem {
        /*let (width, height): (f64, f64) = self.window_state.window.window().inner_size().into();

        let layouts: Vec<_> = self
            .children
            .iter()
            .map(|child| {
                (
                    Offset::zero(),
                    child.layout(ctx, BoxConstraints::loose(Size::new(width, height)), env),
                )
            })
            .collect();

        LayoutItem::with_children(Measurements::new(Size::new(width, height)), layouts)*/
        todo!()
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        for c in self.children.iter() {
            c.paint(ctx, bounds, env);
        }
    }
}

/*
pub struct Window<'a> {
    builder: WindowBuilder,
    contents: BoxedWidget<'a>,
    callbacks: Callbacks,
    parent_window: Option<&'a PlatformWindow>,
}

impl<'a> Window<'a> {
    pub fn new(builder: WindowBuilder) -> Window<'a> {
        Window {
            builder,
            contents: DummyWidget.boxed(),
            callbacks: Callbacks::default(),
            parent_window: None,
        }
    }
}*/
