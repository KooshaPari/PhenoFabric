//! Page components for the Fabric web frontend.

mod network;
mod route;
mod settings;
mod stream;
mod topology;

pub use network::NetworkPage;
pub use route::{CapabilitiesPage, HealthPage, RoutesPage};
pub use settings::SettingsPage;
pub use stream::StreamPage;
pub use topology::TopologyPage;
