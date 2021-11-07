use crate::{
    application::AppCtx,
    bloom::Bloom,
    cache::{Cache, Key},
    call_key::CallId,
    event::{InputState, PointerEvent},
    layout::LayoutItem,
    region::Region,
    BoxConstraints, Data, Environment, Event, InternalEvent, Measurements, Offset, Point, Rect,
    Size,
};
use kyute_macros::composable;
use kyute_shell::{
    drawing::DrawContext,
    winit::{event_loop::EventLoopWindowTarget, window::WindowId},
};
use std::{
    cell::{Cell, RefCell},
    fmt,
    fmt::Formatter,
    hash::{Hash, Hasher},
    num::NonZeroU64,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, Weak},
};

/// Context passed to widgets during the layout pass.
///
/// See [`Widget::layout`].
pub struct LayoutCtx {}

pub struct PaintCtx<'a> {
    pub draw_ctx: &'a mut DrawContext,
    pub id: WidgetId,
    pub window_bounds: Rect,
    pub focus: Option<WidgetId>,
    pub pointer_grab: Option<WidgetId>,
    pub hot: Option<WidgetId>,
    pub inputs: &'a InputState,
    pub scale_factor: f64,
    pub invalid: &'a Region,
    pub hover: bool,
}

impl<'a> PaintCtx<'a> {
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

pub struct EventCtx<'a> {
    pub(crate) app_ctx: &'a mut AppCtx,
    pub(crate) event_loop: &'a EventLoopWindowTarget<()>,
    id: WidgetId,
    child_filter: Bloom<WidgetId>,
}

impl<'a> EventCtx<'a> {
    pub fn widget_id(&self) -> WidgetId {
        self.id
    }

    pub fn set_state<T: 'static>(&mut self, key: Key<T>, value: T) {
        self.app_ctx.cache.set_state(key, value).unwrap()
    }

    pub fn register_window(&mut self, window_id: WindowId) {
        self.app_ctx.register_window_widget(window_id, self.id);
    }

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

/// Trait that defines the behavior of a widget.
pub trait Widget {
    /// Implement to give a debug name to your widget. Used only for debugging.
    fn debug_name(&self) -> &str {
        "Widget"
    }

    /// Propagates an event through the widget hierarchy.
    fn event(&self, ctx: &mut EventCtx, event: &Event);

    /// Measures this widget and layouts the children of this widget.
    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> Measurements;

    /// Paints the widget in the given context.
    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment);

    /// Called only for native window widgets.
    fn window_paint(&self, _ctx: &mut WindowPaintCtx) {}
}

/*
#[derive(Clone, Data)]
pub struct WidgetHandle {
    //state: Arc<RefCell<WidgetState>>,
}

impl WidgetHandle {
    #[composable]
    pub fn new() -> WidgetHandle {
        todo!()
    }
}*/

/// ID of a node in the tree.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct WidgetId(CallId);

impl WidgetId {
    pub(crate) fn from_call_id(call_id: CallId) -> WidgetId {
        WidgetId(call_id)
    }
}

impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04X}", self.0.to_u64())
    }
}

struct WidgetPodInner<T: ?Sized> {
    id: WidgetId,
    offset: Cell<Option<Offset>>,
    measurements: Cell<Option<Measurements>>,
    child_filter: Cell<Option<Bloom<WidgetId>>>,
    initialized: Cell<bool>,
    widget: T,
}

fn compute_child_filter<T: Widget>(widget: &T) -> Bloom<WidgetId> {
    // TODO the widget needs to cooperate but there are no suitable functions in the trait
    // (`event` needs an `EventCtx`, which needs an `AppCtx`).
    Default::default()
}

/// Represents a widget.
pub struct WidgetPod<T: ?Sized = dyn Widget>(Arc<WidgetPodInner<T>>);

impl<T: Widget> WidgetPod<T> {
    /// Creates a new `WidgetPod` wrapping the specified widget.
    #[composable(uncached)]
    pub fn new(widget: T) -> WidgetPod<T> {
        let id = WidgetId::from_call_id(Cache::current_call_id());
        let initialized = !Cache::changed(()); // false on first call, true on following calls
        let inner = WidgetPodInner {
            id,
            offset: Cell::new(Default::default()),
            measurements: Cell::new(Default::default()),
            child_filter: Cell::new(None),
            widget,
            initialized: Cell::new(initialized),
        };
        WidgetPod(Arc::new(inner))
    }
}

impl<T: ?Sized> Clone for WidgetPod<T> {
    fn clone(&self) -> Self {
        WidgetPod(self.0.clone())
    }
}

impl<T: ?Sized + 'static> Data for WidgetPod<T> {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}

// TODO remove this once we have unsized coercions
impl<T: Widget + 'static> From<WidgetPod<T>> for WidgetPod {
    fn from(other: WidgetPod<T>) -> Self {
        WidgetPod(other.0)
    }
}

impl<T: ?Sized + Widget> WidgetPod<T> {
    /// Returns a reference to the wrapped widget.
    pub fn widget(&self) -> &T {
        &self.0.widget
    }

    /// Called to measure this widget and layout the children of this widget.
    pub fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> Measurements {
        if let Some(m) = self.0.measurements.get() {
            m
        } else {
            let m = self.0.widget.layout(ctx, constraints, env);
            self.0.measurements.set(Some(m));
            m
        }
    }

    pub fn set_child_offset(&self, offset: Offset) {
        self.0.offset.set(Some(offset))
    }

    /// Paints the widget.
    pub fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        let offset = if let Some(offset) = self.0.offset.get() {
            offset
        } else {
            tracing::warn!("`paint` called before layout");
            return;
        };
        let measurements = if let Some(m) = self.0.measurements.get() {
            m
        } else {
            tracing::warn!("`paint` called before layout");
            return;
        };
        let size = measurements.size;
        // bounds of this widget in window space
        let window_bounds = Rect::new(ctx.window_bounds.origin + offset, size);
        if !ctx.invalid.intersects(window_bounds) {
            // not invalidated, no need to redraw
            return;
        }

        /*let _span = trace_span!(
            "paint",
            ?self.id,
            ?offset,
            ?measurements,
        ).entered();*/
        // trace!(?ctx.scale_factor, ?ctx.inputs.pointers, ?window_bounds, "paint");

        let hover = ctx.inputs.pointers.iter().any(|(_, state)| {
            window_bounds.contains(Point::new(
                state.position.x * ctx.scale_factor,
                state.position.y * ctx.scale_factor,
            ))
        });

        ctx.draw_ctx.save();
        ctx.draw_ctx.transform(&offset.to_transform());

        {
            let mut child_ctx = PaintCtx {
                draw_ctx: ctx.draw_ctx,
                window_bounds,
                focus: ctx.focus,
                pointer_grab: ctx.pointer_grab,
                hot: ctx.hot,
                inputs: ctx.inputs,
                scale_factor: ctx.scale_factor,
                id: self.0.id,
                hover,
                invalid: &ctx.invalid,
            };
            self.0
                .widget
                .paint(&mut child_ctx, Rect::new(Point::origin(), size), env);
        }

        ctx.draw_ctx.restore();
    }

    pub(crate) fn compute_child_filter(&self, parent_ctx: &mut EventCtx) -> Bloom<WidgetId> {
        if let Some(filter) = self.0.child_filter.get() {
            // already computed
            filter
        } else {
            tracing::trace!("computing child filter");
            // not computed: compute by sending the `UpdateChildFilter` message to the widget,
            // which will be forwarded to all children, which in turn will update `ctx.child_filter`.
            let mut ctx = EventCtx {
                app_ctx: parent_ctx.app_ctx,
                event_loop: parent_ctx.event_loop,
                id: self.0.id,
                child_filter: Bloom::new(),
            };
            self.0
                .widget
                .event(&mut ctx, &Event::Internal(InternalEvent::UpdateChildFilter));
            self.0.child_filter.set(Some(ctx.child_filter));
            ctx.child_filter
        }
    }

    /// Returns whether this widget may contain the specified widget as a child (direct or not).
    fn may_contain(&self, widget: WidgetId) -> bool {
        if let Some(filter) = self.0.child_filter.get() {
            filter.may_contain(&widget)
        } else {
            tracing::warn!("`may_contain` called but child filter not initialized");
            true
        }
    }

    /// Propagates an event to the wrapped widget.
    pub fn event(&self, parent_ctx: &mut EventCtx, event: &Event) {
        // Handle internal events (routing mostly)
        match event {
            Event::Internal(InternalEvent::RouteWindowEvent { target, event }) => {
                if *target == self.0.id {
                    self.event(parent_ctx, &Event::WindowEvent(event.clone()));
                    return;
                }
                if !self.may_contain(*target) {
                    return;
                }
            }
            Event::Internal(InternalEvent::RouteRedrawRequest(target)) => {
                if *target == self.0.id {
                    self.event(parent_ctx, &Event::WindowRedrawRequest);
                    return;
                }
                if !self.may_contain(*target) {
                    return;
                }
            }
            Event::Internal(InternalEvent::UpdateChildFilter) => {
                parent_ctx.child_filter.add(&self.0.id);
                let child_filter = self.compute_child_filter(parent_ctx);
                parent_ctx.child_filter.extend(&child_filter);
                return;
            }
            Event::Internal(InternalEvent::RouteInitialize) => {
                if !self.0.initialized.get() {
                    self.event(parent_ctx, &Event::Initialize);
                    self.0.initialized.set(true);
                } else {
                    // assume all children initialized as well (the widget should propagate the `Initialize` event to its children)
                    // so don't propagate
                    return;
                }
            }
            _ => {}
        }

        // If we fall here, propagate to the widget inside
        let mut ctx = EventCtx {
            app_ctx: parent_ctx.app_ctx,
            event_loop: parent_ctx.event_loop,
            id: self.0.id,
            child_filter: Default::default(),
        };
        self.0.widget.event(&mut ctx, event)
    }

    /// Prepares the root `EventCtx` and calls `self.event()`.
    pub(crate) fn send_root_event(
        &self,
        app_ctx: &mut AppCtx,
        event_loop: &EventLoopWindowTarget<()>,
        event: &Event,
    ) {
        let mut event_ctx = EventCtx {
            app_ctx,
            event_loop,
            id: self.0.id,
            child_filter: Default::default(),
        };
        self.event(&mut event_ctx, event);
    }

    pub(crate) fn ensure_initialized(
        &self,
        app_ctx: &mut AppCtx,
        event_loop: &EventLoopWindowTarget<()>,
    ) {
        let mut event_ctx = EventCtx {
            app_ctx,
            event_loop,
            id: self.0.id,
            child_filter: Default::default(),
        };
        self.compute_child_filter(&mut event_ctx);
        self.send_root_event(
            app_ctx,
            event_loop,
            &Event::Internal(InternalEvent::RouteInitialize),
        );
        self.root_layout(app_ctx);
    }

    pub(crate) fn root_layout(&self, app_ctx: &mut AppCtx) -> Measurements {
        let mut ctx = LayoutCtx {};
        let env = Environment::new();
        self.layout(
            &mut ctx,
            BoxConstraints {
                min: Size::new(0.0, 0.0),
                max: Size::new(f64::INFINITY, f64::INFINITY),
            },
            &env,
        )
    }
}
