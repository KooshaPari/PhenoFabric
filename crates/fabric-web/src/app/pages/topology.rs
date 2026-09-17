//! Topology page — shows nodes from the daemon.

use crate::api::*;
use leptos::prelude::*;

/// Topology page — shows nodes from the daemon.
#[component]
pub fn TopologyPage() -> impl IntoView {
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
