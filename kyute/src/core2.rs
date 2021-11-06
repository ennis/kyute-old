use crate::{
    application::AppCtx,
    cache_cell::CacheCell,
    event::{InputState, PointerEvent},
    layout::LayoutItem,
    region::Region,
    BoxConstraints, Data, Environment, Event, Measurements, Offset, Point, Rect, Size,
};
use kyute_macros::composable;
use kyute_shell::{drawing::DrawContext, winit::window::WindowId};
use std::{
    cell::{Cell, RefCell},
    fmt,
    fmt::Formatter,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, Weak},
};

/// Context passed to widgets during the layout pass.
///
/// See [`Widget::layout`].
pub struct LayoutCtx {}

pub struct PaintCtx<'a> {
    draw_ctx: &'a mut DrawContext,
    window_bounds: Rect,
}

impl<'a> PaintCtx<'a> {
    /*/// Returns the window bounds of the node
    pub fn window_bounds(&self) -> Rect {
        self.window_bounds
    }*/

    /// Returns the bounds of the node.
    pub fn bounds(&self) -> Rect {
        // FIXME: is the local origin always on the top-left corner?
        Rect::new(Point::origin(), self.window_bounds.size)
    }

    ///
    pub fn is_hovering(&self) -> bool {
        todo!()
    }

    /*/// Returns the size of the node.
    pub fn size(&self) -> Size {
        self.window_bounds.size
    }

    pub fn is_hovering(&self) -> bool {
        self.hover
    }

    pub fn is_focused(&self) -> bool {
        self.focus == Some(self.node_id)
    }

    pub fn is_capturing_pointer(&self) -> bool {
        self.pointer_grab == Some(self.node_id)
    }*/
}

// PaintCtx auto-derefs to a DrawContext
impl<'a> Deref for PaintCtx<'a> {
    type Target = DrawContext;

    fn deref(&self) -> &Self::Target {
        self.draw_ctx
    }
}

impl<'a> DerefMut for PaintCtx<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.draw_ctx
    }
}

pub struct EventCtx {
    //pub(crate) focus_request: WeakWidgetRef,
}

impl EventCtx {
    pub fn enqueue_action(&mut self) {}

    /// Returns the bounds of the current widget.
    // TODO in what space?
    pub fn bounds(&self) -> Rect {
        todo!()
    }

    /// Requests a redraw of the current node and its children.
    pub fn request_redraw(&mut self) {
        todo!()
    }

    pub fn request_recomposition(&mut self) {
        todo!()
    }

    /// Requests a relayout of the current node.
    pub fn request_relayout(&mut self) {
        todo!()
    }

    /// Requests that the current node grabs all pointer events in the parent window.
    pub fn capture_pointer(&mut self) {
        todo!()
    }

    /// Returns whether the current node is capturing the pointer.
    pub fn is_capturing_pointer(&self) -> bool {
        todo!()
    }

    /// Releases the pointer grab, if the current node is holding it.
    pub fn release_pointer(&mut self) {
        todo!()
    }

    /// Acquires the focus.
    pub fn request_focus(&mut self) {
        todo!()
    }

    /// Returns whether the current node has the focus.
    pub fn has_focus(&self) -> bool {
        todo!()
    }

    /// Signals that the passed event was handled and should not bubble up further.
    pub fn set_handled(&mut self) {
        todo!()
    }

    #[must_use]
    pub fn handled(&self) -> bool {
        todo!()
    }
}

pub struct WindowPaintCtx {}

/// Internal widget state.
/// Currently empty.
#[derive(Clone)]
pub struct WidgetState {}

impl WidgetState {
    pub fn new() -> WidgetState {
        WidgetState {}
    }
}

pub struct WidgetInner<T: ?Sized> {
    // Widget retained state.
    //state: State<WidgetState>,
    /// Widget delegate
    delegate: T,
}

/// Represents a widget.
pub struct Widget<T: ?Sized = dyn WidgetDelegate>(Arc<WidgetInner<T>>);

impl<T: ?Sized> Clone for Widget<T> {
    fn clone(&self) -> Self {
        Widget(self.0.clone())
    }
}

impl<T: ?Sized+'static> Data for Widget<T> {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}

impl<T: WidgetDelegate + 'static> From<Widget<T>> for Widget {
    fn from(widget: Widget<T>) -> Self {
        Widget(widget.0)
    }
}

impl<T: ?Sized + WidgetDelegate> Widget<T> {
    /// Called to measure this widget and layout the children of this widget.
    pub fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> LayoutItem {
        // TODO cache the layout result
        self.0.delegate.layout(ctx, constraints, env)
    }

    pub fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        self.0.delegate.paint(ctx, bounds, env)
    }
}

impl<T: WidgetDelegate> Widget<T> {
    pub fn new(delegate: T) -> Widget<T> {
        Widget(Arc::new(WidgetInner { delegate }))
    }
}

/// Trait that defines the behavior of a widget.
pub trait WidgetDelegate {
    /// Implement to give a debug name to your widget. Used only for debugging.
    fn debug_name(&self) -> &str {
        "WidgetDelegate"
    }

    fn mount(&self, ctx: &mut AppCtx) {}

    /// Called to measure this widget and layout the children of this widget.
    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> LayoutItem;

    /// Called to paint the widget
    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment);

    /// Called only for native window widgets.
    fn window_paint(&self, _ctx: &mut WindowPaintCtx) {}

    /// Returns `true` if the widget is fully opaque when drawn, `false` if it is semitransparent.
    /// This is mostly used as an optimization: if a semitransparent widget needs to be redrawn,
    /// its background (and thus the parent
    fn is_opaque(&self) -> bool {
        false
    }
}
