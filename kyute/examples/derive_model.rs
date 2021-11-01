use kyute::{
    view,
    widget::{Axis, Flex, Slider},
    BoxConstraints, DynLens, Environment, Event, EventCtx, LayoutCtx, LifecycleEvent, Measurements,
    Model, PaintCtx, Rect, UpdateCtx, Widget,
};

#[derive(Model)]
struct Item {
    name: String,
}

#[derive(Model)]
struct DataModel {
    i: i32,
    f: f32,
    #[model(skip)]
    s: String,
    items: Vec<Item>,
}

fn derive_in_function() {
    #[derive(Model)]
    struct DataModel {
        i: i32,
    }
}

struct CounterLabel_Properties<T> {
    current_value: DynLens<T, f64>,
    label: DynLens<T, String>,
}

struct CounterLabel_State {
    max_value: f64,
}

#[derive(Model)]
struct CounterLabel_Data<T> {
    outer_data: T,
    state: CounterLabel_State,
}

struct CounterLabel<T> {
    props: CounterLabel_Properties<T>,
    state: Option<CounterLabel_State>,
    root: Flex<CounterLabel_Data<T>>,
}

impl<T> CounterLabel<T> {
    pub fn new() -> CounterLabel<T> {
        CounterLabel {
            state: Some(CounterLabel_State::new()),
            root: Flex::new()
                .bind_axis(|_| Axis::Vertical)
                .bind_items(|data, change, items| {
                    if !items.is_empty() {
                        return None;
                    }
                    *items = vec![WidgetPod::new(
                        Slider::new()
                            .bind_min(|_| 0.0)
                            .bind_value(|_| data.state.max_value),
                    )];
                    None
                }),
        }
    }

    pub fn bind_current_value(mut self) -> Self {
        self
    }
}

impl<T> Widget<T> for CounterLabel<T> {
    fn debug_name(&self) -> &str {
        "CounterLabel"
    }

    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut T,
    ) -> Option<<T as Model>::Change> {
        todo!()
    }

    fn lifecycle(&mut self, ctx: &mut EventCtx, lifecycle_event: &LifecycleEvent, data: &mut T) {
        take_mut::take(data, |outer_data| {
            let mut inner_data = CounterLabel_Data {
                outer_data,
                state: self.state.take().unwrap(),
            };
            self.inner.lifecycle(ctx, lifecycle_event, &mut inner_data);
            self.state.replace(inner_data.state);
            inner_data.outer_data
        });
    }

    fn update(&mut self, ctx: &mut UpdateCtx, data: &T, change: &<T as Model>::Change) {
        todo!()
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        data: &T,
        env: &Environment,
    ) -> Measurements {
        self.contents.layout(ctx, constraints, data, env)
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        self.contents.paint(ctx, bounds, env)
    }
}

fn main() {
    view! {
        view CounterLabel(
            mut current_value: f64,
            label: String)
        {
            let mut max_value : f64 = 0.0;

            // Flex<T>
            Flex {
                // .set_orientation(Axis::Vertical)
                orientation: Axis::Vertical;

                // Slider<T>? even if all that slider needs is Slider<f64>
                Slider {
                    min: 0.0;
                    max: max_value;          // issue: how to find max_value here? (it's data.state.max_value)
                    value: current_value;    // same (it's `self.props.current_value.get(&data.outer_data)`), but can't borrow props!
                }
            }
        }
    }

    /*
    struct State {
        counter: f64,
        min: i32,
        max: i32,
    }


    struct Props<'a> {
        text: &'a mut String,
    }

    enum StateChange {
        counter(<f64 as Model>::Change),
        min(<i32 as Model>::Change),
        max(<i32 as Model>::Change),
        text(<String as Model>::Change)
    }

    impl Model for State {
        type Change = StateChange;
    }

    struct Data<'a> {
        state: &'a mut State,
    }

    enum DataChange {
        state(<State as Model>::Change),
        props(),
    }

    impl<'a> Model for Data<'a> {
        type Change = DataChange;
    }

    struct Binder {
        inner: Slider,
    }

    // impl never boxed
    impl<'a> Widget<Data<'a>> for Binder {
        fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Data<'a>) -> Option<DataChange> {
            self.inner.event(ctx, event, &mut data.state.counter).map(|x| DataChange::State(StateChange::counter(x)))
        }

        fn update(&mut self, ctx: &mut UpdateCtx, data: &Data<'a>, change: &DataChange) {
            match change {
                DataChange::State(change) => {
                    match change {
                       StateChange::counter(change) => {
                           self.inner.update(ctx, &data.state.counter, change)
                       },
                        _ => {},
                    }
                }
                _ => {}
            }
        }

        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints, data: &Data<'a>, env: &Environment) -> Measurements {
            self.inner.layout(ctx, constraints, &data.state.counter, env)
        }

        fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
            self.inner.paint(ctx, bounds, env);
        }
    }

    struct View {
        state: State,
        root: Binder,
    }

    impl Widget<()> for View {
        fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut ()) -> Option<()> {
            let mut data = Data {
                state: &mut self.state,
            };
            let change = self.root.event(ctx, event, &mut data);
            None
        }

        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints, data: &Data, env: &Environment) -> Measurements {
            todo!()
        }

        fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
            todo!()
        }
    }

    view! {
        let mut counter : i32 = 0i32;
        let mut min = 0;
        let mut max = 100;
        let mut text : String = Default::default();

        // Flex(Vec<WidgetPod>)
        Flex(.orientation = Axis::Horizontal) {

            // all items in this list are converted into `with_child` calls on the parent

            // this is converted into a "binder" widget that takes a part of the parent state as input
            // (in this case, counter, so `Binder<(i32,)>`.

            // wrapped in a lens that provides read-only access to text and counter (builds a struct)


            Text(format!("Counter value={}, text={}", counter, text))
            TextEdit(text)

            //
            Slider(
                .value = counter,
                .min = min
                .max = max)

            Button(
                // problem: the macro needs to be super smart to convert that into a reactive thing
                // maybe $counter is rewritten to a special object that produces mutations under the hood?
                // like a ChangeTracker<i32> or something?
                // anyway, $counter must expand into a mutable i32 lvalue
                .on_click = {
                    // runs in the context of the root view, somehow?
                    counter += 1
                }
            )
        }
    }

    struct View_State {
        counter: i32,
        text: String,
    }

    struct View {
        root: WidgetPod<Flex>,
        counter: i32,
        text: String,
    }

    struct View_AssociatedData<'a> {
        // no associated data
        state: &'a mut View_State,
    }

    struct Binder_48(Slider);
    impl<'a> Widget<View_AssociatedData<'a>> for Binder_48 {
        fn update(&mut self, ctx: &mut UpdateCtx, data: &T, change: T::Change) {
            match change {

            }
        }
    }

    impl View {
        pub fn new() -> View {

            let root_widget =
                Flex::new()
                    .set_orientation(Axis::Horizontal)
                    .add_child(
                        Binder::<i32>::new(Text::new())
                            .on_change(|ctx, counter, text| text.set_text(format!("Counter value: {}", counter)) )
                    )
                    .add_child(
                        Button::new().on_click(|ctx, state| {
                            let mut counter = &mut state.counter;
                            ctx.push_change(View_28_Change::Counter);
                        })
                    );


        }
    }

    impl Widget for View {

        fn update(&mut self, data: &T, change: T::Change) {
            self.root.update(data, change);
        }
    }*/
}

// I don't like having to cram all properties into the type parameter, it's not very readable.
