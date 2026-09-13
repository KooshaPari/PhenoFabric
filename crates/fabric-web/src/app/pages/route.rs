//! Routes, capabilities, and health pages — simple data-fetching views.

use leptos::prelude::*;
use crate::api::*;

/// Routes page — shows compiled route plans.
#[component]
pub fn RoutesPage() -> impl IntoView {
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
pub fn CapabilitiesPage() -> impl IntoView {
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
pub fn HealthPage() -> impl IntoView {
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
