//! TODO this should be renamed because "composition" is also a term related to text input
use crate::{
    application::AppCtx,
    core::{Node, Widget},
    data::Data,
    env::{EnvKey, Environment},
    key::Key,
    EnvValue, NodeId,
};
use kyute_shell::{
    window::PlatformWindow,
    winit::{event_loop::EventLoopWindowTarget, window::WindowId},
};
use std::{any::Any, mem};

/// Type-erased state stored in the composition table of a node.
pub struct State {
    key: Key,
    data: Box<dyn Any>,
}

/// An entry in a composition table.
pub(crate) enum CompositionSlot {
    /// Marks the start of a scope.
    ScopeStart {
        /// Scope length including this entry and the scope end.
        // u32 to reduce size of `Entry`
        len: u32,
        key: Key,
    },

    /// Marks the end of a scope.
    ScopeEnd { key: Key },

    /// Represents a node.
    Node {
        /// Index of the node in the child list of the node that holds this table.
        // u32 to reduce size of `Entry`
        child_index: u32,
        key: Key,
    },

    /// Holds a piece of state.
    State(Box<State>), // 24b
}

impl CompositionSlot {
    /// Returns the "length" of this entry, i.e. the number of entries to go to the next one at the
    /// same scope level. Returns 0 for `ScopeEnd` entries.
    fn len(&self) -> usize {
        match self {
            CompositionSlot::ScopeStart { len, .. } => *len as usize,
            CompositionSlot::ScopeEnd { .. } => 0,
            _ => 1,
        }
    }

    /// Writes the length of a `ScopeStart` entry.-
    fn set_len(&mut self, new_len: usize) {
        match self {
            CompositionSlot::ScopeStart { len, .. } => *len = new_len as u32,
            _ => panic!("unexpected entry type"),
        }
    }
}

pub(crate) fn dump_composition_table(table: &[CompositionSlot]) {
    let mut indent = 0;
    for e in table.iter() {
        match e {
            CompositionSlot::ScopeStart { len, key } => {
                eprintln!(
                    "{:indent$}Scope `{}` len={} begin",
                    "",
                    key,
                    len,
                    indent = indent
                );
                indent += 2;
            }
            CompositionSlot::ScopeEnd { key } => {
                indent -= 2;
                eprintln!("{:indent$}Scope `{}` end", "", key, indent = indent);
            }

            CompositionSlot::Node { child_index, key } => {
                eprintln!(
                    "{:indent$}Node `{}` index={}",
                    "",
                    key,
                    child_index,
                    indent = indent
                );
            }
            CompositionSlot::State(s) => {
                eprintln!("{:indent$}State `{}`", "", s.key, indent = indent);
            }
        }
    }
}

/// Utility class to update a node's internal composition table
struct Composer {
    /// Composition table
    table: Vec<CompositionSlot>,
    /// Current write index in the scope table
    pos: usize,
    /// Start of the current scope
    scope_start: Option<usize>,
    /// return index
    stack: Vec<Option<usize>>,
}

impl Composer {
    fn new(table: Vec<CompositionSlot>) -> Composer {
        Composer {
            table,
            pos: 0,
            scope_start: None,
            stack: vec![],
        }
    }

    fn restart(&mut self) {
        // composition can only be restarted at the top-level scope
        assert!(self.stack.is_empty());
        self.pos = 0;
        self.scope_start = None;
    }

    ///
    fn insert(&mut self, entry: CompositionSlot) {
        self.table.insert(self.pos, entry);
    }

    /// Find an entry in the current scope.
    fn find(entries: &[CompositionSlot], key: Key) -> Option<usize> {
        let mut i = 0;
        while i < entries.len() {
            match &entries[i] {
                CompositionSlot::ScopeEnd { .. } => break,
                CompositionSlot::ScopeStart { key: this_key, .. } if this_key == &key => {
                    return Some(i)
                }
                CompositionSlot::ScopeStart { len, .. } => {
                    // skip the scope
                    i += *len as usize + 1;
                }
                CompositionSlot::Node { key: this_key, .. } if this_key == &key => return Some(i),
                CompositionSlot::State(s) if s.key == key => return Some(i),
                _ => {
                    i += 1;
                }
            }
            i += entries[i].len()
        }
        None
    }

    /// Rotates an entry in the current scope with the specified key so that it ends up at the current position.
    /// Returns whether the entry was found and rotated in place.
    fn rotate(&mut self, key: Key) -> bool {
        let scope = &mut self.table[self.pos..];
        let r = Self::find(scope, key);

        if let Some(i) = r {
            // entry found at position i
            // move it in place
            scope.rotate_left(i);
            true
        } else {
            false
        }
    }

    /// Rotates a `Node` entry. Returns the child index of the node if found.
    fn rotate_node(&mut self, key: Key) -> Option<u32> {
        if self.rotate(key) {
            match &self.table[self.pos] {
                CompositionSlot::Node { child_index, .. } => Some(*child_index),
                _ => panic!("unexpected entry type"),
            }
        } else {
            None
        }
    }

    /// Rotates a `State` entry. Returns a reference to the contents if found.
    fn rotate_state(&mut self, key: Key) -> Option<&mut State> {
        if self.rotate(key) {
            match self.table[self.pos] {
                CompositionSlot::State(ref mut state) => Some(state),
                _ => panic!("unexpected entry type"),
            }
        } else {
            None
        }
    }

    /// Returns the key of the current scope.
    fn current_scope_key(&self) -> Key {
        match &self.table[self.scope_start.unwrap()] {
            CompositionSlot::ScopeStart { key, .. } => *key,
            _ => panic!("expected scope start"),
        }
    }

    /// Enters a composition scope. Must be matched with a call to `exit`.
    /// Returns true if the entry wasn't there before and was just created.
    fn enter(&mut self, key: Key) -> bool {
        let just_created = if !self.rotate(key) {
            // not found, begin a new scope
            self.table
                .insert(self.pos, CompositionSlot::ScopeStart { len: 2, key });
            self.table
                .insert(self.pos + 1, CompositionSlot::ScopeEnd { key });
            true
        } else {
            false
        };

        // enter the scope
        self.stack.push(self.scope_start);
        self.scope_start = Some(self.pos);
        self.pos += 1;
        just_created
    }

    /// Exits the current composition scope.
    fn exit(&mut self) {
        // find the marker for the end of the scope
        let scope_key = self.current_scope_key();
        let scope_end_rel = self.table[self.pos..]
            .iter()
            .position(|x| match x {
                CompositionSlot::ScopeEnd { key } if *key == scope_key => true,
                _ => false,
            })
            .expect("end of scope not found");

        // remove extra entries
        let scope_end = self.pos + scope_end_rel;
        self.table.drain(self.pos..scope_end);

        // skip scope end marker
        self.pos += 1;

        // set scope length
        let scope_start = self.scope_start.unwrap();
        self.table[scope_start].set_len(self.pos - scope_start);

        // return to the parent scope
        self.scope_start = self.stack.pop().unwrap();
    }

    ///
    unsafe fn with_state_mut<T, F>(&mut self, index: usize, f: F)
    where
        T: Any,
        F: FnOnce(&mut T),
    {
        match self.table[index] {
            CompositionSlot::State(ref mut state) => {
                // safety: ensured by caller
                let state = &mut *(state.data.as_mut() as *mut dyn Any as *mut T);
                f(state);
            }
            _ => panic!("unexpected entry type"),
        }
    }

    /// Emits a state slot and returns the data contained inside.
    fn extract_state<T: Any + Clone>(
        &mut self,
        key: Key,
        init: impl FnOnce() -> T,
    ) -> (usize, Box<dyn Any>) {
        let state = self.rotate_state(key);

        let data = if let Some(State { data, .. }) = state {
            // replace with a dummy
            mem::replace(data, Box::new(()))
        } else {
            // create and insert a new state entry
            let state = Box::new(State {
                key,
                data: Box::new(()),
            });
            // TODO remove double-boxing
            self.insert(CompositionSlot::State(state));
            let data: Box<dyn Any> = Box::new(init());
            data
        };

        let pos = self.pos;
        self.pos += 1;
        (pos, data)
    }

    ///
    fn write_state(&mut self, index: usize, data: Box<dyn Any>) {
        match self.table[index] {
            CompositionSlot::State(ref mut state) => {
                state.data = data;
            }
            _ => panic!("unexpected entry type"),
        }
    }

    /// Emits a node. Returns the child index.
    fn emit_node(&mut self, key: Key, parent_child_count: usize) -> usize {
        let child_index = self.rotate_node(key);

        let child_index = if let Some(i) = child_index {
            i as usize
        } else {
            // insert a new node entry
            self.table.insert(
                self.pos,
                CompositionSlot::Node {
                    key,
                    child_index: parent_child_count as u32,
                },
            );
            parent_child_count
        };

        self.pos += 1;
        child_index
    }

    /// Finishes writes to the table and returns it.
    fn finish(mut self) -> Vec<CompositionSlot> {
        // TODO check for balancing of calls to enter/exit
        self.table.truncate(self.pos);
        self.table
    }
}

/// Context passed to `init` closure of `CompositionCtx::emit_node`.
pub struct InitCtx<'a> {
    node_id: NodeId,
    app_ctx: &'a mut AppCtx,
    event_loop: &'a EventLoopWindowTarget<()>,
    window: Option<PlatformWindow>,
}

impl<'a> InitCtx<'a> {
    /// Returns a handle to the application's event loop. Used to create new windows in a composition context.
    pub fn event_loop(&self) -> &'a EventLoopWindowTarget<()> {
        self.event_loop
    }

    /// Registers the newly created node as a native window widget with the following window ID.
    /// The event loop will call `window_event` whenever an event targeting the window is received.
    pub fn set_window(&mut self, window: PlatformWindow) {
        self.app_ctx
            .register_native_window(window.id(), self.node_id);
        self.window = Some(window);
    }
}

/// Context passed to `update` closure of `CompositionCtx::emit_node`.
pub struct UpdateCtx<'a> {
    app_ctx: &'a mut AppCtx,
    event_loop: &'a EventLoopWindowTarget<()>,
}

impl<'a> UpdateCtx<'a> {
    /// Returns a handle to the application's event loop. Used to create new windows in a composition context.
    pub fn event_loop(&self) -> &'a EventLoopWindowTarget<()> {
        self.event_loop
    }

    pub fn request_relayout(&mut self) {
        self.app_ctx.request_relayout();
    }
}

/// Helper for emit_node function.
unsafe fn downcast_widget_unchecked<T: Widget>(widget: &mut dyn Widget) -> &mut T {
    &mut *(widget as *mut dyn Widget as *mut T)
}

pub struct ActionResult(Option<Box<dyn Any>>);

impl ActionResult {
    pub fn cast<T: Any>(self) -> Option<T> {
        self.0.and_then(|x| x.downcast::<T>().ok()).map(|x| *x)
    }
}

/// Context passed to composable functions that produce nodes.
pub struct CompositionCtx<'a, 'node> {
    app_ctx: &'a mut AppCtx,
    event_loop: &'a EventLoopWindowTarget<()>,
    parent_window_id: Option<WindowId>,
    env: Environment,
    /// Never skip recomposition of a composable even if its parameters did not change.
    /// This is usually set to `true` when a theme variable has changed.
    no_skip: bool,
    /// Node being edited.
    node: &'node mut Node,
    ///
    composer: Composer,
    /// Whether the user has requested a recomposition of this node after the current one.
    recompose_after: bool,
    /// `true` if the IDs of the children of the node being composed may have changed. This will cause
    /// a rebuild of the bloom filters.
    rebuild_child_filter: bool,
}

impl<'a, 'node> CompositionCtx<'a, 'node> {
    /// Enters a composition scope. Must be matched with a call to `exit`.
    #[track_caller]
    pub fn enter(&mut self, id: u64) -> bool {
        let key = Key::from_caller(id);
        self.do_enter(key)
    }

    /// Exits a composition scope.
    pub fn exit(&mut self) {
        self.do_exit();
    }

    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// Gets an environment value.
    pub fn get_env<T: EnvValue>(&self, key: EnvKey<T>) -> Option<T> {
        self.env.get(key)
    }

    /// Emits a child node.
    #[track_caller]
    pub fn emit_node<T, Init, Update, Contents>(
        &mut self,
        init: Init,
        update: Update,
        contents: Contents,
    ) -> ActionResult
    where
        T: Widget,
        Init: FnOnce(&mut InitCtx) -> T,
        Update: FnOnce(&mut UpdateCtx, &mut T),
        Contents: FnMut(&mut CompositionCtx),
    {
        let key = Key::from_caller(0);
        unsafe { self.do_emit_node(key, init, update, contents) }
    }

    #[track_caller]
    pub fn has_changed<T: Data>(&mut self, data: T) -> bool {
        self.with_state(|| data.clone(), |_cx, prev_data| {})
    }

    /// Entirely replaces the environment with another.
    #[track_caller]
    pub fn with_environment(&mut self, env: Environment, inner: impl FnOnce(&mut Self)) {
        self.with_state_no_recomp(Environment::new, |cx, prev_env| {
            let prev_no_skip = cx.no_skip;
            if !prev_env.same(&env) {
                // the environment has changed, update children
                cx.no_skip = true;
            }
            let old_env = mem::replace(&mut cx.env, env);
            inner(cx);
            *prev_env = mem::replace(&mut cx.env, old_env);
            cx.no_skip = prev_no_skip;
        });
    }

    /// Emits a state entry.
    #[track_caller]
    pub fn with_state<T, F, Init>(&mut self, init: Init, f: F) -> bool
    where
        T: Data,
        Init: FnOnce() -> T,
        F: FnOnce(&mut Self, &mut T),
    {
        let key = Key::from_caller(0);
        let _span =
            tracing::trace_span!("with_state", location = key.caller_location().file()).entered();
        let (index, mut data) = self.composer.extract_state(key, init);

        // safety: by construction, if the key matches, then the call site is the same and thus
        // the statically known type T is the same.
        let mut old_data = data.as_mut().downcast_mut::<T>().unwrap();
        let mut new_data = old_data.clone();

        // invoke the closure
        f(self, &mut new_data);
        // compare old & new and request recomp if different
        // FIXME: in some cases, we don't need that
        let data_changed = !old_data.same(&new_data);
        if data_changed {
            tracing::trace!("with_state: data changed, requesting recomposition");
            self.recompose_after = true;
        }
        // write back the data
        *old_data = new_data;
        // put the state back in place
        self.composer.write_state(index, data);
        data_changed
    }

    /// Same as with_state, but don't recompose if the value changes.
    #[track_caller]
    pub fn with_state_no_recomp<T, F, Init>(&mut self, init: Init, f: F)
    where
        T: Data,
        Init: FnOnce() -> T,
        F: FnOnce(&mut Self, &mut T),
    {
        let key = Key::from_caller(0);
        let (index, mut data) = self.composer.extract_state(key, init);
        let mut data_ref = data.as_mut().downcast_mut::<T>().unwrap();
        f(self, data_ref);
        self.composer.write_state(index, data);
    }

    /// Requests a recomposition when after this ctx is finished (because e.g. some state has changed).
    pub fn request_recomposition(&mut self) {
        self.recompose_after = true;
    }
}

fn create_node<T: Widget>(
    app_ctx: &mut AppCtx,
    event_loop: &EventLoopWindowTarget<()>,
    parent_window_id: Option<WindowId>,
    init: impl FnOnce(&mut InitCtx) -> T,
    env: &Environment,
) -> Node {
    let node_id = NodeId::next();
    let mut init_ctx = InitCtx {
        node_id,
        app_ctx,
        event_loop,
        window: None,
    };
    let widget = Box::new(init(&mut init_ctx));
    let mut node: Node = unsafe {
        // safety: node_id was created with `NodeId::next()`
        Node::new(
            widget,
            node_id,
            parent_window_id,
            init_ctx.window,
            env.clone(),
        )
    };
    node
}

//-----------------------------------------------------------------------------
// CompositionCtx internal methods
impl<'a, 'node> CompositionCtx<'a, 'node> {
    fn do_enter(&mut self, key: Key) -> bool {
        self.composer.enter(key)
    }

    fn do_exit(&mut self) {
        self.composer.exit();
    }

    unsafe fn do_emit_node<T>(
        &mut self,
        key: Key,
        init: impl FnOnce(&mut InitCtx) -> T,
        update: impl FnOnce(&mut UpdateCtx, &mut T),
        contents: impl FnMut(&mut CompositionCtx),
    ) -> ActionResult
    where
        T: Widget,
    {
        let child_index = {
            let child_count = self.node.children.len();
            let child_index = self.composer.emit_node(key, child_count);
            if child_index == child_count {
                let node = create_node(
                    self.app_ctx,
                    self.event_loop,
                    self.parent_window_id,
                    init,
                    &self.env,
                );
                let id = node.id;
                self.node.children.push(node);
                self.node.child_filter.add(&id);
                self.app_ctx.request_relayout();
            } else {
                // Do not run `update` if the node has pending actions:
                // Before we can update the node, we must first process all pending actions, which
                // might affect the application state, and in turn affect the node itself.
                // Once all actions have dequeued, the state application state should have
                // "stabilized", and the node can be updated.
                //
                // In theory, this is not necessary, but in practice this reduces the number of calls
                // to `update` and avoids unnecessary invalidation of internal state.
                let node = &mut self.node.children[child_index];
                if node.pending_actions.is_empty() {
                    // SAFETY: ensured by the `do_emit_node` call contract.
                    let t =
                        downcast_widget_unchecked::<T>(node.widget.as_mut());
                    let mut update_ctx = UpdateCtx {
                        app_ctx: self.app_ctx,
                        event_loop: self.event_loop,
                    };
                    update(&mut update_ctx, t);
                }
            }
            child_index
        };

        let node = &mut self.node.children[child_index];

        // recursively recompose the emitted node
        node.recompose(self.app_ctx, self.event_loop, self.env.clone(), contents);

        // If the emitted node has pending actions, we process them first, and then issue
        // a recomposition in case the application state changed as a result of the action.
        if node.pending_actions.len() > 1 {
            self.recompose_after = true;
        }

        ActionResult(node.pending_actions.pop())
    }

    /// Restarts the composition of this node.
    fn restart(&mut self) {
        self.composer.restart();
        self.recompose_after = false;
    }

    fn finish(mut self) {
        let mut table = self.composer.finish();
        // Reorder the child nodes based on the order they appear in the scope table.
        //
        // For instance, given this initial state:
        //      `table`               | `self.node.children`
        //      ----------------------------------------------
        //      Node(index=3, Key C)  |  [1] Node A
        //      Node(index=1, Key A)  |  [2] Node B
        //      Node(index=2, Key B)  |  [3] Node C
        //
        // The final state is:
        //      `table`               | `self.node.children`
        //      ----------------------------------------------
        //      Node(index=1, Key C)  |  [1] Node C      (3->1)
        //      Node(index=2, Key A)  |  [2] Node A      (1->2)
        //      Node(index=3, Key B)  |  [3] Node B      (2->3)
        for child in self.node.children.iter_mut() {
            child.child_index = usize::MAX;
        }
        let mut i = 0;
        for e in table.iter_mut() {
            if let CompositionSlot::Node { child_index, .. } = e {
                let prev_index = mem::replace(child_index, i as u32) as usize;
                self.node.children[prev_index].child_index = i;
                i += 1;
            }
        }
        self.node.children.sort_by_key(|n| n.child_index);

        if i < self.node.children.len() {
            // remove the extra nodes
            self.node.children.truncate(i);
            // some child nodes were removed, rebuild the child filter from scratch
            self.node.child_filter.clear();
            for c in self.node.children.iter() {
                self.node.child_filter.add(&c.id);
            }
        }

        // propagate child filters up to this node (their children might have changed)
        for c in self.node.children.iter() {
            self.node.child_filter.extend(&c.child_filter);
        }

        // place the scope table back in the node
        self.node.composition_table = table;
    }
}

/*#[track_caller]
pub fn with_environment<K: EnvKey, V: EnvValue>(ctx: &mut CompositionCtx, key: K, value: V, inner: impl FnMut(&mut CompositionCtx))
{
    // since all keys have different types, a single call site cannot set a different key
    ctx.with_state_no_recomp(Environment::new, |ctx, child_env| {
        let parent_env = ctx.environment();
        if ctx.has_changed(&parent_env) || ctx.has_changed(&value) {
            *child_env = parent_env.add(key, value);
        }
        ctx.with_environment(child_env, inner);
    });
}*/

impl Node {
    /// Runs recomposition.
    pub(crate) fn recompose(
        &mut self,
        app_ctx: &mut AppCtx,
        event_loop: &EventLoopWindowTarget<()>,
        env: Environment,
        mut f: impl FnMut(&mut CompositionCtx),
    ) {
        // temporarily remove the table to avoid borrowing headaches
        let window_id = self.window_id();
        let table = mem::replace(&mut self.composition_table, Vec::new());
        let composer = Composer::new(table);
        let mut ctx = CompositionCtx {
            app_ctx,
            event_loop,
            parent_window_id: window_id.or(self.parent_window_id),
            env,
            no_skip: false,
            node: self,
            composer,
            recompose_after: false,
            rebuild_child_filter: false,
        };
        // keep recomposing until `recompose_after == false`
        loop {
            f(&mut ctx);
            if !ctx.recompose_after {
                break;
            }
            ctx.restart();
        }
        ctx.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Dummy;

    #[test]
    fn test_scope() {
        let mut table = Vec::new();
        for i in 0..4 {
            eprintln!("====== Composition {} ======", i);
            unsafe {
                let mut c = Composer::new(table);
                c.enter(Key::from_caller(0));
                c.emit_node(Key::from_caller(0), 0);
                c.emit_node(Key::from_caller(0), 0);
                c.enter(Key::from_caller(0));
                if i < 2 {
                    // leaves at 2
                    c.emit_node(Key::from_caller(0), 0); //
                }
                if i > 1 && i < 3 {
                    // appears at 2, leaves at 3
                    c.emit_node(Key::from_caller(0), 0); //
                }
                if i > 2 {
                    // appears at 3
                    c.emit_node(Key::from_caller(0), 0);
                }
                c.exit();
                c.emit_node(Key::from_caller(0), 0);
                c.exit();
                table = c.finish();
                dump_composition_table(&table);
            }
        }
    }

    #[test]
    fn test_reorder() {
        let mut table = Vec::new();
        let mut c = Composer::new(table);

        for i in 0..10 {
            c.enter(Key::from_caller(i));
            unsafe {
                c.emit_node(Key::from_caller(0), 0);
            }
            c.exit();
        }

        table = c.finish();
        dump_composition_table(&table);
    }
}
