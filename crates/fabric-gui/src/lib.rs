//! fabric-gui: egui-based native GUI for Phenotype Fabric.
//!
//! Provides topology visualization, route management, lease monitoring,
//! and daemon health in a native desktop window.

pub mod app;
pub mod panels;

pub use app::{GuiApp, Tab};
