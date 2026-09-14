//! Network page — shows UPnP, Tailscale, and STUN status.

use leptos::prelude::*;
use crate::api::*;

/// Network page — shows UPnP port mappings, Tailscale peers, and STUN external address.
#[component]
pub fn NetworkPage() -> impl IntoView {
    let (network, set_network) = signal(Option::<NetworkStatus>::None);
    let (error, set_error) = signal(Option::<String>::None);
    let (last_refresh, set_last_refresh) = signal(String::new());

    // Initial fetch on mount.
    leptos::task::spawn_local(async move {
        let url = format!("{}/network", daemon_base_url());
        match fetch_json::<NetworkStatus>(&url).await {
            Ok(data) => {
                set_network.set(Some(data));
                set_last_refresh.set("now".to_string());
            }
            Err(e) => set_error.set(Some(e)),
        }
    });

    // Auto-refresh every 5 seconds via setInterval.
    leptos::task::spawn_local(async move {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let closure = js_sys::Function::new_no_args(
            "() => { const btn = document.querySelector('[data-refresh-network]'); if (btn) btn.click(); }",
        );
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(&closure, 5000);
    });

    // Re-fetch handler (triggered by the hidden refresh button).
    let do_refresh = move |_| {
        leptos::task::spawn_local(async move {
            let url = format!("{}/network", daemon_base_url());
            match fetch_json::<NetworkStatus>(&url).await {
                Ok(data) => {
                    set_network.set(Some(data));
                    set_last_refresh.set("just now".to_string());
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    view! {
        <h2 class="page-title">"Network Status"</h2>
        <p class="page-subtitle">"UPnP, STUN, and Tailscale mesh connectivity"</p>

        // Hidden refresh button for setInterval callback.
        <button
            class="refresh-trigger"
            data-refresh-network="true"
            on:click=do_refresh
            style="display:none"
        >"refresh"</button>

        <Show
            when=move || error.get().is_some()
            fallback=|| view! { <p class="loading">"Loading network status..."</p> }
        >
            <p class="error">{move || error.get().unwrap_or_default()}</p>
        </Show>

        <Show
            when=move || network.get().is_some()
            fallback=|| view! { <p class="empty">"No network data. Connect to a Fabric daemon."</p> }
        >
            <div class="last-refresh">"Last refresh: " {move || last_refresh.get()}</div>
            <div class="card-grid">
                // UPnP card
                <div class="glass-card">
                    <div class="card-header">
                        <span class="card-title">"UPnP"</span>
                        {move || {
                            network.get().map(|n| {
                                let label = if n.upnp.is_some() { "Active" } else { "Unavailable" };
                                let cls = if n.upnp.is_some() { "status-dot status-ok" } else { "status-dot status-off" };
                                view! { <span class=cls>{label}</span> }
                            })
                        }}
                    </div>
                    {move || {
                        network.get().and_then(|n| n.upnp).map(|upnp| {
                            view! {
                                <div class="card-body">
                                    <div class="info-row"><span class="info-label">"External IP"</span><span class="info-value mono">{upnp.external_ip}</span></div>
                                    <div class="info-row"><span class="info-label">"Mapped Port"</span><span class="info-value">{format!("{}", upnp.mapped_port)}</span></div>
                                    <div class="info-row"><span class="info-label">"Internal Port"</span><span class="info-value">{format!("{}", upnp.internal_port)}</span></div>
                                    <div class="info-row"><span class="info-label">"Protocol"</span><span class="info-value">{upnp.protocol}</span></div>
                                </div>
                            }
                        })
                    }}
                </div>

                // STUN card
                <div class="glass-card">
                    <div class="card-header">
                        <span class="card-title">"STUN"</span>
                        {move || {
                            network.get().map(|n| {
                                let label = if n.stun.is_some() { "Resolved" } else { "Unavailable" };
                                let cls = if n.stun.is_some() { "status-dot status-ok" } else { "status-dot status-off" };
                                view! { <span class=cls>{label}</span> }
                            })
                        }}
                    </div>
                    {move || {
                        network.get().and_then(|n| n.stun).map(|stun| {
                            view! {
                                <div class="card-body">
                                    <div class="info-row"><span class="info-label">"External IP"</span><span class="info-value mono">{stun.external_ip}</span></div>
                                    <div class="info-row"><span class="info-label">"External Port"</span><span class="info-value">{format!("{}", stun.external_port)}</span></div>
                                    <div class="info-row"><span class="info-label">"NAT Type"</span><span class="info-value">{stun.nat_type}</span></div>
                                </div>
                            }
                        })
                    }}
                </div>

                // Tailscale card
                <div class="glass-card">
                    <div class="card-header">
                        <span class="card-title">"Tailscale"</span>
                        {move || {
                            network.get().map(|n| {
                                let label = if n.tailscale.is_some() { "Connected" } else { "Unavailable" };
                                let cls = if n.tailscale.is_some() { "status-dot status-ok" } else { "status-dot status-off" };
                                view! { <span class=cls>{label}</span> }
                            })
                        }}
                    </div>
                    {move || {
                        network.get().and_then(|n| n.tailscale).map(|ts| {
                            let self_ip = ts.self_ip.clone();
                            view! {
                                <div class="card-body">
                                    <div class="info-row"><span class="info-label">"Self IP"</span><span class="info-value mono">{self_ip}</span></div>
                                    <div class="peers-section">
                                        <span class="info-label">"Peers"</span>
                                        <table class="data-table compact">
                                            <thead>
                                                <tr>
                                                    <th>"Host"</th>
                                                    <th>"IP"</th>
                                                    <th>"Status"</th>
                                                    <th>"Relay"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                <For
                                                    each=move || ts.peers.clone()
                                                    key=|p| p.hostname.clone()
                                                    children=move |peer| {
                                                        let status_label = if peer.online { "Online" } else { "Offline" };
                                                        let status_cls = if peer.online { "status-dot status-ok" } else { "status-dot status-off" };
                                                        let relay_label = if peer.relay { "Yes" } else { "No" };
                                                        view! {
                                                            <tr>
                                                                <td>{peer.hostname}</td>
                                                                <td class="mono">{peer.tailscale_ip}</td>
                                                                <td><span class=status_cls>{status_label}</span></td>
                                                                <td>{relay_label}</td>
                                                            </tr>
                                                        }
                                                    }
                                                />
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            }
                        })
                    }}
                </div>
            </div>
        </Show>
    }
}