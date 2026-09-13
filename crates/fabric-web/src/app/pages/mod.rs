//! Page components for the Fabric web frontend.

mod topology;
mod route;
mod stream;

pub use topology::TopologyPage;
pub use route::{RoutesPage, CapabilitiesPage, HealthPage};
pub use stream::StreamPage;
