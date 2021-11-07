use crate::{
    application::AppCtx,
    bloom::Bloom,
    cache_cell::CacheCell,
    call_key::CallKey,
    event::{InputState, PointerEvent},
    layout::LayoutItem,
    region::Region,
    BoxConstraints, Cache, CacheInvalidationToken, Data, Environment, Event, InternalEvent,
    Measurements, Offset, Point, Rect, Size,
};
use kyute_macros::composable;
use kyute_shell::{drawing::DrawContext, winit::window::WindowId};
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

pub struct EventCtx<'a> {
    app_ctx: &'a mut AppCtx,
    pub(crate) child_filter: Bloom<WidgetId>,
}

impl EventCtx {
    pub fn invalidate(&mut self, token: CacheInvalidationToken) {
        self.app_ctx.invalidate_cache(token)
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
        "WidgetDelegate"
    }

    /// Propagates an event through the widget hierarchy.
    fn event(&self, ctx: &mut EventCtx, event: &Event);

    /// Measures this widget and layouts the children of this widget.
    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> LayoutItem;

    /// Paints the widget in the given context.
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

/// Internal widget state.
/// Currently empty.
struct WidgetState {
    // TODO
}

#[derive(Clone, Data)]
pub struct WidgetHandle {
    token: CacheInvalidationToken,
    state: Arc<RefCell<WidgetState>>,
}

impl WidgetHandle {
    #[composable]
    pub fn new() -> WidgetHandle {
        let token = Cache::get_invalidation_token();
        let state = Arc::new(RefCell::new(WidgetState {}));
        WidgetHandle { token, state }
    }
}

/// ID of a node in the tree.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct WidgetId(CallKey);

impl WidgetId {
    pub(crate) fn from_call_key(key: CallKey) -> WidgetId {
        WidgetId(key)
    }
}

impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04X}", self.0.get())
    }
}

struct WidgetInner<T: ?Sized> {
    id: WidgetId,
    child_filter: Bloom<WidgetId>,
    delegate: T,
}

fn compute_child_filter<T: Widget>(delegate: &T) -> Bloom<WidgetId> {
    let mut ctx = EventCtx {
        child_filter: Default::default(),
    };
    delegate.event(&mut ctx, &Event::Internal(InternalEvent::UpdateChildren));
    ctx.child_filter
}

/// Represents a widget.
pub struct WidgetPod<T: ?Sized = dyn Widget>(Arc<WidgetInner<T>>);

impl<T: Widget> WidgetPod<T> {
    #[composable(uncached)]
    pub fn new(delegate: T) -> WidgetPod<T> {
        let child_filter = compute_child_filter(&delegate);
        let id = WidgetId::from_call_key(Cache::current_call_key());
        let inner = WidgetInner {
            id,
            child_filter,
            delegate,
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

impl<T: Widget + 'static> From<WidgetPod<T>> for WidgetPod {
    fn from(widget: WidgetPod<T>) -> Self {
        WidgetPod(widget.0)
    }
}

impl<T: ?Sized + Widget> WidgetPod<T> {
    /// Propagates an event to the wrapped widget.
    pub fn event(&self, parent_ctx: &mut EventCtx, event: &Event) {
        // Handle internal events
        match event {
            Event::Internal(InternalEvent::UpdateChildren) => {
                parent_ctx.child_filter.extend(&self.0.child_filter);
                parent_ctx.child_filter.add(&self.0.id);
                return;
            }
            _ => {}
        }

        self.inner.event(&mut ctx, event, data)
    }

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
