//! Settings page — daemon configuration and feature toggles.

use leptos::prelude::*;
use crate::api::*;

/// Settings page — daemon configuration and feature toggles.
#[component]
pub fn SettingsPage() -> impl IntoView {
    let (settings, set_settings) = signal(Option::<SettingsResponse>::None);
    let (features, set_features) = signal(FeatureToggles::default());
    let (save_status, set_save_status) = signal(String::new());
    let (save_error, set_save_error) = signal(Option::<String>::None);

    // Fetch settings on mount.
    leptos::task::spawn_local(async move {
        let url = format!("{}/config", daemon_base_url());
        match fetch_json::<SettingsResponse>(&url).await {
            Ok(data) => {
                set_features.set(data.features.clone());
                set_settings.set(Some(data));
            }
            Err(e) => set_save_error.set(Some(e)),
        }
    });

    let save_config = move |_| {
        set_save_status.set("Saving...".to_string());
        set_save_error.set(None);

        let current_features = features.get();
        leptos::task::spawn_local(async move {
            let url = format!("{}/config", daemon_base_url());
            let body = SettingsResponse {
                listen: String::new(),
                max_connections: 0,
                request_timeout_ms: 0,
                features: current_features,
            };
            match post_json::<SettingsResponse, _>(&url, &body).await {
                Ok(data) => {
                    set_features.set(data.features.clone());
                    set_settings.set(Some(data));
                    set_save_status.set("Saved".to_string());
                }
                Err(e) => set_save_error.set(Some(e)),
            }
        });
    };

    view! {
        <h2 class="page-title">"Settings"</h2>
        <p class="page-subtitle">"Daemon configuration and feature toggles"</p>

        <Show
            when=move || save_error.get().is_some()
            fallback=|| view! {}
        >
            <p class="error">{move || save_error.get().unwrap_or_default()}</p>
        </Show>

        <div class="card-grid">
            // Daemon config card
            <div class="glass-card">
                <div class="card-header">
                    <span class="card-title">"Daemon Configuration"</span>
                </div>
                <div class="card-body">
                    {move || {
                        settings.get().map(|s| {
                            view! {
                                <div class="config-grid">
                                    <div class="info-row"><span class="info-label">"Listen Address"</span><span class="info-value mono">{s.listen}</span></div>
                                    <div class="info-row"><span class="info-label">"Max Connections"</span><span class="info-value">{format!("{}", s.max_connections)}</span></div>
                                    <div class="info-row"><span class="info-label">"Request Timeout"</span><span class="info-value">{format!("{}ms", s.request_timeout_ms)}</span></div>
                                </div>
                            }
                        })
                    }}
                </div>
            </div>

            // Feature toggles card
            <div class="glass-card">
                <div class="card-header">
                    <span class="card-title">"Feature Toggles"</span>
                </div>
                <div class="card-body">
                    <div class="toggle-grid">
                        <FeatureToggle label="UPnP" feature_key="upnp" features=features set_features=set_features />
                        <FeatureToggle label="Logging" feature_key="logging" features=features set_features=set_features />
                        <FeatureToggle label="Federation" feature_key="federation" features=features set_features=set_features />
                    </div>
                </div>
            </div>
        </div>

        <div class="save-section">
            <button class="btn-save" on:click=save_config>"Save Configuration"</button>
            <Show
                when=move || !save_status.get().is_empty()
                fallback=|| view! {}
            >
                <span class="save-status">{move || save_status.get()}</span>
            </Show>
        </div>
    }
}

/// A single toggle switch component.
#[component]
fn FeatureToggle(
    label: &'static str,
    feature_key: &'static str,
    features: ReadSignal<FeatureToggles>,
    set_features: WriteSignal<FeatureToggles>,
) -> impl IntoView {
    let is_enabled = Memo::new(move |_| match feature_key {
        "upnp" => features.get().upnp_enabled,
        "logging" => features.get().logging_enabled,
        "federation" => features.get().federation_enabled,
        _ => false,
    });

    let cls = Memo::new(move |_| {
        if is_enabled.get() {
            "toggle-switch on".to_string()
        } else {
            "toggle-switch off".to_string()
        }
    });

    let handle_click = move |_| {
        set_features.update(|f| match feature_key {
            "upnp" => f.upnp_enabled = !f.upnp_enabled,
            "logging" => f.logging_enabled = !f.logging_enabled,
            "federation" => f.federation_enabled = !f.federation_enabled,
            _ => {}
        });
    };

    view! {
        <div class="toggle-row">
            <span class="toggle-label">{label}</span>
            <button class=cls on:click=handle_click>
                <span class="toggle-thumb" />
            </button>
        </div>
    }
}