//! WebRTC Data Channel adapter for the Fabric frame transport protocol.
//!
//! Bridges `web_sys::RtcDataChannel` with the length-prefixed binary frame
//! protocol used by `fabric-frame-transport`. This allows the Leptos web SPA
//! to receive desktop frames over WebRTC data channels instead of TCP.

use bytes::{Bytes, BytesMut};
use fabric_frame_transport::transport::{encode_wire, parse_message};
use fabric_frame_transport::{
    FrameAck, FrameHeader, FrameMessage, KeyFrameRequest, MessageType, Ping, Pong, SessionInit,
    TransportError,
};
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Errors specific to WebRTC transport.
#[derive(Debug, Clone)]
pub enum WebRtcTransportError {
    /// The data channel is not open.
    ChannelNotOpen,
    /// Failed to send binary data through the channel.
    SendFailed(String),
    /// Failed to parse the received binary message.
    ParseFailed(String),
    /// Received an unexpected message type.
    UnexpectedMessage(String),
    /// The WebRTC API returned a JavaScript error.
    JsError(String),
}

impl std::fmt::Display for WebRtcTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChannelNotOpen => write!(f, "data channel is not open"),
            Self::SendFailed(msg) => write!(f, "send failed: {msg}"),
            Self::ParseFailed(msg) => write!(f, "parse failed: {msg}"),
            Self::UnexpectedMessage(msg) => write!(f, "unexpected message: {msg}"),
            Self::JsError(msg) => write!(f, "JS error: {msg}"),
        }
    }
}

/// Wraps a WebRTC data channel for sending/receiving Fabric frame protocol messages.
///
/// Usage:
/// ```no_run
/// // After creating an RtcDataChannel:
/// let channel = WebRtcChannel::new(data_channel);
///
/// // Send a session init:
/// channel.send_session_init(&session_init).await?;
///
/// // Receive messages (via the onmessage callback):
/// channel.on_message(|msg| {
///     match msg {
///         FrameMessage::FrameData { header, payload } => { /* render frame */ }
///         FrameMessage::FrameAck(ack) => { /* update RTT stats */ }
///         _ => {}
///     }
/// });
/// ```
pub struct WebRtcChannel {
    channel: web_sys::RtcDataChannel,
    open: Rc<Cell<bool>>,
}

impl WebRtcChannel {
    /// Create a new WebRTC channel wrapper.
    pub fn new(channel: web_sys::RtcDataChannel) -> Self {
        Self {
            channel,
            open: Rc::new(Cell::new(false)),
        }
    }

    /// Returns the underlying data channel label.
    pub fn label(&self) -> String {
        self.channel.label()
    }

    /// Returns true if the channel has been reported open via the onopen callback.
    pub fn is_open(&self) -> bool {
        self.open.get()
    }

    // ---- Sending methods ----

    /// Send a SessionInit message.
    pub fn send_session_init(&self, init: &SessionInit) -> Result<(), WebRtcTransportError> {
        let wire = encode_wire(MessageType::SessionInit, &serde_json::to_vec(init).unwrap())
            .map_err(|e| WebRtcTransportError::SendFailed(e.to_string()))?;
        self.send_wire(&wire)
    }

    /// Send a SessionAck message.
    pub fn send_session_ack(
        &self,
        ack: &fabric_frame_transport::SessionAck,
    ) -> Result<(), WebRtcTransportError> {
        let wire = encode_wire(MessageType::SessionAck, &serde_json::to_vec(ack).unwrap())
            .map_err(|e| WebRtcTransportError::SendFailed(e.to_string()))?;
        self.send_wire(&wire)
    }

    /// Send a FrameData message (header + payload).
    pub fn send_frame_data(
        &self,
        header: &FrameHeader,
        payload: &[u8],
    ) -> Result<(), WebRtcTransportError> {
        if payload.len() != header.payload_len as usize {
            return Err(WebRtcTransportError::SendFailed(format!(
                "payload length {} does not match header payload_len {}",
                payload.len(),
                header.payload_len
            )));
        }
        let mut body = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
        header.encode(&mut body);
        body.extend_from_slice(payload);
        let wire = encode_wire(MessageType::FrameData, &body)
            .map_err(|e| WebRtcTransportError::SendFailed(e.to_string()))?;
        self.send_wire(&wire)
    }

    /// Send a FrameAck message.
    pub fn send_frame_ack(&self, ack: &FrameAck) -> Result<(), WebRtcTransportError> {
        let wire = encode_wire(MessageType::FrameAck, &serde_json::to_vec(ack).unwrap())
            .map_err(|e| WebRtcTransportError::SendFailed(e.to_string()))?;
        self.send_wire(&wire)
    }

    /// Send a Ping message.
    pub fn send_ping(&self, ping: &Ping) -> Result<(), WebRtcTransportError> {
        let wire = encode_wire(MessageType::Ping, &serde_json::to_vec(ping).unwrap())
            .map_err(|e| WebRtcTransportError::SendFailed(e.to_string()))?;
        self.send_wire(&wire)
    }

    /// Send a Pong message.
    pub fn send_pong(&self, pong: &Pong) -> Result<(), WebRtcTransportError> {
        let wire = encode_wire(MessageType::Pong, &serde_json::to_vec(pong).unwrap())
            .map_err(|e| WebRtcTransportError::SendFailed(e.to_string()))?;
        self.send_wire(&wire)
    }

    /// Send an Error message.
    pub fn send_error(&self, error: &TransportError) -> Result<(), WebRtcTransportError> {
        let wire = encode_wire(MessageType::Error, &serde_json::to_vec(error).unwrap())
            .map_err(|e| WebRtcTransportError::SendFailed(e.to_string()))?;
        self.send_wire(&wire)
    }

    /// Send a KeyFrameRequest message.
    pub fn send_keyframe_request(&self, req: &KeyFrameRequest) -> Result<(), WebRtcTransportError> {
        let wire = encode_wire(
            MessageType::KeyFrameRequest,
            &serde_json::to_vec(req).unwrap(),
        )
        .map_err(|e| WebRtcTransportError::SendFailed(e.to_string()))?;
        self.send_wire(&wire)
    }

    // ---- Receiving ----

    /// Register a callback for incoming messages.
    ///
    /// The callback receives parsed `FrameMessage` values. Unparseable messages
    /// are logged to console and dropped.
    pub fn on_message<F>(&self, callback: F)
    where
        F: Fn(FrameMessage) + 'static,
    {
        let callback = Rc::new(callback);
        let callback_ref = callback.clone();

        let closure = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            if let Ok(data) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&data);
                let bytes = array.to_vec();
                if bytes.len() < 5 {
                    // Too short: 4 bytes length prefix + 1 byte message type
                    web_sys::console::warn_1(
                        &format!(
                            "[fabric-web] Received too-short message ({} bytes)",
                            bytes.len()
                        )
                        .into(),
                    );
                    return;
                }

                // Parse wire format: [4 bytes: total_len LE] [1 byte: msg_type] [payload]
                let total_len =
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
                let msg_type_byte = bytes[4];

                if total_len < 1 || bytes.len() < 4 + total_len {
                    web_sys::console::warn_1(
                        &format!(
                            "[fabric-web] Wire length mismatch: total_len={}, actual={}",
                            total_len,
                            bytes.len() - 4
                        )
                        .into(),
                    );
                    return;
                }

                let payload_start = 5; // 4 (length) + 1 (type)
                let payload_end = 4 + total_len;
                let payload = Bytes::copy_from_slice(&bytes[payload_start..payload_end]);

                let msg_type = match MessageType::from_u8(msg_type_byte) {
                    Some(mt) => mt,
                    None => {
                        web_sys::console::warn_1(
                            &format!("[fabric-web] Unknown message type: {msg_type_byte}").into(),
                        );
                        return;
                    }
                };

                match parse_message(msg_type, payload) {
                    Ok(msg) => callback_ref(msg),
                    Err(e) => {
                        web_sys::console::warn_1(
                            &format!("[fabric-web] Failed to parse message: {e}").into(),
                        );
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        self.channel
            .set_onmessage(Some(closure.as_ref().unchecked_ref()));
        closure.forget(); // Leak the closure to keep it alive
    }

    /// Register a callback for channel state changes.
    pub fn on_open<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        let open_flag = self.open.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            open_flag.set(true);
            callback();
        }) as Box<dyn FnMut(web_sys::Event)>);

        self.channel
            .set_onopen(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    /// Register a callback for channel close.
    pub fn on_close<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        let open_flag = self.open.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            open_flag.set(false);
            callback();
        }) as Box<dyn FnMut(web_sys::Event)>);

        self.channel
            .set_onclose(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    /// Register a callback for channel errors.
    pub fn on_error<F>(&self, callback: F)
    where
        F: Fn(String) + 'static,
    {
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let msg = event
                .dyn_ref::<web_sys::ErrorEvent>()
                .map(|e| e.message().to_string())
                .unwrap_or_else(|| "unknown error".to_string());
            callback(msg);
        }) as Box<dyn FnMut(web_sys::Event)>);

        self.channel
            .set_onerror(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    /// Close the data channel.
    pub fn close(&self) {
        self.channel.close();
    }

    // ---- Private ----

    fn send_wire(&self, wire: &[u8]) -> Result<(), WebRtcTransportError> {
        if !self.is_open() {
            return Err(WebRtcTransportError::ChannelNotOpen);
        }
        self.channel
            .send_with_u8_array(wire)
            .map_err(|e| WebRtcTransportError::SendFailed(format!("{e:?}")))
    }
}
