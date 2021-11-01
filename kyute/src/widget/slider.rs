//! Sliders provide a way to make a value vary linearly between two bounds by dragging a knob along
//! a line.
use crate::{
    binding::{DynLens, LensExt, ValueLens},
    event::{Event, LifecycleEvent, PointerEventKind},
    style::State,
    theme, BoxConstraints, Environment, EventCtx, LayoutCtx, Lens, Measurements, Model, PaintCtx,
    Point, Rect, SideOffsets, Size, UpdateCtx, Widget,
};

// TODO just pass f64 directly as the action?
#[derive(Copy, Clone, Debug)]
enum SliderAction {
    ValueChanged(f64),
}

/// Utility class representing a slider track on which a knob can move.
struct SliderTrack {
    start: Point,
    end: Point,
}

impl SliderTrack {
    fn new(start: Point, end: Point) -> SliderTrack {
        SliderTrack { start, end }
    }

    /// Returns the value that would be set if the cursor was at the given position.
    fn value_from_position(&self, pos: Point, min: f64, max: f64) -> f64 {
        /*let hkw = 0.5 * get_knob_width(track_width, divisions, min_knob_width);
        // at the end of the sliders, there are two "dead zones" of width kw / 2 that
        // put the slider all the way left or right
        let x = pos.x.max(hkw).min(track_width-hkw-1.0);*/

        // project the point on the track line
        let v = self.end - self.start;
        let c = pos - self.start;
        let x = v.normalize().dot(c);
        let track_len = v.length();
        (min + (max - min) * x / track_len).clamp(min, max)
    }

    /// Returns the position of the knob on the track.
    fn knob_position(&self, value: f64) -> Point {
        self.start + (self.end - self.start) * value
    }
}

impl Default for SliderTrack {
    fn default() -> Self {
        SliderTrack {
            start: Default::default(),
            end: Default::default(),
        }
    }
}

/*fn draw_slider_knob(
    ctx: &mut PaintCtx,
    size: Size,
    pos: f64,
    divisions: Option<u32>,
    theme: &Theme,
) {
    // half the height
    let min_knob_w = (0.5 * theme.button_metrics.min_height).ceil();
    let knob_w = get_knob_width(size.width, divisions, min_knob_w);

    let off = ((w - knob_w) * pos).ceil();
    let knob = Rect::new(Point::new(off, 0.0), Size::new(knob_w, h));

    // draw the knob rectangle
    let knob_brush = DEFAULT_COLORS.slider_grab.into_brush();
    ctx.fill_rectangle(knob, &knob_brush);
}*/

pub struct Slider<T> {
    track: SliderTrack,
    value: DynLens<T, f64>,
    min: DynLens<T, f64>,
    max: DynLens<T, f64>,
}

fn normalize_value(value: f64, min: f64, max: f64) -> f64 {
    (value - min) / (max - min)
}

impl<T: Model> Slider<T> {
    pub fn new() -> Slider<T> {
        Slider {
            // endpoints calculated during layout
            track: Default::default(),
            value: Box::new(|| 0.0),
            min: Box::new(|| 0.0),
            max: Box::new(|| 1.0),
        }
    }

    pub fn bind_min(mut self, min: impl Into<DynLens<T, f64>>) -> Self {
        self.min = min.into();
        self
    }

    pub fn bind_max(mut self, max: impl Into<DynLens<T, f64>>) -> Self {
        self.max = max.into();
        self
    }

    pub fn bind_value(mut self, value: impl Into<DynLens<T, f64>>) -> Self {
        self.value = value.into();
        self
    }
}

impl<T: Model> Widget<T> for Slider<T> {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T) -> Option<T::Change> {
        let min = self.min.get_owned(data);
        let max = self.max.get_owned(data);

        match event {
            Event::Pointer(p) => match p.kind {
                PointerEventKind::PointerOver | PointerEventKind::PointerOut => {
                    ctx.request_redraw();
                    None
                }
                PointerEventKind::PointerDown => {
                    let new_value = self.track.value_from_position(p.position, min, max);
                    self.value.set(data, new_value);
                    ctx.capture_pointer();
                    ctx.request_focus();
                    todo!()
                }
                PointerEventKind::PointerMove => {
                    if ctx.is_capturing_pointer() {
                        let new_value = self.track.value_from_position(p.position, min, max);
                        self.value.set(data, new_value);
                        todo!()
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn lifecycle(&mut self, _ctx: &mut EventCtx, _event: &LifecycleEvent, _data: &mut T) {
        // nothing
    }

    fn update(&mut self, ctx: &mut UpdateCtx, data: &mut T, change: &T::Change) {
        todo!()
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        _data: &mut T,
        _env: &Environment,
    ) -> Measurements {
        let height = 14.0; //env.get(theme::SliderHeight);
        let knob_width = 11.0; //env.get(theme::SliderKnobWidth);
        let knob_height = 11.0; //env.get(theme::SliderKnobHeight);
        let padding = SideOffsets::new_all_same(0.0);

        // fixed height
        let size = Size::new(
            constraints.max_width(),
            constraints.constrain_height(height),
        );

        // position the slider track inside the layout
        let inner_bounds = Rect::new(Point::origin(), size).inner_rect(padding);

        // calculate knob width
        //let knob_width = get_knob_width(inner_bounds.size.width, self.divisions, min_knob_width);
        // half knob width
        let hkw = 0.5 * knob_width;
        // y-position of the slider track
        let y = 0.5 * size.height;

        // center vertically, add some padding on the sides to account for padding and half-knob size
        self.track.start = Point::new(inner_bounds.min_x() + hkw, y);
        self.track.end = Point::new(inner_bounds.max_x() - hkw, y);

        Measurements {
            size,
            baseline: None,
        }
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, data: &mut T, env: &Environment) {
        let value = self.value.get_owned(data);
        let min = self.min.get_owned(data);
        let max = self.max.get_owned(data);

        let track_y = env.get(theme::SLIDER_TRACK_Y).unwrap_or_default();
        let track_h = env.get(theme::SLIDER_TRACK_HEIGHT).unwrap_or_default();
        let knob_w = env.get(theme::SLIDER_KNOB_WIDTH).unwrap_or_default();
        let knob_h = env.get(theme::SLIDER_KNOB_HEIGHT).unwrap_or_default();
        let knob_y = env.get(theme::SLIDER_KNOB_Y).unwrap_or_default();
        let track_style = env.get(theme::SLIDER_TRACK_STYLE).unwrap();
        let knob_style = env.get(theme::SLIDER_KNOB_STYLE).unwrap();

        let track_x_start = self.track.start.x;
        let track_x_end = self.track.end.x;

        // track bounds
        let track_bounds = Rect::new(
            Point::new(track_x_start, track_y - 0.5 * track_h),
            Size::new(track_x_end - track_x_start, track_h),
        );

        let kpos = self.track.knob_position(normalize_value(value, min, max));
        let kx = kpos.x.round() + 0.5;

        let knob_bounds = Rect::new(
            Point::new(kx - 0.5 * knob_w, track_y - knob_y),
            Size::new(knob_w, knob_h),
        );

        // track
        track_style.draw_box(ctx, &track_bounds, State::empty());
        knob_style.draw_box(ctx, &knob_bounds, State::empty());
    }
}
