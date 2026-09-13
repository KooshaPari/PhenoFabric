//! Leptos application components for the Fabric web frontend.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::path;

mod pages;
use pages::{CapabilitiesPage, HealthPage, RoutesPage, StreamPage, TopologyPage};

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Html {..} lang="en" dir="ltr" data-theme="dark" />
        <Title text="Phenotype Fabric" />
        <Meta name="description" content="Phenotype Fabric — capability-aware compute topology" />

        <Router>
            <nav class="sidebar">
                <h1>"Fabric"</h1>
                <A href="/">"Topology"</A>
                <A href="/routes">"Routes"</A>
                <A href="/capabilities">"Capabilities"</A>
                <A href="/health">"Health"</A>
                <A href="/stream">"Stream"</A>
            </nav>
            <main class="content">
                <Routes fallback=|| view! { <p>"Not found"</p> }>
                    <Route path=path!("/") view=TopologyPage />
                    <Route path=path!("/routes") view=RoutesPage />
                    <Route path=path!("/capabilities") view=CapabilitiesPage />
                    <Route path=path!("/health") view=HealthPage />
                    <Route path=path!("/stream") view=StreamPage />
                </Routes>
            </main>
        </Router>
    }
}
