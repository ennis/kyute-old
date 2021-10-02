extern crate self as kyute;

#[macro_use]
mod data;

//pub mod application;
mod bloom;
//mod composition;
//mod core;
mod event;
//mod key;
mod layout;
//mod style;
mod util;
//pub mod widget;
//mod window;
pub mod region;
#[macro_use]
mod env;
//pub mod theme;
//mod default_style;
mod cache;
mod call_key;
mod context;
mod core2;
pub mod widget;
pub mod application;
mod cache_cell;
mod window;
//mod style;

pub use kyute_macros::composable;

pub use event::Event;
pub use context::Context;
pub use data::Data;
pub use env::{EnvKey, EnvValue, Environment};
pub use layout::{align_boxes, Alignment, BoxConstraints, Measurements, LayoutItem};
pub use core2::{Widget, WidgetDelegate, LayoutCtx, PaintCtx, EventCtx};
pub use window::Window;

pub type Dip = kyute_shell::drawing::Dip;
pub type Px = kyute_shell::drawing::Px;

pub type DipToPx = euclid::Scale<f64, Dip, Px>;
pub type PxToDip = euclid::Scale<f64, Px, Dip>;
pub type SideOffsets = euclid::SideOffsets2D<f64, Dip>;
pub type Size = kyute_shell::drawing::Size;
pub type PhysicalSize = kyute_shell::drawing::PhysicalSize;
pub type Rect = kyute_shell::drawing::Rect;
pub type Offset = kyute_shell::drawing::Offset;
pub type Point = kyute_shell::drawing::Point;
pub type PhysicalPoint = kyute_shell::drawing::PhysicalPoint;
