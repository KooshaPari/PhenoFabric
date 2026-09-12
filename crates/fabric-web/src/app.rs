//! Leptos application components for the Fabric web frontend.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::{Route, Router, path};

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Html lang="en" attr:dir="ltr" attr:theme="dark" />
        <Title text="Phenotype Fabric" />
        <Meta name="description" content="Phenotype Fabric — capability-aware compute topology" />

        <Router>
            <main>
                <nav class="sidebar">
                    <h1>"Fabric"</h1>
                    <A href="/">"Topology"</A>
                    <A href="/routes">"Routes"</A>
                    <A href="/capabilities">"Capabilities"</A>
                    <A href="/health">"Health"</A>
                </nav>
                <section class="content">
                    <Routes>
                        <Route path=path!("/") view=TopologyPage />
                        <Route path=path!("/routes") view=RoutesPage />
                        <Route path=path!("/capabilities") view=CapabilitiesPage />
                        <Route path=path!("/health") view=HealthPage />
                    </Routes>
                </section>
            </main>
        </Router>
    }
}

/// Topology page — shows nodes and edges.
#[component]
fn TopologyPage() -> impl IntoView {
    let (nodes, set_nodes) = signal(Vec::<TopologyNode>::new());
    let (error, set_error) = signal(Option::<String>::None);

    // Fetch topology on mount
    leptos::task::spawn_local(async move {
        match fetch_json::<TopologyResponse>("/api/topology").await {
            Ok(data) => set_nodes.set(data.nodes),
            Err(e) => set_error.set(Some(format!("Failed to load topology: {e}"))),
        }
    });

    view! {
        <h2>"Topology Nodes"</h2>

        <Show
            when=move || error.get().is_some()
            fallback=|| view! { <p class="loading">"Loading..."</p> }
        >
            <p class="error">{move || error.get().unwrap_or_default()}</p>
        </Show>

        <Show
            when=move || !nodes.get().is_empty()
            fallback=|| view! {
                <p class="empty">
                    "No topology loaded. Connect to a Fabric daemon to view topology."
                </p>
            }
        >
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"ID"</th>
                        <th>"Label"</th>
                        <th>"Locality"</th>
                        <th>"Capabilities"</th>
                        <th>"Tags"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || nodes.get()
                        key=|node| node.id.clone()
                        children=move |node| view! {
                            <tr>
                                <td class="mono">{node.id[..node.id.len().min(12)].to_string()}</td>
                                <td>{node.label.unwrap_or_else(|| "—".to_string())}</td>
                                <td>{node.locality}</td>
                                <td>{format!("{}", node.cap_count)}</td>
                                <td>{node.tags}</td>
                            </tr>
                        }
                    />
                </tbody>
            </table>
        </Show>
    }
}

/// Routes page — shows compiled route plans.
#[component]
fn RoutesPage() -> impl IntoView {
    let (routes, set_routes) = signal(Vec::<RoutePlan>::new());

    leptos::task::spawn_local(async move {
        if let Ok(data) = fetch_json::<RoutesResponse>("/api/routes").await {
            set_routes.set(data.routes);
        }
    });

    view! {
        <h2>"Route Plans"</h2>

        <Show
            when=move || !routes.get().is_empty()
            fallback=|| view! {
                <p class="empty">
                    "No route plans found. Use 'fabric route compile' to create one."
                </p>
            }
        >
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Intent"</th>
                        <th>"Steps"</th>
                        <th>"Cost"</th>
                        <th>"Trust"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || routes.get()
                        key=|route| route.intent.clone()
                        children=move |route| view! {
                            <tr>
                                <td>{route.intent}</td>
                                <td>{format!("{}", route.steps)}</td>
                                <td>{format!("{:.1}", route.cost)}</td>
                                <td class={format!("trust-{}", route.trust_level.to_lowercase())}>
                                    {route.trust_level}
                                </td>
                            </tr>
                        }
                    />
                </tbody>
            </table>
        </Show>
    }
}

/// Capabilities page — shows capability descriptors from topology.
#[component]
fn CapabilitiesPage() -> impl IntoView {
    let (caps, set_caps) = signal(Vec::<CapEntry>::new());

    leptos::task::spawn_local(async move {
        if let Ok(data) = fetch_json::<CapsResponse>("/api/capabilities").await {
            set_caps.set(data.capabilities);
        }
    });

    view! {
        <h2>"Capabilities"</h2>

        <Show
            when=move || !caps.get().is_empty()
            fallback=|| view! {
                <p class="empty">
                    "No capabilities found. Use 'fabric cap probe' to scan this machine."
                </p>
            }
        >
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Node"</th>
                        <th>"Descriptor ID"</th>
                        <th>"Trust"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || caps.get()
                        key=|cap| cap.descriptor_id.clone()
                        children=move |cap| view! {
                            <tr>
                                <td>{cap.node_name}</td>
                                <td class="mono">{cap.descriptor_id[..cap.descriptor_id.len().min(20)].to_string()}</td>
                                <td class={format!("trust-{}", cap.trust.to_lowercase())}>
                                    {cap.trust}
                                </td>
                            </tr>
                        }
                    />
                </tbody>
            </table>
        </Show>
    }
}

/// Health page — daemon and workspace status.
#[component]
fn HealthPage() -> impl IntoView {
    let (health, set_health) = signal(Option::<HealthInfo>::None);

    leptos::task::spawn_local(async move {
        if let Ok(data) = fetch_json::<HealthInfo>("/api/health").await {
            set_health.set(Some(data));
        }
    });

    view! {
        <h2>"Health"</h2>

        <Show
            when=move || health.get().is_some()
            fallback=|| view! { <p class="loading">"Checking daemon health..."</p> }
        >
            {move || {
                health.get().map(|h| view! {
                    <table class="info-grid">
                        <tr>
                            <td>"Daemon"</td>
                            <td class={if h.daemon_healthy { "status-ok" } else { "status-error" }}>
                                {if h.daemon_healthy { "● Healthy" } else { "● Offline" }}
                            </td>
                        </tr>
                        <tr>
                            <td>"Nodes"</td>
                            <td>{format!("{}", h.node_count)}</td>
                        </tr>
                        <tr>
                            <td>"Edges"</td>
                            <td>{format!("{}", h.edge_count)}</td>
                        </tr>
                        <tr>
                            <td>"Capabilities"</td>
                            <td>{format!("{}", h.cap_count)}</td>
                        </tr>
                        <tr>
                            <td>"Route Plans"</td>
                            <td>{format!("{}", h.route_count)}</td>
                        </tr>
                    </table>
                })
            }}
        </Show>
    }
}

// --- Data types for API responses ---

#[derive(serde::Deserialize, Clone, Debug)]
struct TopologyResponse {
    nodes: Vec<TopologyNode>,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct TopologyNode {
    id: String,
    label: Option<String>,
    locality: String,
    cap_count: usize,
    tags: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct RoutesResponse {
    routes: Vec<RoutePlan>,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct RoutePlan {
    intent: String,
    steps: usize,
    cost: f64,
    trust_level: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct CapsResponse {
    capabilities: Vec<CapEntry>,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct CapEntry {
    node_name: String,
    descriptor_id: String,
    trust: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct HealthInfo {
    daemon_healthy: bool,
    node_count: usize,
    edge_count: usize,
    cap_count: usize,
    route_count: usize,
}

// --- API helpers ---

async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    use web_sys::Request;

    let window = web_sys::window().ok_or("No window")?;
    let resp_val = window
        .fetch_with_str(url)
        .await
        .map_err(|e| format!("Fetch failed: {e:?}"))?;

    let resp: web_sys::Response = resp_val.into();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e:?}"))?;

    serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {e}"))
}
