pub mod application;

//pub mod node;
//mod visual;
//mod window;

mod bloom;
mod composition;
mod core;
mod data;
mod event;
mod key;
mod layout;
mod style;
mod util;
pub mod widget;
mod window;
pub mod region;
#[macro_use]
mod env;
pub mod theme;
mod default_style;
//mod style;

pub use crate::core::{
    Dummy, EventCtx, FocusAction, LayoutCtx, NodeId, PaintCtx, RepaintRequest, Widget,
};
pub use composition::CompositionCtx;
pub use key::Key;
pub use layout::{align_boxes, Alignment, BoxConstraints, Measurements};
pub use window::WindowWidget;
pub use env::{EnvKey, EnvValue, Environment};
pub use default_style::get_default_application_style;

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
