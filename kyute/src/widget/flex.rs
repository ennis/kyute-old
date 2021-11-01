use crate::{
    binding::{DynLens, LensExt},
    core2::{LayoutCtx, PaintCtx, UpdateCtx, WidgetPod},
    event::LifecycleEvent,
    model::CollectionChange,
    BoxConstraints, Environment, Event, EventCtx, Measurements, Model, Offset, Rect, Size, Widget,
};
use tracing::trace;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn cross_axis(self) -> Axis {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    pub fn main_len(self, size: Size) -> f64 {
        match self {
            Axis::Vertical => size.height,
            Axis::Horizontal => size.width,
        }
    }

    pub fn cross_len(self, size: Size) -> f64 {
        match self {
            Axis::Vertical => size.width,
            Axis::Horizontal => size.height,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MainAxisAlignment {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CrossAxisAlignment {
    Baseline,
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MainAxisSize {
    Min,
    Max,
}

pub struct Flex<T: Model> {
    axis: DynLens<T, Axis>,
    items: Vec<WidgetPod<T>>,
    update_items: Box<dyn Fn(&T, &T::Change, &mut Vec<WidgetPod<T>>) -> Option<CollectionChange>>,
}

impl<T: Model> Flex<T> {
    pub fn new() -> Flex<T> {
        Flex {
            axis: Box::new(|| Axis::Horizontal),
            items: vec![],
            update_items: Box::new(|_, _, _| None),
        }
    }

    pub fn bind_axis(mut self, axis: impl Into<DynLens<T, Axis>>) -> Self {
        self.axis = axis.into();
        self
    }

    pub fn bind_items(
        mut self,
        update_items: impl Fn(&T, &T::Change, &mut Vec<WidgetPod<T>>) -> Option<CollectionChange> + 'static,
    ) -> Self {
        self.update_items = Box::new(update_items);
        self
    }
}

impl<T: Model> Widget<T> for Flex<T> {
    fn lifecycle(&mut self, ctx: &mut EventCtx, event: &LifecycleEvent, data: &mut T) {
        for item in self.items.iter_mut() {
            item.lifecycle(ctx, event, data);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, data: &mut T, change: &T::Change) {
        (self.update_items)(data, change, &mut self.items);
        for item in self.items.iter_mut() {
            item.update(ctx, data, change);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        data: &mut T,
        env: &Environment,
    ) -> Measurements {
        let axis = self.axis.get_owned(data);

        let item_measures: Vec<Measurements> = self
            .items
            .iter_mut()
            .map(|item| item.layout(ctx, constraints, data, env))
            .collect();

        let max_cross_axis_len = item_measures
            .iter()
            .map(|l| axis.cross_len(l.size()))
            .fold(0.0, f64::max);

        // preferred size of this flex: max size in axis direction, max elem width in cross-axis direction
        let cross_axis_len = match axis {
            Axis::Vertical => constraints.constrain_width(max_cross_axis_len),
            Axis::Horizontal => constraints.constrain_height(max_cross_axis_len),
        };

        // distribute children
        let mut d = 0.0;
        //let spacing = env.get(theme::FlexSpacing);
        let spacing = 1.0;

        let size = match axis {
            Axis::Vertical => Size::new(cross_axis_len, constraints.constrain_height(d)),
            Axis::Horizontal => Size::new(constraints.constrain_width(d), cross_axis_len),
        };

        for i in 0..self.items.len() {
            let len = axis.main_len(item_measures[i].size());
            let offset = match axis {
                Axis::Vertical => Offset::new(0.0, d),
                Axis::Horizontal => Offset::new(d, 0.0),
            };
            self.items[i].set_child_offset(offset);
            d += len + spacing;
            d = d.ceil();
        }

        Measurements::new(size)
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, _data: &mut T, env: &Environment) {
        todo!()
    }
}
