use crate::key::Key;
use std::any::Any;
use crate::widget::Node;

enum EntryKind {
    ScopeStart,
    ScopeEnd,
    State(Box<dyn Any>),
    Node(Box<Node>) // Node could be unsized
}

struct Entry {
    key: Key,
    kind: EntryKind
}

struct ScopeTable {
    entries: Vec
}