//! fabric-web: Leptos WASM SPA for Phenotype Fabric.
//!
//! Single-page application that connects to the Fabric daemon wire server
//! for topology visualization, route management, and surface streaming.

pub mod app;
pub mod api;

use leptos::mount::mount_to_body;
use wasm_bindgen::prelude::*;

/// WASM entry point for client-side hydration.
#[wasm_bindgen]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(app::App);
}
