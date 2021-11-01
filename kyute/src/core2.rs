use crate::{
    application::AppCtx,
    bloom::Bloom,
    event::{InputState, LifecycleEvent, PointerEvent},
    region::Region,
    util::Counter,
    BoxConstraints, Data, Environment, Event, InternalEvent, Measurements, Model, Offset, Point,
    Rect, Size,
};
use kyute_shell::{
    drawing::DrawContext,
    winit::{event::WindowEvent, window::WindowId},
};
use std::{
    fmt,
    fmt::Formatter,
    hash::{Hash, Hasher},
    marker::PhantomData,
    num::NonZeroU64,
    ops::{Deref, DerefMut},
};

/// ID of a node in the tree.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct WidgetId(NonZeroU64);

impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04X}", self.0.get())
    }
}

static WIDGET_ID_COUNTER: Counter = Counter::new();

impl WidgetId {
    /// Generates a new node ID unique for this execution of the program.
    pub fn next() -> WidgetId {
        let val = WIDGET_ID_COUNTER.next_nonzero();
        WidgetId(val)
    }
}

/// Context passed to widgets during the layout pass.
///
/// See [`Widget::layout`].
pub struct LayoutCtx<'ctx> {
    app_ctx: &'ctx mut AppCtx,
}

pub struct PaintCtx<'a> {
    draw_ctx: &'a mut DrawContext,
    window_bounds: Rect,
}

impl<'a> PaintCtx<'a> {
    pub fn new(draw_ctx: &'a mut DrawContext, window_bounds: Rect) -> PaintCtx<'a> {
        PaintCtx {
            draw_ctx,
            window_bounds,
        }
    }
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

pub struct EventCtx<'a, 'ctx> {
    pub(crate) app_ctx: &'ctx mut AppCtx,
    pub(crate) state: Option<&'a mut WidgetState>,
}

impl<'a, 'ctx> EventCtx<'a, 'ctx> {
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

pub struct UpdateCtx<'a, 'ctx> {
    app_ctx: &'ctx mut AppCtx,
    state: Option<&'a mut WidgetState>,
    children_changed: bool,
}

impl<'a, 'ctx> UpdateCtx<'a, 'ctx> {
    pub fn children_changed(&mut self) {
        self.children_changed = true;
    }
}

/// Trait implemented by widgets.
pub trait Widget<T: Model> {
    /// Implement to give a debug name to your widget. Used only for debugging.
    fn debug_name(&self) -> &str {
        "Widget"
    }

    /// Propagates an event to the widget hierarchy.
    // FIXME: right now it can only return one change
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T) -> Option<T::Change> {
        None
    }

    fn lifecycle(&mut self, ctx: &mut EventCtx, event: &LifecycleEvent, data: &mut T);

    /// Propagates a data update.
    fn update(&mut self, ctx: &mut UpdateCtx, data: &mut T, change: &T::Change);

    /// Called to measure this widget and layout the children of this widget.
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        data: &mut T,
        env: &Environment,
    ) -> Measurements;

    /// Called to paint the widget
    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, data: &mut T, env: &Environment);
}

/// Boxed widget impls.
impl<T, W> Widget<T> for Box<W>
where
    T: Model,
    W: Widget<T> + ?Sized,
{
    fn debug_name(&self) -> &str {
        Widget::debug_name(&**self)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T) -> Option<T::Change> {
        Widget::event(&mut **self, ctx, event, data)
    }

    fn lifecycle(&mut self, ctx: &mut EventCtx, event: &LifecycleEvent, data: &mut T) {
        Widget::lifecycle(&mut **self, ctx, event, data)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, data: &mut T, change: &T::Change) {
        Widget::update(&mut **self, ctx, data, change)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        data: &mut T,
        env: &Environment,
    ) -> Measurements {
        Widget::layout(&mut **self, ctx, constraints, data, env)
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, data: &mut T, env: &Environment) {
        Widget::paint(&**self, ctx, bounds, data, env)
    }
}

/// Common internal state of widgets.
pub(crate) struct WidgetState {
    /// Whether this widget received the `Initialize` event.
    initialized: bool,

    /// Unique widget ID, assigned on creation.
    id: WidgetId,

    /// Visual position within the parent.
    offset: Offset,

    /// Measurements returned by the last call to `Widget::layout`
    measurements: Measurements,

    /// A bloom filter used to filter out children that don't belong to this widget.
    ///
    /// It should be updated whenever a child is added to or removed from this widget.
    child_filter: Bloom<WidgetId>,
}

/// A container for a widget in the hierarchy.
pub struct WidgetPod<T, W = Box<dyn Widget<T>>> {
    /// Internal widget state.
    ///
    /// Split into a separate struct for easier split borrowing.
    pub(crate) state: WidgetState,

    /// The widget itself.
    pub(crate) inner: W,

    _phantom: PhantomData<*const T>,
}

impl<T: Model, W: Widget<T>> WidgetPod<T, W> {
    /// Creates a new `WidgetPod` wrapping the given widget.
    pub fn new(inner: W) -> WidgetPod<T, W> {
        WidgetPod {
            state: WidgetState {
                initialized: false,
                id: WidgetId::next(),
                offset: Default::default(),
                measurements: Default::default(),
                child_filter: Default::default(),
            },
            inner,
            _phantom: PhantomData,
        }
    }

    /// Sets the layout position of this widget within its parent.
    /// Should be called by widgets in `Widget::layout`.
    pub fn set_child_offset(&mut self, offset: Offset) {
        self.state.offset = offset;
    }

    /// Propagates an event to the wrapped widget.
    pub fn event(
        &mut self,
        parent_ctx: &mut EventCtx,
        event: &Event,
        data: &mut T,
    ) -> Option<T::Change> {
        // Handle internal events (routing mostly)
        match event {
            Event::Internal(InternalEvent::RouteWindowEvent { target, event }) => {
                if *target == self.state.id {
                    return self.event(parent_ctx, &Event::WindowEvent(event.clone()), data);
                }
                if !self.state.child_filter.may_contain(target) {
                    return None;
                }
            }
            Event::Internal(InternalEvent::RouteRedrawRequest(target)) => {
                if *target == self.state.id {
                    return self.event(parent_ctx, &Event::WindowRedrawRequest, data);
                }
                if !self.state.child_filter.may_contain(target) {
                    return None;
                }
            }
            _ => {}
        }

        // Propagate
        let mut ctx = EventCtx {
            app_ctx: parent_ctx.app_ctx,
            state: Some(&mut self.state),
        };
        self.inner.event(&mut ctx, event, data)
    }

    /// Propagates a lifecycle event to the wrapped widget.
    pub fn lifecycle(&mut self, parent_ctx: &mut EventCtx, event: &LifecycleEvent, data: &mut T) {
        match event {
            LifecycleEvent::UpdateChildFilter => {
                if let Some(state) = parent_ctx.state.as_deref_mut() {
                    state.child_filter.add(&self.state.id);
                    state.child_filter.extend(&self.state.child_filter);
                } else {
                    tracing::warn!("UpdateChildFilter sent to root widget");
                }
                // No need to propagate
                return;
            }
            LifecycleEvent::RouteInitialize => {
                if !self.state.initialized {
                    // send initialize event
                    // FIXME: with this API, an `Initialize` event can modify data, but the potential resulting change is ignored.
                    // Replace this by a specialized method (e.g. `lifecycle` from druid)
                    self.lifecycle(parent_ctx, &LifecycleEvent::Initialize, data);
                    self.state.initialized = true;
                } else {
                    // assume all children initialized as well, so don't propagate
                    return;
                }
            }
            _ => {}
        }

        // Propagate
        let mut ctx = EventCtx {
            app_ctx: parent_ctx.app_ctx,
            state: Some(&mut self.state),
        };
        self.inner.lifecycle(&mut ctx, event, data);
    }

    /// Propagates a data change to the wrapped widget.
    pub fn update(&mut self, parent_ctx: &mut UpdateCtx, data: &mut T, change: &T::Change) {
        // parent_ctx: ctx of the parent widget
        // ctx: ctx of this widget

        // propagate
        let mut ctx = UpdateCtx {
            app_ctx: parent_ctx.app_ctx,
            state: Some(&mut self.state),
            children_changed: false,
        };
        self.inner.update(&mut ctx, data, change);

        if ctx.children_changed {
            // children changed: filter is invalid, rebuild it
            ctx.state.as_mut().unwrap().child_filter.clear();
            let mut event_ctx = EventCtx {
                app_ctx: ctx.app_ctx,
                state: ctx.state,
            };
            self.inner.lifecycle(
                &mut event_ctx,
                &LifecycleEvent::UpdateChildFilter,
                data,
            );
            parent_ctx.children_changed = true;

            // initialize new children
            self.inner.lifecycle(
                &mut event_ctx,
                &LifecycleEvent::RouteInitialize,
                data,
            );
        }
    }

    /// Called to measure this widget and layout the children of this widget.
    pub fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        data: &mut T,
        env: &Environment,
    ) -> Measurements {
        self.state.measurements = self.inner.layout(ctx, constraints, data, env);
        self.state.measurements
    }

    /// Draws the widget using the given `PaintCtx`.
    pub fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, data: &mut T, env: &Environment) {
        self.inner.paint(ctx, bounds, data, env)
    }

    pub(crate) fn send_root_event(&mut self, app_ctx: &mut AppCtx, event: &Event, data: &mut T) {
        let mut event_ctx = EventCtx {
            app_ctx,
            state: None,
        };

        self.event(&mut event_ctx, event, data);
    }
}
