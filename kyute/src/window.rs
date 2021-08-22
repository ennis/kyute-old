use crate::{
    core::{
        EventCtx, FocusAction, LayoutCtx, Node, PaintCtx, RepaintRequest, Widget,
        WindowPaintCtx,
    },
    event::{Event, InputState, KeyboardEvent, PointerButton, PointerEvent, PointerEventKind},
    layout::{BoxConstraints, Measurements},
    NodeId, PhysicalSize, Point, Rect, Size,
};
use keyboard_types::KeyState;
use kyute_shell::{
    drawing::Color,
    platform::Platform,
    window::{PlatformWindow, WindowDrawContext},
    winit,
    winit::event::{DeviceId, VirtualKeyCode, WindowEvent},
};
use std::time::Instant;
use tracing::trace_span;

/// Window event callbacks.
struct Callbacks {
    on_close_requested: Option<Box<dyn Fn()>>,
    on_move: Option<Box<dyn Fn(u32, u32)>>,
    on_resize: Option<Box<dyn Fn(u32, u32)>>,
    on_focus_gained: Option<Box<dyn Fn()>>,
    on_focus_lost: Option<Box<dyn Fn()>>,
}

impl Default for Callbacks {
    fn default() -> Callbacks {
        Callbacks {
            on_close_requested: None,
            on_move: None,
            on_resize: None,
            on_focus_gained: None,
            on_focus_lost: None,
        }
    }
}

/// A window managed by kyute.
// TODO remove this, and make window handling built-in?
// move ownership of the PlatformWindow to kyute-shell?
//
// Potential issues:
// - creating a child window for 3D rendering => no platform window
pub struct WindowWidget {
    title: String,
}

impl WindowWidget {
    pub fn new() -> WindowWidget {
        WindowWidget {
            title: "".to_string(),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, new_title: impl Into<String>) {
        self.title = new_title.into();
        // TODO
        //self.window.window().set_title(&self.title);
    }
}

impl WindowWidget {
    /*fn get_pointer_event_target(
        &self,
        pointer_position: Point,
        children: &[Node],
    ) -> Option<NodeId> {
        self.focus
            .pointer_grab
            .or_else(|| children.iter().find_map(|c| c.hit_test(pointer_position)))
    }*/
}

impl Widget for WindowWidget {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /*/// Handles raw window events.
    fn window_event(
        &mut self,
        ctx: &mut WindowEventCtx,
        children: &mut [Node],
        winit_event: &kyute_shell::winit::event::WindowEvent,
    ) {
    }*/

    fn event(&mut self, ctx: &mut EventCtx, children: &mut [Node], event: &Event) {
        // this node doesn't receive processed events
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut [Node],
        constraints: &BoxConstraints,
    ) -> Measurements {
        // layout children
        let window_size = ctx.parent_window_size();
        for child in children.iter_mut() {
            child.layout(ctx, &BoxConstraints::loose(window_size));
        }
        // A child window doesn't take any space in its parent
        Measurements::default()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, children: &mut [Node], bounds: Rect) {
        for c in children.iter_mut() {
            c.paint(ctx);
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
