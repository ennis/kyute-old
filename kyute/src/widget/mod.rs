//! `Widget` base trait and built-in widgets.
mod flex;
mod grid;
mod button;
mod scope_table;
mod gap_buffer;

use crate::{
    layout::{BoxConstraints, Measurements},
    node::{NodeCursor, NodeId, NodeRef, NodeTree, PaintCtx},
    Offset, Point, Rect, Size,
};

use crate::key::Key;
use std::{any::Any, cell::Cell, marker::PhantomData, panic::Location};

struct LayoutCtx {}

pub trait Widget: Any {
    /// Called to measure this widget and layout the children of this widget (`ctx.children_mut()`).
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut [Node],
        constraints: &BoxConstraints,
    ) -> Measurements;

    /// Called to paint the widget
    fn paint(&mut self, ctx: &mut PaintCtx, children: &[Node], bounds: Rect);
}

pub struct State {
    key: Key,
    data: Box<dyn Any>,
}

pub struct Node<W = Box<dyn Widget>> {
    /// Offset of the node relative to the parent
    pub(crate) offset: Offset,

    /// Layout of the node (size and baseline).
    pub(crate) measurements: Measurements,

    /// Position of the node in window coordinates.
    pub(crate) window_pos: Cell<Point>,

    /// Widget
    pub(crate) widget: W,
}

impl<W: Widget> Node<W> {
    pub fn new(widget: W) -> Node<W> {
        Node {
            offset: Default::default(),
            measurements: Default::default(),
            window_pos: Cell::new(Default::default()),
            widget,
            children: vec![],
            state_entries: vec![],
        }
    }

    /// Layouts the node.
    pub fn layout(&mut self, ctx: &mut LayoutCtx, constraints: &BoxConstraints) -> Measurements {
        let mut ctx = LayoutCtx {};
        self.widget
            .layout(&mut ctx, &mut self.children, constraints)
    }

    /// Sets the offset of this node relative to its parent. Call during layout.
    pub fn set_offset(&mut self, offset: Offset) {
        self.offset = offset;
    }

    fn paint(&mut self, ctx: &mut PaintCtx, bounds: Rect) {
        let mut ctx = PaintCtx {};
        self.widget.paint(&mut ctx, &mut self.children, bounds);
    }
}

enum ScopeTreeNode {
    Scope(ScopeTree),
    State(State),
}

struct ScopeTree {
    key: Key,
    slots: Vec<ScopeTreeNode>,
}

/// T: the type of child widgets that are expected. Might be `Box<dyn Widget>`.
pub struct CompositionCtx<'a> {
    /// The parent node for which we are recomposing the children of.
    parent: &'a mut Node,
    /// Call tree node
    scope_tree: &'a mut ScopeTree,
    /// Where we should emit the next node in the list of children.
    pos: usize,
    /// Same, but for the call tree
    call_tree_pos: usize,
}

impl<'a> CompositionCtx<'a> {
    pub fn new(node: &'a mut Node, scope_tree: &'a mut CallTree) -> CompositionCtx<'a> {
        CompositionCtx {
            parent: node,
            scope_tree,
            pos: 0,
            call_tree_pos: 0
        }
    }

    /// Enter a composition scope.
    pub fn enter(&mut self) {
        let key = Key::from_caller();
        let pos = self.scope_tree.slots[self.call_tree_pos..].iter().position(|s| match s {
           ScopeTreeNode::Scope(ScopeTree {
                                    key: key, slots
                                }) => false,
            _ => false
        } );

        if let Some(pos) = pos {

        }
    }

    #[track_caller]
    pub fn state<T: Any + Clone>(&mut self, init: impl FnOnce() -> T) -> T {
        let key = Key::from_caller();

        let index = self.parent.slots[self.pos..]
            .iter()
            .position(|&w| w.key == key);

        if let Some(index) = index {
            // found an existing slot, swap it in place
            self.parent.state_entries.swap(index, self.pos);
        } else {
            // create and insert a new node with the provided constructor
            self.parent
                .state_entries
                .insert(self.pos, State { key, data: init() });
        };

        self.state_pos += 1;

        // safety: if the key matches, then this call originates from the same location that created
        // the state entry, and thus the type must be the same.
        unsafe { &*(state as *mut dyn Any as *mut T).clone() }
    }

    /// Emits (get or create) a child widget with the specified key.
    #[track_caller]
    pub fn emit<T>(&mut self, init: impl FnOnce() -> T) -> CompositionCtx<'a>
    where
        T: Widget,
    {
        let key = Key::from_caller();

        let index = self.parent.children[self.pos..]
            .iter()
            .position(|&w| w.key == key);

        if let Some(index) = index {
            // found an existing node, swap it in place
            self.parent.children.swap(index, self.pos);
        } else {
            // create and insert a new node with the provided constructor
            self.parent
                .children
                .insert(self.pos, Node::new(Box::new(init())));
        };

        // create child composition context
        CompositionCtx {
            parent: node,
            pos: 0,
            state_pos: 0,
        }
    }
}

// Simple wrapper widgets can provide a composition context with a type T, inferred from the result

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Dummy;

impl Widget for Dummy {
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &mut [Node],
        constraints: &BoxConstraints,
    ) -> Measurements {
        todo!()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, children: &[Node], bounds: Rect) {
        todo!()
    }
}
