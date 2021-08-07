
# Designing a data model

## Goals
- Easy implementation of undo/redo in applications (no additional user code needed)
    - no need for the command pattern
- Load/save from a file comes for free
- Automatically ensures the consistency of the data
- Extensible: plugins can "plug into" this data model by associating data

## Ideas
- take inspiration from veda; what worked, what was clunky
- data model is an entity-component database
    - or, more simply, a database (entity = primary key, component type = table, component instance = row)
- Entities are fundamental
- Entities exist in a Database
- Components attached to entities
- Can remove entities, and all components are removed when an entity is removed
- Components are stored within the database
- Schemas correspond to a set of components attached to an entity
- Components can be unsized types and trait objects (dyn Trait)
- There can be only one instance of a component type on an entity

* Assume that every operation can be done by an end-user
* Every transaction should result in a valid data model state
* The database should be easily introspectable

- The basic operations are:
  - Create/Delete an entity
  - Create an entity from a schema
  - Add a component to an entity
  - Remove a component
  - Modify a component

- Databases should maintain coherence:
  - Assume that a component refers to two other entities (relationship)
  - If one of the entities is removed, remove the component
  - (DB: on delete cascade)

- Components are Objects
- Objects can be reflected:
  - Iterate over fields
- Don't pay for what we don't use

- Undo/redo should be supported without too much extra code
  e.g. component.set_xxx(value, &mut edit)

- problem: representing an operation on a complex data model in an undo command
    - path = value
    - If directly modifying the value through a mut reference, there's no way of preserving the previous state.
    - Lenses, possibly?

## Lightweight lenses
Concretely, refer to an element with a _string path_, as a form of type erasure.
From a `&str` return a `&dyn Any` that represent an element inside a bigger data structure.
Path lookup is automatically implemented via a procedural macro for structs, or impl manually for types like `Vec<T>`.


The main advantage is that it can be used dynamically, "outside" of a compiled program.
One example would be an external GUI description that binds to items in the data model with paths.

We lose the efficiency of addresses and typed lenses (more dynamic checks). However it _might_ be possible to add
a "typed" wrapper over paths that can skip some checks.

### Example
Given the following definitions:
```rust
#[derive(Data)]
struct Root {
    nodes: HashMap<String, Entry>,
}

#[derive(Data)]
struct Entry {
    value: i32
}
```

Then the path `.nodes.[name].value` on an instance of `Root` resolves to a
reference to the field `value` of entry `name` in the `nodes` HashMap.

The equivalent calls to resolve this path would be:
```rust
fn resolve(root: &Root) -> &dyn Data {
    let nodes = root.lookup_field("nodes")?;
    let v = nodes.lookup_entry("name")?;
    let value = v.lookup_field("value")?;
    return value;
}
```

Note that there is no concept of "type" within a path: any syntactically-valid path should be considered valid until
proven otherwise (i.e. resolves to `None`).

There _could_ be a concept of run-time type annotations to encode expectations about the type at some path, e.g.
`.nodes.[name].value:i32`


### Lenses and components?

### Goals
Don't forget the main goal: UI should be easy and quick to build. Strive for a dear ImGui-like experience.
Minimal boilerplate.

A UI designer is too much work. Is it possible to reuse one?
- Expression Blend
    - Needs parsing of XAML

### Parse XAML?
- Need to support
    - Grids
- What workflow?
    - at compile time, take XAML and turn it into a `Widget` taking a `&mut DataContext`.
    - two-way bindings?
        - bit more difficult
- XAML static resources:
    - Key -> Value pair
    - Resources are associated to an element
        - Globally on the application, on the container, on leaf elements...
        - Resources in parent visible to the children
        - Resource lookup necessary
        - Like druid:Env?
    - Styling:
        - Style == Collection of attributes
    - Template:
        - template == `fn (|context| -> Widget) -> Widget`
        - "higher-order" widget
    - Animation:
      
    - Mapping to rust:
        - Simple data => translate to constants
        - Strings => &'static str
        - Geometry => Paths or whatever

- Conclusion: too complicated
    - start with a bespoke description language with interactive update
    - could also put rust code directly inside

### Bespoke UI description language (kyute-iml)
- Describes a `Widget`
- Dynamically loaded widget:
    - `ImlWidget::new` takes a `&mut DataSource` as input and a property dictionary:
- `DataSource`
    - Automatically reflected trait

## Consider Druid again
- Good font rendering, rich text
- Native menus
- Authors seem to know what they're doing

# UI Node v2

Kyute currently makes a distinction between the widget tree and the node tree. The widget tree can be considered
as a structured "program" to update the retained node tree. 
The widget tree is short-lived, because it needs to borrow the data model.

Alternative design:
- fuse widget and node
- use light-weight lenses (automatically generated) to access inner parts of the data model without
    needing to traverse the whole thing
  
## Components


## Rendering
* Does the library own the window? Yes.
* Does the library own a vulkan context? Yes.
    * Does it get it through graal? Yes.
* Does the library own the event loop? Yes.

Basically, the UI "shell" framework (kyute-shell) creates and owns the graal/vulkan context along with various "native" service instances 
(previously this was text, 2D drawing, imaging, but now those are handled by skia). 
A custom configuration for the graal context can be requested. 

Kyute sits on top of kyute-shell and owns the event loop. It owns windows and draws the UI on them. 
A custom widget can be used to render arbitrary things, with a callback that is passed a `graal::Frame`

## Nodes 
- In C++, that would be a tree of objects with a common base class
- In Rust, that's more complicated
    - old kyute used Node+Box<dyn Visual>
        - downcasting is inconvenient (need to define as_any, as_any_mut)
- what node-specific behavior is in the node?
    - rendering
    - layout (given BoxConstraints, return Measurements)
    - hit-testing
    - local state:
        - slider position
        - etc.
      
    - when do we need to downcast?
        - previously, it was necessary to downcast in Widget::layout because each widget had its own visual type with
          all sorts of data in it, and wanted to do its own reconciliation on it.
        - replace with a fixed set of ~~elements~~ payloads?
            - text, or div, like xxgui
            - and then, Box<Any> for the attributes, attached to a **position** in the tree (i.e. can be between nodes) 

## Building widgets on demand
The `Layout` method of `Element` should be able to modify the list of children.

## Cross-platform?
If we need cross-platform capabilities, then use skia. Otherwise, use Direct2D.
2D drawing will be behind kyute_shell::drawing anyway, so that's not a problem.

## TODO
- re-enable 2D drawing ASAP on kyute-shell
- disable (comment) graal interop for now
- develop a sample "tree-editing" application on top of kyute


## The big picture

A GUI application is composed of two elements: the data, and the GUI.
There's some kind of function `F(Data) -> GUI` that produces the GUI from the data.
The data changes over time, but re-evaluating the function and re-building the result may be costly, so we want to do
things *incrementally*: i.e. from a small change in the data, make a small change to the previously returned GUI to
make it match the current data, instead of rebuilding it from scratch.

It is also important to not destroy parts of the GUI that don't change because some GUI elements have *internal state*
that we want to keep as much as possible across data updates
(e.g. the position of a scrollbar, the currently selected tab in a tab view, whether an accordion is collapsed or not...).

There are several ways to perform incremental updates:
- don't: rebuild everything from scratch every time
    - examples: imgui
- events-based: whenever a part of the data changes, an event is emitted, which is handled by parts of the UI that depend
  on the data. Those parts then update themselves with the new data.
    - examples: Qt, and a lot more
- traversal: the GUI holds the previous version of the data that it depends on. Whenever the data changes, the GUI and the data are
  traversed simultaneously and the widgets update themselves if their data is out of date.
    - This supposes that each piece of data is cheap to copy and compare (value types). Having data like this has other advantages, however.
    - examples: druid
- reconciliation: same as the traversal, but instead of comparing against the previous data, compare against the created widget.
    - examples: react, druid/crochet

The druid approach seems to work rather well. The restrictions on the data model may seem harsh, but they bring advantages
like easy undo/redo.


Traversal: need to store a copy of the data, so must be templated by the type
    - This complicates hot-reload

Reconciliation: doesn't, because it doesn't need to store a copy of the data

Modifying the tree:
- overwrite widgets?
- mutate widgets?

Issue: state
- where to put it? as a widget in the child list? in a hash map?
-> in a "Scope" (wraps internal state)
  
```
if_changed(data, |cx| {
    with_state<T>(|cx,internal_state| {
        
    })
})
```

Question:
- container-owns VS framework-owns
- container-owns: the container widget owns the list of children in WidgetPods and also the state necessary for subcomposition
    - since the logic for that is complicated, widgets are probably going to use one or two predefined widget+state containers
    - the container can access its children at any time
    
- framework-owns: the framework owns the tree of nodes
    - widgets don't control their children anymore
    - one less field in widget structs
    - must pass the children in the contexts when calling the widget interface
        - thus, harder to call child widgets manually
    
- alternatively: store the children in the WidgetPod (==Node)?


Recap:
- the call tree is complicated, and its structure differs from the node tree
- we could store the call tree in the nodes, but that would  
-> instead of storing state/scopes in the node, store them in a flat gap buffer, the ScopeTable (like jetpack compose)


Why not store a call tree per node, instead of a flat one?

# Events, actions and relayouts
How/when to communicate widget actions?
During recomposition:

```
fn gui() {
    let mut text = cx.state::<ArcStr>();
    label(&text);
    text_box(&mut text); 
}
```

The label text must update when the text changes.
Mechanism:
- text is some kind of `RefMut`. When it is dropped, it looks in the table to see if its value has changed. If so, then
  recompose automatically.
    - basically loop recomposition until the state doesn't change anymore.
    
```rust
fn gui(cx) {
    let mut text : StateRefMut<ArcStr> = cx.state::<ArcStr>();
    
    hbox(cx, |cx| {
        label(cx, &text);
        text_box(&mut text);
    });
    
    // for each state entry:
    if StateRefMut::changed(&text) {
        cx.request_composition();
    }
}
```

Returning a direct `&mut T` to the state in intractable. Other options:
- return a clone (via a proxy), and write back the result when exiting the composition scope.
    - issue: ensuring that the proxy doesn't escape the composition scope: it needs a borrow, which prevents concurrent modification of the table.
    - introducing a scope (with a closure) would work, but makes the API a bit verbose and impractical:
    
```rust
fn gui(cx) {
    cx.with_state(|cx, text: &mut ArcStr| {
        hbox(cx, |cx| {
            label(cx, &text);
            text_box(&mut text);
        });     
    });
}
```

# Dynamically adding child nodes in events:
- Right now we can only add/remove nodes during composition
- but a widget can trigger a recomposition as a result of an event
    
# Event delivery
- need to deliver some events to a particular node, or node chain
    - how to route an event quickly though the chain?
        - druid has bloom filters to quickly filter subtrees that may contain a specific children (with relatively high probability)
    - alternatively: change node hierarchy so that it's randomly accessible
        - this can make the Widget API a bit more complicated (i.e. no more `children: &mut [Node]`)
- keep a map?
    - node ID -> child indices
    
- Note that some widgets don't even need an ID: those that can't be focused, and 

- Input events handled by the root application, not by windows
    - winit events are forwarded to the corresponding widget with `window_event`
    - however, the framework needs to know the DPI of the target surface to perform the conversion from physical coordinates to logical coordinates
        - call the node?
    
- Programmatic window resize?
    - request relayout
