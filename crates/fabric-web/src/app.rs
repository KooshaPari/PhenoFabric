//! Leptos application components for the Fabric web frontend.

use leptos::prelude::*;
use leptos_meta::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlInputElement};
use leptos_router::components::*;
use leptos_router::path;

use crate::api::*;
use crate::webrtc_channel::WebRtcChannel;
use fabric_frame_transport::{Codec, FrameHeader, FrameMessage};

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

/// WebRTC offer request body for the daemon signaling API.
#[derive(serde::Serialize)]
struct WebrtcOfferRequest {
    #[serde(rename = "type")]
    msg_type: String,
    target: String,
    sdp: String,
}

/// WebRTC offer response from the daemon.
#[derive(serde::Deserialize)]
struct WebrtcOfferResponse {
    #[serde(rename = "type")]
    _msg_type: String,
    #[allow(dead_code)]
    target: String,
    sdp: String,
    #[allow(dead_code)]
    status: String,
}

/// WebRTC ICE candidate request body for the daemon signaling API.
#[derive(serde::Serialize)]
struct WebrtcIceRequest {
    #[serde(rename = "type")]
    msg_type: String,
    from: String,
    candidate: String,
}

/// WebRTC ICE candidate response from the daemon.
#[derive(serde::Deserialize)]
struct WebrtcIceResponse {
    #[serde(rename = "type")]
    _msg_type: String,
    #[allow(dead_code)]
    status: String,
    _from: String,
}

/// Stream page — WebRTC surface streaming from a Fabric node.
#[component]
fn StreamPage() -> impl IntoView {
    let (status, set_status) = signal("Disconnected".to_string());
    let (target_node, set_target_node) = signal(String::new());
    let (connection_log, set_connection_log) = signal(Vec::<String>::new());

    // Shared state for the peer connection and channel.
    let pc_ref: Rc<RefCell<Option<web_sys::RtcPeerConnection>>> = Rc::new(RefCell::new(None));

    let add_log = move |msg: String| {
        set_connection_log.update(|logs| logs.push(msg));
    };

    // Capture canvas element once when the component mounts.
    let canvas: Option<HtmlCanvasElement> = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|doc| doc.get_element_by_id("stream-canvas"))
        .and_then(|el| el.dyn_into::<HtmlCanvasElement>().ok());
    let canvas = Rc::new(RefCell::new(canvas));

    // Clone pc_ref so disconnect closure can use it after connect closure captures it.
    let pc_ref_disconnect = pc_ref.clone();

    let connect = move |_: web_sys::MouseEvent| {
        let target = target_node.get();
        if target.is_empty() {
            set_status.set("Enter a node ID".to_string());
            return;
        }

        set_status.set(format!("Connecting to {}...", target));
        add_log(format!("Initiating WebRTC connection to {}", target));

        let base = daemon_base_url();
        let target_clone = target.clone();
        let pc_ref = pc_ref.clone();
        let canvas_clone = canvas.clone();

        leptos::task::spawn_local(async move {
            // --- 1. Create RTCPeerConnection ---
            add_log("Creating RTCPeerConnection...".to_string());
            let ice_server = web_sys::RtcIceServer::new();
            ice_server.set_urls(
                &js_sys::Array::of1(&JsValue::from_str("stun:stun.l.google.com:19302")),
            );
            let ice_servers = js_sys::Array::of1(&ice_server.into());
            let rtc_config = web_sys::RtcConfiguration::new();
            rtc_config.set_ice_servers(&ice_servers);

            let pc = match web_sys::RtcPeerConnection::new_with_configuration(&rtc_config) {
                Ok(pc) => pc,
                Err(e) => {
                    let msg = format!("Failed to create RtcPeerConnection: {e:?}");
                    add_log(msg.clone());
                    set_status.set(msg);
                    return;
                }
            };
            *pc_ref.borrow_mut() = Some(pc.clone());

            // --- 2. Create data channel ---
            add_log("Creating data channel 'fabric-frames'...".to_string());
            let _dc = pc.create_data_channel("fabric-frames");

            // --- 3. Create SDP offer ---
            add_log("Creating SDP offer...".to_string());
            let offer_val = match wasm_bindgen_futures::JsFuture::from(pc.create_offer())
                .await
                .map_err(|e| format!("Failed to create offer: {e:?}"))
            {
                Ok(v) => v,
                Err(msg) => {
                    add_log(msg.clone());
                    set_status.set(msg);
                    return;
                }
            };

            // Extract SDP string from the returned RTCSessionDescription JS object.
            let sdp = match js_sys::Reflect::get(&offer_val, &"sdp".into()) {
                Ok(v) => match v.as_string() {
                    Some(s) => s,
                    None => {
                        let msg = "SDP is not a string".to_string();
                        add_log(msg.clone());
                        set_status.set(msg);
                        return;
                    }
                },
                Err(e) => {
                    let msg = format!("Failed to read SDP from offer: {e:?}");
                    add_log(msg.clone());
                    set_status.set(msg);
                    return;
                }
            };
            add_log(format!("SDP offer created ({} chars)", sdp.len()));

            // --- 4. Set local description ---
            add_log("Setting local description...".to_string());
            let offer_sdp_init =
                web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Offer);
            offer_sdp_init.set_sdp(&sdp);
            if let Err(e) = wasm_bindgen_futures::JsFuture::from(
                pc.set_local_description(&offer_sdp_init),
            )
            .await
            {
                let msg = format!("Failed to set local description: {e:?}");
                add_log(msg.clone());
                set_status.set(msg);
                return;
            }

            // --- 5. Send offer to daemon ---
            let offer_url = format!("{}/webrtc_offer", base);
            let offer_body = WebrtcOfferRequest {
                msg_type: "webrtc_offer".to_string(),
                target: target_clone.clone(),
                sdp: sdp.clone(),
            };
            add_log(format!("POST {} with target={}", offer_url, target_clone));

            let answer = match post_json::<WebrtcOfferResponse, _>(&offer_url, &offer_body).await
            {
                Ok(a) => a,
                Err(e) => {
                    let msg = format!("Failed to get SDP answer: {e}");
                    add_log(msg.clone());
                    set_status.set(msg);
                    return;
                }
            };
            add_log(format!(
                "Received answer: status={}, sdp_len={}",
                answer.status,
                answer.sdp.len()
            ));

            // --- 6. Set remote description with answer SDP ---
            let answer_sdp =
                web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
            answer_sdp.set_sdp(&answer.sdp);
            if let Err(e) = wasm_bindgen_futures::JsFuture::from(
                pc.set_remote_description(&answer_sdp),
            )
            .await
            {
                let msg = format!("Failed to set remote description: {e:?}");
                add_log(msg.clone());
                set_status.set(msg);
                return;
            }
            add_log("Remote description set".to_string());

            // --- 7. Register onicecandidate handler ---
            {
                let base_ice = base.clone();
                let target_ice = target_clone.clone();
                let add_log_ice = add_log.clone();

                let onice = Closure::wrap(Box::new(
                    move |event: web_sys::RtcPeerConnectionIceEvent| {
                        if let Some(candidate) = event.candidate() {
                            let cand_str = candidate.candidate();
                            let truncated = if cand_str.len() > 60 {
                                &cand_str[..60]
                            } else {
                                &cand_str
                            };
                            add_log_ice(format!("ICE candidate: {truncated}"));
                            let ice_url = format!("{}/webrtc_ice", base_ice);
                            let ice_body = WebrtcIceRequest {
                                msg_type: "webrtc_ice".to_string(),
                                from: "browser".to_string(),
                                candidate: cand_str,
                            };
                            let target_send = target_ice.clone();
                            leptos::task::spawn_local(async move {
                                match post_json::<WebrtcIceResponse, _>(&ice_url, &ice_body).await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        web_sys::console::warn_1(
                                            &format!(
                                                "Failed to relay ICE candidate to {}: {}",
                                                target_send, e
                                            )
                                            .into(),
                                        );
                                    }
                                }
                            });
                        } else {
                            add_log_ice("ICE gathering complete".to_string());
                        }
                    },
                ) as Box<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>);
                pc.set_onicecandidate(Some(onice.as_ref().unchecked_ref()));
                onice.forget();
            }

            // --- 8. Register ondatachannel handler ---
            {
                let canvas_dc = canvas_clone.clone();
                let add_log_dc = add_log.clone();
                let set_status_dc = set_status.clone();

                let ondc = Closure::wrap(Box::new(move |event: web_sys::RtcDataChannelEvent| {
                    let data_channel = event.channel();
                    add_log_dc(format!(
                        "Data channel opened: '{}'",
                        data_channel.label()
                    ));

                    let channel = WebRtcChannel::new(data_channel);

                    // Register frame message handler.
                    {
                        let canvas_render = canvas_dc.clone();
                        let set_status_frame = set_status_dc.clone();

                        channel.on_message(move |msg| match msg {
                            FrameMessage::FrameData { header, payload } => {
                                render_frame(
                                    &canvas_render,
                                    &set_status_frame,
                                    &header,
                                    &payload,
                                );
                            }
                            _ => {
                                web_sys::console::log_1(
                                    &format!("[fabric-web] Non-frame message: {:?}", msg).into(),
                                );
                            }
                        });
                    }

                    // Mark channel as open.
                    {
                        let set_status_open = set_status_dc.clone();
                        channel.on_open(move || {
                            set_status_open.set("Connected (channel open)".to_string());
                        });
                    }
                }) as Box<dyn FnMut(web_sys::RtcDataChannelEvent)>);
                pc.set_ondatachannel(Some(ondc.as_ref().unchecked_ref()));
                ondc.forget();
            }

            set_status.set("Connecting (waiting for ICE)...".to_string());
            add_log("WebRTC signaling complete. Waiting for connection...".to_string());
        });
    };

    let disconnect = move |_: web_sys::MouseEvent| {
        if let Some(pc) = pc_ref_disconnect.borrow_mut().take() {
            let _ = pc.close();
        }
        set_status.set("Disconnected".to_string());
        add_log("Connection closed".to_string());
    };

    view! {
        <h2>"Surface Stream"</h2>
        <p class="subtitle">"WebRTC-based desktop streaming from Fabric nodes"</p>

        <div class="stream-controls">
            <div class="input-group">
                <label>"Target Node"</label>
                <input
                    type="text"
                    placeholder="e.g. gpu-node-1"
                    prop:value=move || target_node.get()
                    on:input=move |ev: web_sys::Event| {
                        if let Some(target) = ev.target() {
                            if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                                set_target_node.set(input.value());
                            }
                        }
                    }
                />
            </div>
            <div class="button-group">
                <button class="btn-primary" on:click=connect>"Connect"</button>
                <button class="btn-secondary" on:click=disconnect>"Disconnect"</button>
            </div>
        </div>

        <div class="stream-status">
            <span class="status-label">"Status: "</span>
            <span class={move || {
                let s = status.get();
                if s == "Disconnected" { "status-offline" } else { "status-connected" }
            }}>
                {move || status.get()}
            </span>
        </div>

        <div class="stream-viewport">
            <canvas id="stream-canvas" width="1920" height="1080" style="width:100%;background:#1a1a2e;">
                "Video stream will appear here when connected via WebRTC data channel."
            </canvas>
        </div>

        <div class="connection-log">
            <h3>"Connection Log"</h3>
            <div class="log-entries">
                <For
                    each=move || connection_log.get()
                    key=|entry| entry.clone()
                    children=move |entry| {
                        view! { <p class="log-entry">{entry}</p> }
                    }
                />
            </div>
        </div>
    }
}

/// Render a frame to the canvas element.
///
/// For RGBA frames, the payload is painted directly via `ImageData`.
/// For encoded codecs (HEVC/AV1), a log message is emitted since WASM
/// decoding is not yet implemented.
fn render_frame(
    canvas: &Rc<RefCell<Option<HtmlCanvasElement>>>,
    set_status: &WriteSignal<String>,
    header: &FrameHeader,
    payload: &[u8],
) {
    let canvas_borrow = canvas.borrow();
    let canvas = match canvas_borrow.as_ref().clone() {
        Some(c) => c,
        None => {
            web_sys::console::warn_1(
                &"[fabric-web] Canvas element not found, cannot render frame".into(),
            );
            return;
        }
    };

    // Resize canvas if the frame dimensions differ.
    if canvas.width() != header.width || canvas.height() != header.height {
        canvas.set_width(header.width);
        canvas.set_height(header.height);
        web_sys::console::log_1(
            &format!(
                "[fabric-web] Canvas resized to {}x{}",
                header.width, header.height
            )
            .into(),
        );
    }

    let ctx = match canvas.get_context("2d") {
        Ok(Some(ctx)) => ctx,
        _ => {
            web_sys::console::warn_1(
                &"[fabric-web] Failed to get 2d canvas context".into(),
            );
            return;
        }
    };

    let ctx: web_sys::CanvasRenderingContext2d = match ctx.dyn_into() {
        Ok(c) => c,
        Err(_) => {
            web_sys::console::warn_1(
                &"[fabric-web] Canvas context is not 2d".into(),
            );
            return;
        }
    };

    match header.codec {
        Codec::Rgba => {
            // RGBA payload: paint directly via ImageData.
            let expected_len = (header.width * header.height * 4) as usize;
            if payload.len() < expected_len {
                web_sys::console::warn_1(
                    &format!(
                        "[fabric-web] RGBA payload too short: {} < {}",
                        payload.len(),
                        expected_len
                    )
                    .into(),
                );
                return;
            }

            let pixel_data = js_sys::Uint8Array::from(payload).to_vec();
            let clamped = wasm_bindgen::Clamped(pixel_data.as_slice());
            match web_sys::ImageData::new_with_u8_clamped_array_and_sh(
                clamped,
                header.width,
                header.height,
            ) {
                Ok(image_data) => {
                    if let Err(e) = ctx.put_image_data(&image_data, 0.0, 0.0) {
                        web_sys::console::warn_1(
                            &format!("[fabric-web] putImageData failed: {e:?}").into(),
                        );
                    }
                }
                Err(e) => {
                    web_sys::console::warn_1(
                        &format!("[fabric-web] ImageData creation failed: {e:?}").into(),
                    );
                }
            }
        }
        Codec::Hevc | Codec::Av1 => {
            web_sys::console::log_1(
                &format!(
                    "[fabric-web] {} decoding not yet implemented in WASM (seq={}, {}x{})",
                    header.codec.name(),
                    header.seq,
                    header.width,
                    header.height
                )
                .into(),
            );
        }
        Codec::Nv12 => {
            web_sys::console::log_1(
                &format!(
                    "[fabric-web] NV12 rendering not yet implemented (seq={}, {}x{})",
                    header.seq, header.width, header.height
                )
                .into(),
            );
        }
    }

    // Update status with frame info.
    set_status.set(format!(
        "Connected — seq={} {}x{} {}",
        header.seq,
        header.width,
        header.height,
        header.codec.name(),
    ));
}
