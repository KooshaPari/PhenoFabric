//! Leptos application components for the Fabric web frontend.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::path;

mod pages;
use pages::{CapabilitiesPage, HealthPage, NetworkPage, RoutesPage, SettingsPage, StreamPage, TopologyPage};

/// Premium CSS for the Fabric web UI — liquid glass aesthetic.
const PREMIUM_CSS: &str = r#"
/* ===== Fabric Premium — Liquid Glass ===== */
:root {
    --bg-primary: #0C0E14;
    --bg-secondary: #12141C;
    --card-bg: rgba(108, 99, 255, 0.08);
    --card-border: rgba(108, 99, 255, 0.15);
    --card-border-hover: rgba(108, 99, 255, 0.30);
    --primary: #6C63FF;
    --primary-dim: rgba(108, 99, 255, 0.50);
    --accent: #00D4FF;
    --text-primary: #E8E8F0;
    --text-secondary: #8B8BA0;
    --text-muted: #5A5A72;
    --green: #4ADE80;
    --yellow: #FBBF24;
    --red: #F87171;
    --radius: 14px;
    --radius-sm: 8px;
    --glass-blur: blur(12px);
    --transition: 0.3s ease;
}

*, *::before, *::after {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

html, body {
    font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
    background: var(--bg-primary);
    color: var(--text-primary);
    line-height: 1.6;
    min-height: 100vh;
    -webkit-font-smoothing: antialiased;
}

/* === Layout === */
#app {
    display: flex;
    min-height: 100vh;
}

nav.sidebar {
    width: 220px;
    min-width: 220px;
    background: rgba(12, 14, 20, 0.85);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-right: 1px solid var(--card-border);
    padding: 28px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    position: sticky;
    top: 0;
    height: 100vh;
    overflow-y: auto;
}

nav.sidebar h1 {
    font-size: 1.25rem;
    font-weight: 700;
    background: linear-gradient(135deg, var(--primary), var(--accent));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    margin-bottom: 24px;
    padding: 0 8px;
    letter-spacing: -0.02em;
}

nav.sidebar a {
    display: block;
    padding: 10px 12px;
    color: var(--text-secondary);
    text-decoration: none;
    border-radius: var(--radius-sm);
    font-size: 0.875rem;
    font-weight: 500;
    transition: all var(--transition);
}

nav.sidebar a:hover {
    color: var(--text-primary);
    background: rgba(108, 99, 255, 0.10);
}

nav.sidebar a.active,
nav.sidebar a[aria-current="page"] {
    color: var(--accent);
    background: rgba(0, 212, 255, 0.08);
    font-weight: 600;
}

main.content {
    flex: 1;
    padding: 32px 40px;
    max-width: 1100px;
}

/* === Page Headers === */
.page-title {
    font-size: 1.5rem;
    font-weight: 700;
    background: linear-gradient(135deg, var(--text-primary), var(--primary-dim));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    margin-bottom: 4px;
}

.page-subtitle {
    color: var(--text-secondary);
    font-size: 0.875rem;
    margin-bottom: 28px;
}

/* === Glass Cards === */
.card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 20px;
    margin-bottom: 24px;
}

.glass-card {
    background: var(--card-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 1px solid var(--card-border);
    border-radius: var(--radius);
    padding: 0;
    transition: all var(--transition);
    overflow: hidden;
}

.glass-card:hover {
    border-color: var(--card-border-hover);
    transform: translateY(-1px);
    box-shadow: 0 8px 32px rgba(108, 99, 255, 0.08);
}

.card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--card-border);
}

.card-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-primary);
}

.card-body {
    padding: 16px 20px;
}

/* === Info Rows === */
.info-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid rgba(108, 99, 255, 0.06);
}

.info-row:last-child {
    border-bottom: none;
}

.info-label {
    font-size: 0.8125rem;
    color: var(--text-secondary);
}

.info-value {
    font-size: 0.8125rem;
    color: var(--text-primary);
    font-weight: 500;
}

.mono {
    font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 0.8125rem;
}

/* === Data Tables === */
.data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8125rem;
}

.data-table.compact {
    font-size: 0.75rem;
}

.data-table thead th {
    text-align: left;
    padding: 10px 12px;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 1px solid var(--card-border);
}

.data-table tbody td {
    padding: 10px 12px;
    border-bottom: 1px solid rgba(108, 99, 255, 0.06);
    color: var(--text-primary);
}

.data-table tbody tr:hover {
    background: rgba(108, 99, 255, 0.04);
}

/* === Status Dots === */
.status-dot {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8125rem;
    font-weight: 500;
}

.status-dot::before {
    content: '';
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
}

.status-dot.status-ok::before {
    background: var(--green);
    box-shadow: 0 0 8px rgba(74, 222, 128, 0.4);
}

.status-dot.status-warn::before {
    background: var(--yellow);
    box-shadow: 0 0 8px rgba(251, 191, 36, 0.4);
}

.status-dot.status-error::before {
    background: var(--red);
    box-shadow: 0 0 8px rgba(248, 113, 113, 0.4);
}

.status-dot.status-off::before {
    background: var(--text-muted);
}

.status-ok {
    color: var(--green);
}

.status-error {
    color: var(--red);
}

/* === Toggle Switches === */
.toggle-grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.toggle-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid rgba(108, 99, 255, 0.06);
}

.toggle-row:last-child {
    border-bottom: none;
}

.toggle-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-primary);
}

.toggle-switch {
    position: relative;
    width: 44px;
    height: 24px;
    border-radius: 12px;
    border: none;
    cursor: pointer;
    transition: all var(--transition);
    padding: 0;
}

.toggle-switch.on {
    background: var(--primary);
    box-shadow: 0 0 12px rgba(108, 99, 255, 0.3);
}

.toggle-switch.off {
    background: var(--text-muted);
}

.toggle-thumb {
    position: absolute;
    top: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: white;
    transition: transform var(--transition);
    pointer-events: none;
}

.toggle-switch.on .toggle-thumb {
    transform: translateX(22px);
}

.toggle-switch.off .toggle-thumb {
    transform: translateX(2px);
}

/* === Buttons === */
.btn-save {
    background: linear-gradient(135deg, var(--primary), #8B5CF6);
    color: white;
    border: none;
    padding: 10px 24px;
    border-radius: var(--radius-sm);
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition);
}

.btn-save:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 16px rgba(108, 99, 255, 0.3);
}

.btn-save:active {
    transform: translateY(0);
}

.save-section {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 20px;
}

.save-status {
    font-size: 0.8125rem;
    color: var(--green);
    font-weight: 500;
}

/* === Status / Info Grid === */
.info-grid {
    width: 100%;
    border-collapse: collapse;
}

.info-grid td {
    padding: 10px 12px;
    border-bottom: 1px solid rgba(108, 99, 255, 0.06);
    font-size: 0.875rem;
}

.info-grid td:first-child {
    color: var(--text-secondary);
    width: 160px;
}

.info-grid td:last-child {
    color: var(--text-primary);
    font-weight: 500;
}

/* === Peers Section === */
.peers-section {
    margin-top: 12px;
}

.peers-section .info-label {
    display: block;
    margin-bottom: 8px;
}

/* === Refresh Indicator === */
.last-refresh {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-bottom: 20px;
}

/* === Utility === */
.loading {
    color: var(--text-secondary);
    font-size: 0.875rem;
}

.error {
    color: var(--red);
    font-size: 0.875rem;
    padding: 12px 16px;
    background: rgba(248, 113, 113, 0.08);
    border: 1px solid rgba(248, 113, 113, 0.2);
    border-radius: var(--radius-sm);
    margin-bottom: 16px;
}

.empty {
    color: var(--text-secondary);
    font-size: 0.875rem;
}

.config-grid {
    display: flex;
    flex-direction: column;
}

/* === Responsive === */
@media (max-width: 768px) {
    #app {
        flex-direction: column;
    }

    nav.sidebar {
        width: 100%;
        min-width: unset;
        height: auto;
        flex-direction: row;
        flex-wrap: wrap;
        padding: 12px 16px;
        position: static;
    }

    nav.sidebar h1 {
        margin-bottom: 8px;
        width: 100%;
    }

    main.content {
        padding: 20px 16px;
    }

    .card-grid {
        grid-template-columns: 1fr;
    }
}
"#;

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Html {..} lang="en" dir="ltr" data-theme="dark" />
        <Title text="Phenotype Fabric" />
        <Meta name="description" content="Phenotype Fabric — capability-aware compute topology" />
        <Style>{PREMIUM_CSS}</Style>

        <Router>
            <nav class="sidebar">
                <h1>"Fabric"</h1>
                <A href="/">"Topology"</A>
                <A href="/routes">"Routes"</A>
                <A href="/capabilities">"Capabilities"</A>
                <A href="/health">"Health"</A>
                <A href="/network">"Network"</A>
                <A href="/settings">"Settings"</A>
                <A href="/stream">"Stream"</A>
            </nav>
            <main class="content">
                <Routes fallback=|| view! { <p>"Not found"</p> }>
                    <Route path=path!("/") view=TopologyPage />
                    <Route path=path!("/routes") view=RoutesPage />
                    <Route path=path!("/capabilities") view=CapabilitiesPage />
                    <Route path=path!("/health") view=HealthPage />
                    <Route path=path!("/network") view=NetworkPage />
                    <Route path=path!("/settings") view=SettingsPage />
                    <Route path=path!("/stream") view=StreamPage />
                </Routes>
            </main>
        </Router>
    }
}
