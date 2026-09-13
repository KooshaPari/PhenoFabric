//! fabric-gui: egui-based native GUI for Phenotype Fabric.
//!
//! Provides topology visualization, route management, lease monitoring,
//! and daemon health in a native desktop window with liquid glass aesthetics.

pub mod animation;
pub mod app;
pub mod daemon_manager;
pub mod panels;
pub mod premium;
pub mod theme;
pub mod widgets;

pub use app::{GuiApp, Tab};
pub use daemon_manager::{DaemonManager, DaemonState};
