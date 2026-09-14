//! Page components for the Fabric web frontend.

mod topology;
mod route;
mod stream;
mod network;
mod settings;

pub use topology::TopologyPage;
pub use route::{RoutesPage, CapabilitiesPage, HealthPage};
pub use stream::StreamPage;
pub use network::NetworkPage;
pub use settings::SettingsPage;
