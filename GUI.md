
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


# Environment a.k.a "ambient values"
- during recomp, override a specific value in the environment, identified by a key
- store the previous value; if it is different, force a recomp

# Handling events:
- events should be handled during comp, 

# Modifiers: 
- used for layout (add padding, align in parent)
- used for painting backgrounds
- used to add event handlers

Issue: modifiers can be passed as a parameter to a composable function, so they need to implement `Data`.
Maybe just do pointer equality, as with Environment?

# Pending issues:
- simple widgets for background painting, layout adjustments
- handling events during recomp
  - ideally we would want to handle event during recomp, where we have access to state
  - however, in the current strategy, recomp is skipped if the parameters are the same
    - how does jetpack compose works?
      - JC supports true partial recomposition: each composition "block" (a call of a composable function)
        can be called again independently.
        - Since arguments to a composable function are stored in the comp table, can just call the function again.  
      - "Naked" mutable state is not supported (mutation happens in callbacks, not during recomp; recomp is triggered because of a state change)
        

# When to use composable functions vs widgets

Sometimes, it's simply more convenient to implement behavior in the widget implementation. For instance, if you have
a widget with many different parts that do not react to events, it might be more convenient to draw them all at once 
in the widget implementation. 

# Conflict between internal and external state during recomposition when an action is emitted
Consider a text editor widget.
there are two copies of the string:
 - the one stored by the user as part of their app state: "user (or external) state"
 - the internal string stored by the LineEdit: "internal state"

 1. LineEdit is created with a string "A"
 2. a character is inserted, the internal state is set to "AB"
 3. this triggers recomposition: text_line_edit is called again, 
    with the user string, which still contains "A"
 4. since user state ("A") != internal state ("AB"), internal state is updated to "A" => the internal selection is cleared
 5. the "TextChanged" action is dequeued: user state is updated to "AB"
 
Solution: in emit_node, don't run update() if there are pending actions.

# Architecture decision: store actions in a top level queue
Don't store actions within the node, and don't process all actions within the same composition pass: this causes problems.
Instead, it is simpler to add actions to a top-level queue and process them in sequence.


# Widgets
- drop downs
- spinners
- 3D view
- color picker
- text edit
  - I-beam cursor on hover 

# Investigate the possibility of recomposing locally
Currently, whenever a composable should be re-run, parent composable must run as well, since there's no way to run the composable function in isolation.
To do so, we'd need to store a copy of the function parameters (in a way, a copy of the whole invocation).
This would require all arguments of composables to be `Data`. It also means that it would be impossible to pass things
by reference (no more `&str`, must use `Arc<str>`, etc.)

Pros:
- recomposition can be done locally: each scope owns a copy of the invocation, which can be run independently of the parent.

Cons:
- composables cannot "return" anything: how can they modify state?

Widget delegates directly call handlers, which update parts of the state
-> any mutable state must be wrapped
-> we end up with lenses, which defeats the purpose of composables in the first place 

# Focus/defocus
1. mouse event is propagated to a target widget
2. event queue for this widget is invalidated, forcing a re-evaluation of a new revision of the widget tree
3. during recomp, `TextEdit::new` sees the event and acquires the focus (somehow)
4. the previously focused item in the window is invalidated, forcing another re-evaluation 

Cached values:
- Widget: event queue
- Widget: hovered
- Widget: focused

F(AppState) -> GUI: not enough
F(AppState, InternalState) -> GUI
focus is in internal state



```rust
fn widget() -> Widget<Button> {
    
    let b = Button::new();
    
    if b.focused() {
        // do stuff if button is focused 
    } else {
        // stuff if unfocused
    }
    
    if b.hovered() {
        // stuff if hovered
    }
}

struct WidgetBox<T: ?Sized + WidgetDelegate> {
    hovered: State<bool>,
    focused: State<bool>,
    delegate: T
}

struct Widget<T>(Arc<WidgetBox<T>>);

// widgets don't need to be stored in the cache:
// we only need to signal that the value computed by the function might not be valid anymore

impl Button {
    #[composable(synthetic)]
    pub fn new() -> Widget<Button> {

        // makes the parent value (Widget<Button>) dependent on the focus state
        let widget_state = WidgetState::new();
        let events = widget_state.take_events();
        
        // ... iterate over events ...
        widget_state.acquire_focus();
        
        Widget {
            focused,
            hovered,
            delegate: Button::new(...),
        }
        
    }
}


impl Widget<Button> {
    pub fn focused(&self) -> bool {
        self.0.focused
    }
}

```

# Widget tree
Could be:

## Option A: an owned object, non-cloneable, fully rebuilt on each recomposition
- if so, avoid doing too many recomps
- widgets can't be cached
- costly parts could be wrapped in Arc so that they may be cached/shared between comps

## Option B: `Arc<WidgetImpl>`: fully immutable, shareable data object
- no interior mutability: not easy to cache data inside, must use an map outside

## Option C: `Arc<RefCell<WidgetImpl>>`: shareable with interior mutability
- somewhat error prone?

## Issues:

### Sending targeted events
- e.g keyboard events to currently focused item

With options B and C, could in theory keep an weak ref to the widget.
With option A, widgets are only identifiable by ID, and can only reach it through a traversal, for which the delegate must cooperate.
The traversal could be accelerated by a bloom filter, but this requires additional bookkeeping.

- druid: bloom filter
- iced: no targeted events (full traversal on every event)

Let's go with option B. How to efficiently deliver targeted events?
(delivery = finding the widget with a specified key and calling `Widget::event` on it)
A traversal is necessary, unless a more complete widget graph (with parent links) is created. 
However, creating a more complete widget graph has some overhead (memory and syntactical).
Note that such a graph already exists: it's the cache dependency graph.

### Keeping widget identity across recomps
With option A, the widget object has no identity across recomp: given two `Widget` objects, we initially have no way of telling whether
they are actually the same widget but at two different times. To do so, we must add some key: it can be a CallKey or
a synthesized widget ID. The CallKey seems more appropriate.

With option B, same thing: a widget's state can change across recomp, but still keep its own "identity", so we must use a key.

With option C: the identity is the pointer in the Arc itself. Since we use interior mutability, we can mutate the widget state
without affecting its identity.

### TextEdit
A text editor widget should work like this:
- create a TextEdit object with some string
- layout & render
- propagate events:
  - if key event, convert to char, send updated string 
  - this triggers a recomp
- recomp catches the updated string and updates some internal state entry, which in turn invalidates more cache entries  

# High-level architectures
1. A fully retained, stateful, mutable widget tree with a functional layer on top
  - functions return tree mutations instead of full widgets
2. Moxie-style (maybe?), functions return fully formed widget, widget identity with call keys, cache with dependency tree 

```rust
#[composable]
fn labeled_widget(label: &str) -> Widget {
    let mut slider_value = Context::state(|| 0.0);
    let slider = Slider::new(slider_value.get());
    // issue: if slider_value changes in the next line, the label will be built for nothing 
    let label = Label::new(format!("{}_{}", label, slider_value.get()));
    slider.on_change(|new_value| slider_value.set(new_value));
    HBox::from([label, vbox]).into()
}
```

```rust
#[track_caller]
fn labeled_widget(label: &str) -> Widget {
    // Rotates the table entries in the current group so that a given call key marker
    // ends up in the current slot, then go to the next slot.
    // If the call key wasn't found, insert a call key tag.
    Cache::group(move |dirty| {
        let mut changed = dirty;
        changed |= Cache::changed(label);
        
        let (value, slot) = Cache::expect::<Widget>();

        if !changed {
            // expect a widget
            // skip child group
            Cache::skip_to_end();
            value
        } else {
            // enter dependency group
            // we don't actually need to do a key search for the group here: either 
            // there's a group there, and enter it, or there's not, and create the group
            let result = {
                let mut slider_value = Cache::state(|| 0.0);
                let slider = Slider::new(slider_value.get());
                slider.on_change(|new_value| slider_value.set(new_value));
                let label = Label::new(format!("{}_{}", label, slider_value.get()));
                let invalidation_token = Cache::invalidation_token();
                HBox::from([label, vbox]).into()
            };
            
            Cache::set_value(slot, result);
        };
    })
}
```

Translates into the following slot table:
    
    0: StartGroup(<labeled_widget@0>)
    1:     Value(String)   // label: &str
    2:     Value(Widget)   // return value of <labeled_widget>
    3:     // dependency group of #2
    4:     Value(...)
            ....
    6: EndGroup()


    Tag         (<labeled_widget@0>)
    StartGroup 
    Flag        (dirty)
    Value       (String)
    Value       (Widget)
    Tag         (<labeled_widget@566>)
    Value       (f32)
    Tag         (<labeled_widget@567>)
    Value       (WidgetState)
    Value       (SliderState)
    Value       ...       
    EndGroup

# Final answer to the state question

The example: editing a list

```rust

pub type ItemList = Arc<Vec<Item>>;

struct ItemCollection {
    items: ItemList
}

#[composable]
fn item_collection_editor() {
    let mut list_state : State<Arc<Vec<Row>>> = Context::state(|| ...);
    let mut row_widgets = vec![];
    for row in list_state.get().iter() {
        let r = row_editor(row);
        row_widgets.push(r);
    }
    let add_item_button = Button::new("Add item");
    let remove_item_button = Button::new("Remove item");
    add_item_button.on_click(|| { list_state.set(list_state.get().clone().append(...)); });
    // finally, update the state (if it has changed?)
}

// Just pass a mutable row - no need to bother with proxies or lenses or bindings.
// We still need to detect changes to the row, so all types must be `Data`
#[composable]
fn row_editor(row: &mut Row) -> Widget {
    // do something with row
}
```

```rust
#[composable]
struct ItemCollectionEditor {
    #[state] items: ItemList,   // State<ItemList>
}

#[composable]
struct RowEditor {
    #[binding] row: Row,        // Binding<Row>
}

impl RowEditor {
    pub fn render(&mut self) -> Widget {
        
    } 
}
```

Issues:
1. passing bits of state to child composables:

```rust
fn list_editor() {
    let mut list_state : State<Arc<Vec<Row>>> = Context::state(|| ...);
    
    let mut row_widgets = vec![];
    for row in list_state.get().iter() {
        let r = row_editor(row);
        row_widgets.push(r);
    }
    
    let add_item_button = Button::new("Add item");
    let remove_item_button = Button::new("Remove item");
    
    // issue: must clone list_state in the closure, which is annoying
    add_item_button.on_click(|| { list_state.set(list_state.get().clone().append(...)); });
}

with_state::<ItemCollection>(list_editor);

fn row_editor(row: &Row) -> (Widget, Row) {
}


```

Options:

1. State mutation happens during composition => yes
   
2. State mutation happens during event propagation, in event callbacks
    - complicated: what if we want to edit a smaller part of a bigger structure?
    - swiftui: `@Binding`, which uses f*cking lenses under the hood (writablekeypath)
    - do we need lenses after all?
        - if we use lenses now, we might as well use druid's approach directly: it's cleaner and more principled
        - just use some macro magic on top to make lens composition more palatable


# External state
Two kinds of state:
- state that is accessed only during recomp: most of the state
    - not too complicated
- state that can be accessed and written outside of recomp (e.g. during event propagation) and that can invalidate values in the cache 

# Sending events
- Sending an event means:
  - setting some interior widget state
  - invalidating the dependent cache entries
  - re-evaluating the widget function



# The great UI challenges:
1. (Non-cloneability/minimally invasive) Allowing mutable state that does not need to be `Clone`
    1. (Non-comparability) Allowing mutable state that does not need to be `Data` (comparable)
2. (Lensing state) Views that access/mutate only parts of a bigger state
3. (Identity) Retaining widget identity
4. (Targeted events) Sending events to specific widgets in the tree efficiently
5. (Declarative) Express the UI declaratively, the user shouldn't have to write imperative code to update the UI structures.
6. (Incrementality) Rebuild only what's necessary on UI changes.
7. (Tooling) Plays well with existing tooling, such as IDE autocompletion
    - macro / DSL based solutions are suboptimal
    - stuff that only uses existing syntax is preferred
8. (Extensibility) Make it easy to create new widgets.
9. (Internal state) Components/views can have private retained internal state

=> avoid solutions that create more challenges!


Strategies to handle state modification:

1. watch how the state is modified and update the view accordingly: `dF(dState) -> dView; View += dView`
    1.1 restrict the ways the user can modify the state to primitives that the framework understands, 
        but let the user choose their own data model types (Vec, etc.), provided the framework knows how to modify them (unintrusive)
    1.2 the framework provides observable collection types that must be used in the data model (intrusive)
   
2. diff the state after it is modified and update the view from this diff: 
   - update: `dF(State(t+dt) - State(t)) -> dView; View += dView`
   - rebuild: `if State(t+dt) != State(t) then View = F(State(t+dt)) else View = F(State(t))`
   
3. don't watch the state, but instead watch the produced view and diff it with the retained view: `dView = F(State(t+dt)) - F(State(t)); View += dView`
    - rare? 
   

Strategy 1 is closer to building an incremental function; but like automatic differentiation, it's not possible for arbitrary functions:
- simple list-to-list mapping: easy
- list-to-list mapping with conditionals: OK

## Decision: continue with strategy 2.2 (Caching) or try strategy 1 (Reactive)?
The main difference is that caching needs comparable data models to be effective, while reactive can work with any data model.
However, caching may be able to represent more complex datamodel->view transformations, and is less restrictive on 
how the data model can be updated.
Reactive needs compiler support (through a macroDSL which will most likely prevent all IDE autocompletion), while caching can work with minimal macros.
(Quoting Raph Levien on zulip: "how can you express UI using fairly vanilla language constructs?")

