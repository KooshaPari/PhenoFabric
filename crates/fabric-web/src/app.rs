//! Leptos application components for the Fabric web frontend.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::api::*;

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
            </nav>
            <main class="content">
                <Routes fallback=|| view! { <p>"Not found"</p> }>
                    <Route path=path!("/") view=TopologyPage />
                    <Route path=path!("/routes") view=RoutesPage />
                    <Route path=path!("/capabilities") view=CapabilitiesPage />
                    <Route path=path!("/health") view=HealthPage />
                </Routes>
            </main>
        </Router>
    }
}

/// Topology page — shows nodes from the daemon.
#[component]
fn TopologyPage() -> impl IntoView {
    let (nodes, set_nodes) = signal(Vec::<TopologyNode>::new());
    let (error, set_error) = signal(Option::<String>::None);

    leptos::task::spawn_local(async move {
        let url = format!("{}/topology", daemon_base_url());
        match fetch_json::<TopologyResponse>(&url).await {
            Ok(data) => set_nodes.set(data.nodes),
            Err(e) => set_error.set(Some(e)),
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
                <p class="empty">"No topology loaded. Connect to a Fabric daemon."</p>
            }
        >
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"ID"</th>
                        <th>"Label"</th>
                        <th>"Locality"</th>
                        <th>"Caps"</th>
                        <th>"Tags"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || nodes.get()
                        key=|node| node.id.clone()
                        children=move |node| {
                            let tags = node.tags.join(", ");
                            let cap_count = format!("{}", node.cap_count);
                            view! {
                                <tr>
                                    <td class="mono">{node.id}</td>
                                    <td>{node.label.unwrap_or_else(|| "—".to_string())}</td>
                                    <td>{node.locality}</td>
                                    <td>{cap_count}</td>
                                    <td>{tags}</td>
                                </tr>
                            }
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
        let url = format!("{}/routes", daemon_base_url());
        if let Ok(data) = fetch_json::<RoutesResponse>(&url).await {
            set_routes.set(data.routes);
        }
    });

    view! {
        <h2>"Route Plans"</h2>

        <Show
            when=move || !routes.get().is_empty()
            fallback=|| view! {
                <p class="empty">"No route plans. Use 'fabric route compile'."</p>
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
                        children=move |route| {
                            let steps = format!("{}", route.steps);
                            let cost = format!("{:.1}", route.cost);
                            view! {
                                <tr>
                                    <td>{route.intent}</td>
                                    <td>{steps}</td>
                                    <td>{cost}</td>
                                    <td>{route.trust_level}</td>
                                </tr>
                            }
                        }
                    />
                </tbody>
            </table>
        </Show>
    }
}

/// Capabilities page — shows capability descriptors.
#[component]
fn CapabilitiesPage() -> impl IntoView {
    let (caps, set_caps) = signal(Vec::<CapEntry>::new());

    leptos::task::spawn_local(async move {
        let url = format!("{}/capabilities", daemon_base_url());
        if let Ok(data) = fetch_json::<CapabilitiesResponse>(&url).await {
            set_caps.set(data.capabilities);
        }
    });

    view! {
        <h2>"Capabilities"</h2>

        <Show
            when=move || !caps.get().is_empty()
            fallback=|| view! {
                <p class="empty">"No capabilities. Use 'fabric cap probe'."</p>
            }
        >
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Node"</th>
                        <th>"Descriptor"</th>
                        <th>"Trust"</th>
                    </tr>
                </thead>
                <tbody>
                    <For
                        each=move || caps.get()
                        key=|cap| cap.descriptor_id.clone()
                        children=move |cap| {
                            let short_id = if cap.descriptor_id.len() > 20 {
                                cap.descriptor_id[..20].to_string()
                            } else {
                                cap.descriptor_id.clone()
                            };
                            view! {
                                <tr>
                                    <td>{cap.node_name}</td>
                                    <td class="mono">{short_id}</td>
                                    <td>{cap.trust}</td>
                                </tr>
                            }
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
    let (health, set_health) = signal(Option::<HealthResponse>::None);

    leptos::task::spawn_local(async move {
        let url = format!("{}/health", daemon_base_url());
        if let Ok(data) = fetch_json::<HealthResponse>(&url).await {
            set_health.set(Some(data));
        }
    });

    view! {
        <h2>"Health"</h2>

        <Show
            when=move || health.get().is_some()
            fallback=|| view! { <p class="loading">"Checking daemon..."</p> }
        >
            {move || {
                health.get().map(|h| {
                    let status_text = if h.daemon_healthy { "● Healthy" } else { "● Offline" };
                    let status_class = if h.daemon_healthy { "status-ok" } else { "status-error" };
                    view! {
                        <table class="info-grid">
                            <tr>
                                <td>"Daemon"</td>
                                <td class=status_class>{status_text}</td>
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
                    }
                })
            }}
        </Show>
    }
}
