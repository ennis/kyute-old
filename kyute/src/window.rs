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

struct WindowState {
    window: PlatformWindow,
    //pointer_grab: Option<WeakWidgetRef>,
    //hot: Option<WeakWidgetRef>,
    inputs: InputState,
    scale_factor: f64,
    invalid: Region,
    needs_layout: bool,
}

/// A window managed by kyute.
pub struct Window {
    window_state: Arc<WindowState>,
    children: Vec<Widget>,
}

impl Window {
    #[composable(uncached)]
    pub fn new(
        window_builder: winit::window::WindowBuilder,
        children: Vec<Widget>,
    ) -> Widget<Window> {

        // retained widget state
        let widget_state = WidgetState {};

        // create the window (called only once)
        /*let window_state = Context::cache((), move |_| {
            // FIXME: parent window?
            // FIXME: we cannot create the window here: due to winit's design,
            // the event loop cannot be accessed in `'static` contexts (we must use &EventLoopWindowTarget, which
            // has a closure-bound unique lifetime).
            //
            // Solutions:
            // 1. delay the creation of the window: `WidgetDelegate::mount(&AppCtx)`?
            //
            let window =
                PlatformWindow::new(Platform::instance().event_loop(), window_builder, None)
                    .unwrap();
            let ws = WindowState {
                window,
                //pointer_grab: None,
                //hot: None,
                inputs: Default::default(),
                scale_factor: 0.0,
                invalid: Default::default(),
                needs_layout: false,
            };

            Arc::new(ws)
        });*/

        // FIXME: update window properties

        todo!()

        /*// TODO update window state from parameters
        Widget::new(
            widget_state,
            Window {
                window_state,
                children,
            },
        )*/
    }
}

impl WidgetDelegate for Window {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> LayoutItem {
        let (width, height): (f64, f64) = self.window_state.window.window().inner_size().into();

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

        LayoutItem::with_children(Measurements::new(Size::new(width, height)), layouts)
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
