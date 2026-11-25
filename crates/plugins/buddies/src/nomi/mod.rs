//! NOMI 智能座舱虚拟伴侣与桌面交互系统。

pub mod genome;
mod headwear;
pub mod physics;
pub mod renderer;
mod settings;
mod view;

pub(crate) use settings::spec as settings_spec;
pub(crate) use view::spec;
