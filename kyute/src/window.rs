use crate::{
    composable, core2::WidgetState, event::InputState, region::Region, BoxConstraints,
    Environment, Event, EventCtx, LayoutCtx, LayoutItem, Measurements, Offset, PaintCtx, Rect,
    Size, Widget, WidgetDelegate,
};
use keyboard_types::KeyState;
use kyute_shell::{
    drawing::Color,
    platform::Platform,
    window::{PlatformWindow, WindowDrawContext},
    winit,
    winit::event::{DeviceId, VirtualKeyCode, WindowEvent},
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tracing::trace_span;
//use crate::context::State;
use crate::application::AppCtx;
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
    window_state: Arc<Mutex<WindowState>>,
    children: Vec<Widget>,
}

impl Window {
    #[composable(uncached)]
    pub fn new(window_builder: WindowBuilder, children: Vec<Widget>) -> Widget<Window> {
        // Get or create the internal widget state.
        // we might have already called this function through the same call tree:
        // in this case, it will return the same object.

        // this is rewritten automatically so that the value is written back to the cache
        // at the end of the scope. but it's otherwise accessible as an owned value.

        /*#[state]
        let mut window_state = WindowState {
            window: None,
            window_builder: Some(window_builder),
            inputs: Default::default(),
            scale_factor: 0.0,
            invalid: Default::default(),
            needs_layout: false,
        };

        // contains: widget ID
        let widget_state = WidgetState::new();

        Widget::new(
            widget_state.into(),
            Window {
                window_state: Arc::new(Mutex::new(window_state)),
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

    fn mount(&self, app_ctx: &mut AppCtx) {
        /*let s = self.window_state.lock().unwrap();
        s.window = Some(
            PlatformWindow::new(app_ctx.event_loop, s.window_builder.take().unwrap(), None)
                .unwrap(),
        );*/
    }

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
