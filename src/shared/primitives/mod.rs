//! Value types shared by every layer: geometry, ids, colors, monitors, layout results and render
//! buffers.
mod binary;
pub mod color;
pub mod dynamic;
mod focus;
pub mod geometry;
mod ids;
pub mod layout;
mod monitor;
mod offset;
mod options;
pub mod render;
#[cfg(test)]
mod tests;
mod text_align;

pub(crate) use binary::BinaryData;
pub use color::{Color, DrawingColor};
pub(crate) use dynamic::DynamicValue;
pub(crate) use focus::MonitorFocus;
#[cfg(test)]
pub(crate) use geometry::Scale;
pub(crate) use geometry::{Position, Size};
pub use ids::{FunctionName, ModuleId, ModuleInstanceId, ModuleName, MonitorId};
pub use layout::SizeConstraint;
pub(crate) use layout::{
    ChildBounds, ChildModuleLayout, ChildReport, ChildSizesMap, LayoutSurface, ModuleKey,
    ModuleSite,
};
pub use monitor::ScriptMonitorInfo;
pub use offset::PopupOffset;
pub use options::ModuleOptions;
pub use text_align::{TextAlign, TextAlignment};
