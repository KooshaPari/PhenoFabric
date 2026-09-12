//! fabric-web: Leptos-based web frontend for Phenotype Fabric.
//!
//! Single-page application providing topology visualization, route
//! management, and daemon health monitoring in the browser.
//! Supports SSR via leptos_actix and client-side hydration.

pub mod app;

use wasm_bindgen::prelude::*;

/// Entry point for WASM client-side hydration.
#[wasm_bindgen]
pub fn hydrate() {
    #[cfg(feature = "hydrate")]
    {
        _ = console_error_panic_hook::set_once();
        leptos::mount_to_body(app::App);
    }
}
