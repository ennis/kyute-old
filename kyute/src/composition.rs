//! TODO this should be renamed because "composition" is also a term related to text input
use crate::{
    application::AppCtx,
    core::{Widget, WidgetDelegate},
    data::Data,
    env::{EnvKey, Environment},
    key::CallKey,
    EnvValue, NodeId,
};
use kyute_shell::{
    window::PlatformWindow,
    winit::{event_loop::EventLoopWindowTarget, window::WindowId},
};
use tracing::trace;
use std::{any::Any, mem};

/// Type-erased state stored in the composition table of a node.
pub struct State {
    key: CallKey,
    data: Box<dyn Any>,
}

/// An entry in a composition table.
pub(crate) enum CompositionSlot {
    /// Marks the start of a scope.
    ScopeStart {
        /// Scope length including this entry and the scope end.
        // u32 to reduce size of `Entry`
        len: u32,
        key: CallKey,
    },

    /// Marks the end of a scope.
    ScopeEnd { key: CallKey },

    /// Represents a node.
    Node {
        /// Index of the node in the child list of the node that holds this table.
        // u32 to reduce size of `Entry`
        child_index: u32,
        key: CallKey,
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
    fn find(entries: &[CompositionSlot], key: CallKey) -> Option<usize> {
        let mut i = 0;
        while i < entries.len() {
            match &entries[i] {
                CompositionSlot::ScopeEnd { .. } => break,
                CompositionSlot::ScopeStart { key: this_key, .. } if this_key == &key => {
                    return Some(i)
                }
                CompositionSlot::Node { key: this_key, .. } if this_key == &key => return Some(i),
                CompositionSlot::State(s) if s.key == key => return Some(i),
                _ => i += entries[i].len(),
            }
        }
        None
    }

    /// Rotates an entry in the current scope with the specified key so that it ends up at the current position.
    /// Returns whether the entry was found and rotated in place.
    fn rotate(&mut self, key: CallKey) -> bool {
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
    fn rotate_node(&mut self, key: CallKey) -> Option<u32> {
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
    fn rotate_state(&mut self, key: CallKey) -> Option<&mut State> {
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
    fn current_scope_key(&self) -> CallKey {
        match &self.table[self.scope_start.unwrap()] {
            CompositionSlot::ScopeStart { key, .. } => *key,
            _ => panic!("expected scope start"),
        }
    }

    /// Enters a composition scope. Must be matched with a call to `exit`.
    /// Returns true if the entry wasn't there before and was just created.
    fn enter(&mut self, key: CallKey) -> bool {
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
    fn skip(&mut self) {
        self.pos += self.table[self.pos].len();
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
        key: CallKey,
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

    /// Resets the writing position to a prior location
    fn rewind(&mut self, position: usize) {
        assert!(position <= self.pos);

        // check that the position to rewind to within the current scope.
        assert!(position >= self.scope_start.unwrap_or(0));

        // reset position
        self.pos = position;
    }

    /// Writes a state entry at the specified position.
    fn write_state(&mut self, pos: usize, data: Box<dyn Any>) {
        match self.table[pos] {
            CompositionSlot::State(ref mut state) => {
                state.data = data;
            }
            _ => panic!("unexpected entry type"),
        }
    }

    /// Emits a node.
    /// Returns the index in the list of child nodes (this is *not* the table position).
    fn emit_node(&mut self, key: CallKey, parent_child_count: usize) -> usize {
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
        // check for balancing of calls to enter/exit
        let mut level = 0;
        for entry in self.table.iter() {
            match entry {
                CompositionSlot::ScopeStart { .. } => {
                    level += 1;
                }
                CompositionSlot::ScopeEnd { .. } => {
                    level -= 1;
                }
                _ => {}
            }
        }
        assert_eq!(level, 0);

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
unsafe fn downcast_widget_unchecked<T: WidgetDelegate>(widget: &mut dyn WidgetDelegate) -> &mut T {
    &mut *(widget as *mut dyn WidgetDelegate as *mut T)
}

///
pub struct ActionResult(Option<Box<dyn Any>>);

impl ActionResult {
    pub fn cast<T: Any>(self) -> Option<T> {
        self.0.and_then(|x| x.downcast::<T>().ok()).map(|x| *x)
    }
}

impl Default for ActionResult {
    fn default() -> Self {
        ActionResult(None)
    }
}

/// Context passed to composable functions that produce nodes.
pub struct CompositionCtx<'a, 'node> {
    app_ctx: &'a mut AppCtx,
    event_loop: &'a EventLoopWindowTarget<()>,
    parent_window_id: Option<WindowId>,
    action: Option<(NodeId, Box<dyn Any>)>,
    action_target_path: &'node [CallKey],
    env: Environment,
    /// Never skip recomposition of a composable even if its parameters did not change.
    /// This is usually set to `true` when a theme variable has changed.
    no_skip: bool,
    node: &'node mut Widget,
    composer: &'node mut Composer,
}

impl<'a, 'node> CompositionCtx<'a, 'node> {
    pub fn skip(&mut self) {
        self.composer.skip();
    }

    /// Enters a composition scope. Must be matched with a call to `exit`.
    #[track_caller]
    pub fn enter(&mut self, id: u64) -> bool {
        let key = CallKey::from_caller(id);
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
        T: WidgetDelegate,
        Init: FnOnce(&mut InitCtx) -> T,
        Update: FnOnce(&mut UpdateCtx, &mut T),
        Contents: FnMut(&mut CompositionCtx),
    {
        let key = CallKey::from_caller(0);
        unsafe { self.do_emit_node(key, init, update, contents) }
    }

    #[track_caller]
    pub fn has_changed<T: Data>(&mut self, data: T) -> bool {
        self.with_state(|| data.clone(), |_cx, prev_data| { !prev_data.same(&data) })
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
    pub fn with_state<T, F, R, Init>(&mut self, init: Init, mut f: F) -> R
    where
        T: Data,
        Init: FnOnce() -> T,
        F: FnOnce(&mut Self, &mut T) -> R,
    {
        let key = CallKey::from_caller(0);
        let _span =
            tracing::trace_span!("with_state", location = key.caller_location().file()).entered();

        let (index, mut data) = self.composer.extract_state(key, init);

        // safety: by construction, if the key matches, then the call site is the same and thus
        // the statically known type T is the same.
        let mut old_data = data.as_mut().downcast_mut::<T>().unwrap();
        let mut new_data = old_data.clone();
        let result = f(self, &mut new_data);
        if !old_data.same(&new_data) {
            *old_data = new_data;
            // FIXME: we should be able to request a recomposition of this scope only
            self.app_ctx.request_recomposition();
        }

        // put the state back in place
        self.composer.write_state(index, data);
        result
    }

    /// Same as with_state, but don't recompose if the value changes.
    #[track_caller]
    pub fn with_state_no_recomp<T, F, Init>(&mut self, init: Init, f: F)
    where
        T: Data,
        Init: FnOnce() -> T,
        F: FnOnce(&mut Self, &mut T),
    {
        let key = CallKey::from_caller(0);
        let (index, mut data) = self.composer.extract_state(key, init);
        let mut data_ref = data.as_mut().downcast_mut::<T>().unwrap();
        f(self, data_ref);
        self.composer.write_state(index, data);
    }

    /*/// Requests a recomposition when after this ctx is finished (because e.g. some state has changed).
    pub fn request_recomposition(&mut self) {
        self.recompose_after = true;
    }*/
}

fn create_node<T: WidgetDelegate>(
    app_ctx: &mut AppCtx,
    key: CallKey,
    event_loop: &EventLoopWindowTarget<()>,
    parent_window_id: Option<WindowId>,
    init: impl FnOnce(&mut InitCtx) -> T,
    env: &Environment,
) -> Widget {
    let node_id = NodeId::next();
    let mut init_ctx = InitCtx {
        node_id,
        app_ctx,
        event_loop,
        window: None,
    };
    let widget = Box::new(init(&mut init_ctx));
    let mut node: Widget = unsafe {
        // safety: node_id was created with `NodeId::next()`
        Widget::new(
            widget,
            node_id,
            key,
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
    fn do_enter(&mut self, key: CallKey) -> bool {
        self.composer.enter(key)
    }

    fn do_exit(&mut self) {
        self.composer.exit();
    }

    unsafe fn do_emit_node<T>(
        &mut self,
        key: CallKey,
        init: impl FnOnce(&mut InitCtx) -> T,
        update: impl FnOnce(&mut UpdateCtx, &mut T),
        contents: impl FnMut(&mut CompositionCtx),
    ) -> ActionResult
    where
        T: WidgetDelegate,
    {
        let child_index = {
            let child_count = self.node.children.len();
            let child_index = self.composer.emit_node(key, child_count);
            if child_index == child_count {
                let node = create_node(
                    self.app_ctx,
                    key,
                    self.event_loop,
                    self.parent_window_id,
                    init,
                    &self.env,
                );
                let id = node.id;
                trace!("add node {:?} [{} @ {}]", node.id, node.debug_name(), node.key);
                self.node.children.push(node);
                self.node.child_filter.add(&id);
                self.app_ctx.request_relayout();
            } else {
                let node = &mut self.node.children[child_index];
                // process all pending actions first
                if self.action.as_ref().map(|a| a.0) == Some(node.id) {
                    trace!(?node.id, "returning action");
                    return ActionResult(Some(self.action.take().unwrap().1));
                }
                // SAFETY: ensured by the `do_emit_node` call contract.
                let t = downcast_widget_unchecked::<T>(node.widget.as_mut());
                let mut update_ctx = UpdateCtx {
                    app_ctx: self.app_ctx,
                    event_loop: self.event_loop,
                };
                update(&mut update_ctx, t);
            }
            child_index
        };

        let node = &mut self.node.children[child_index];

        // recurse only if we're on the target path, or if we're doing a full recomp (`self.target_path == &[]`)
        match self.action_target_path.split_first() {
            None => {
                node.recompose_impl(
                    self.app_ctx,
                    self.event_loop,
                    self.env.clone(),
                    self.action.take(),
                    &[],
                    contents,
                );
            }
            Some((first, rest)) if first == &node.key => {
                node.recompose_impl(
                    self.app_ctx,
                    self.event_loop,
                    self.env.clone(),
                    self.action.take(),
                    rest,
                    contents,
                );
            }
            _ => {}
        }

        ActionResult(None)
    }

    /// Restarts the composition of this node.
    fn restart(&mut self) {
        self.composer.restart();
    }
}

struct CompositionTarget<'a> {
    path: &'a [CallKey],
    action: Box<dyn Any>,
}

impl Widget {
    /// Reorders `self.children` based on the order they appear in the composition table. Removes
    /// all nodes that are not referenced in the composition table.
    fn reorder_and_truncate_child_nodes(&mut self) {
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

        for child in self.children.iter_mut() {
            child.child_index = usize::MAX;
        }
        let mut i = 0;
        for e in self.composition_table.iter_mut() {
            if let CompositionSlot::Node { child_index, .. } = e {
                let prev_index = mem::replace(child_index, i as u32) as usize;
                self.children[prev_index].child_index = i;
                i += 1;
            }
        }
        self.children.sort_by_key(|n| n.child_index);

        if i < self.children.len() {
            // remove the extra nodes
            self.children.truncate(i);
            // some child nodes were removed, rebuild the child filter from scratch
            self.child_filter.clear();
            for c in self.children.iter() {
                self.child_filter.add(&c.id);
            }
        }

        // propagate child filters up to this node (their children might have changed)
        for c in self.children.iter() {
            self.child_filter.extend(&c.child_filter);
        }
    }

    /// Recomposes the children of this node with the given closure.
    ///
    /// # Arguments
    ///
    /// * `app_ctx` - global application context
    /// * `event_loop` - application event loop proxy, used to crete new windows
    /// * `env` - composition environment
    /// * `action_target_path` - an optional _key path_ to a specific target node that needs to be recomposed.
    /// If `action_target_path` is not `None`, recomposition will skip all nodes and scopes that are not on the
    /// key path.
    /// * `f` - composition closure
    fn recompose_impl(
        &mut self,
        app_ctx: &mut AppCtx,
        event_loop: &EventLoopWindowTarget<()>,
        env: Environment,
        action: Option<(NodeId, Box<dyn Any>)>,
        action_target_path: &[CallKey],
        f: impl FnOnce(&mut CompositionCtx),
    ) {
        // We usually have `target != None`, when a node has emitted an action that needs to be
        // processed by the composition logic.
        // In this case, we recursively traverse the composition tree until we reach the target,
        // and ignore the rest.
        // The composition logic will most likely mutate state entries as a result, which warrants
        // another recomposition afterwards: those follow-up recompositions ignore the target
        // path.

        // temporarily remove the table from the node so that we can also pass a borrow of the node
        let window_id = self.window_id();
        let table = mem::replace(&mut self.composition_table, Vec::new());
        let mut composer = Composer::new(table);

        {
            let has_action = action.is_some();

            let mut ctx = CompositionCtx {
                app_ctx,
                event_loop,
                parent_window_id: window_id.or(self.parent_window_id),
                action,
                action_target_path,
                env,
                no_skip: false,
                node: self,
                composer: &mut composer,
            };
            f(&mut ctx);

            if ctx.action.is_some() && has_action {
                tracing::warn!(?self.id, "action has not been delivered");
            }
        }

        self.composition_table = composer.finish();
        self.reorder_and_truncate_child_nodes();
    }

    /// Recomposes the children of this node with the given closure.
    ///
    /// # Arguments
    ///
    /// * `app_ctx` - global application context
    /// * `event_loop` - application event loop proxy, used to crete new windows
    /// * `env` - composition environment
    /// * `f` - composition closure
    pub(crate) fn recompose(
        &mut self,
        app_ctx: &mut AppCtx,
        event_loop: &EventLoopWindowTarget<()>,
        env: Environment,
        f: impl FnOnce(&mut CompositionCtx),
    ) {
        self.recompose_impl(app_ctx, event_loop, env, None, &[], f);
    }

    /// Recomposes the children of this node as a result of an action emitted by a child node.
    ///
    /// # Arguments
    ///
    /// * `app_ctx` - global application context
    /// * `event_loop` - application event loop proxy, used to crete new windows
    /// * `env` - composition environment
    /// * `target` - the node that emitted the action and that needs to be recomposed.
    /// Recomposition will skip all nodes and scopes that are not on the path to the target.
    /// * `action` - the action emitted by the node
    /// * `f` - composition closure
    pub(crate) fn recompose_on_action(
        &mut self,
        app_ctx: &mut AppCtx,
        event_loop: &EventLoopWindowTarget<()>,
        env: Environment,
        action_target: NodeId,
        action: Box<dyn Any>,
        f: impl FnOnce(&mut CompositionCtx),
    ) {
        if let Some(target_key_path) = self.key_path_to_child(action_target) {
            tracing::trace!(
                "recomposing on action: target key path {:?}",
                &target_key_path
            );
            self.recompose_impl(
                app_ctx,
                event_loop,
                env,
                Some((action_target, action)),
                &target_key_path,
                f,
            );
        } else {
            tracing::warn!("invalid target for action");
        }
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
                c.enter(CallKey::from_caller(0));
                c.emit_node(CallKey::from_caller(0), 0);
                c.emit_node(CallKey::from_caller(0), 0);
                c.enter(CallKey::from_caller(0));
                if i < 2 {
                    // leaves at 2
                    c.emit_node(CallKey::from_caller(0), 0); //
                }
                if i > 1 && i < 3 {
                    // appears at 2, leaves at 3
                    c.emit_node(CallKey::from_caller(0), 0); //
                }
                if i > 2 {
                    // appears at 3
                    c.emit_node(CallKey::from_caller(0), 0);
                }
                c.exit();
                c.emit_node(CallKey::from_caller(0), 0);
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
            c.enter(CallKey::from_caller(i));
            unsafe {
                c.emit_node(CallKey::from_caller(0), 0);
            }
            c.exit();
        }

        table = c.finish();
        dump_composition_table(&table);
    }
}
