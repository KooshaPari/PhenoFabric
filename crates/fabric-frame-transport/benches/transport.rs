//! Criterion benchmarks for fabric-frame-transport encode/decode paths.

use bytes::BytesMut;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use fabric_frame_transport::{
    Codec, FrameHeader, FrameMessage, MessageType, SessionInit, PROTOCOL_VERSION,
};
use fabric_frame_transport::transport::{encode_wire, parse_message};

// ---------------------------------------------------------------------------
// Existing benchmarks: RGBA frame encode/decode
// ---------------------------------------------------------------------------

/// Benchmark encoding a 1080p RGBA frame header to wire format.
fn bench_encode_frame_1080p(c: &mut Criterion) {
    let header = FrameHeader {
        seq: 1,
        pts_us: 16_667,
        dts_us: 16_667,
        is_keyframe: true,
        codec: Codec::Rgba,
        width: 1920,
        height: 1080,
        payload_len: (1920 * 1080 * 4) as u32, // RGBA
        duration_us: 16_667,
    };

    let payload = vec![0u8; 1920 * 1080 * 4];
    let total_bytes = FrameHeader::SERIALIZED_SIZE + payload.len();

    let mut group = c.benchmark_group("encode_frame_1080p_rgba");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("frame_header_encode", |b| {
        b.iter(|| {
            let mut buf = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE);
            header.encode(black_box(&mut buf));
            black_box(&buf);
        });
    });

    group.bench_function("wire_encode_full", |b| {
        b.iter(|| {
            let mut body =
                BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
            header.encode(&mut body);
            body.extend_from_slice(&payload);
            let wire = encode_wire(MessageType::FrameData, black_box(&body)).unwrap();
            black_box(&wire);
        });
    });

    group.bench_function("header_payload_combined", |b| {
        b.iter(|| {
            let mut body =
                BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
            header.encode(&mut body);
            body.extend_from_slice(&payload);
            black_box(&body);
        });
    });

    group.finish();
}

/// Benchmark decoding a 1080p RGBA frame header from wire format.
fn bench_decode_frame_1080p(c: &mut Criterion) {
    let header = FrameHeader {
        seq: 1,
        pts_us: 16_667,
        dts_us: 16_667,
        is_keyframe: true,
        codec: Codec::Rgba,
        width: 1920,
        height: 1080,
        payload_len: (1920 * 1080 * 4) as u32,
        duration_us: 16_667,
    };
    let payload = vec![0u8; 1920 * 1080 * 4];

    // Prepare encoded wire format
    let mut body = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
    header.encode(&mut body);
    body.extend_from_slice(&payload);
    let wire = encode_wire(MessageType::FrameData, &body).unwrap();
    let wire_bytes = wire.freeze();
    let total_bytes = wire_bytes.len();

    // Pre-encode just the header for header-only decode benchmark
    let mut header_buf = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE);
    header.encode(&mut header_buf);
    let header_bytes = header_buf.freeze();

    let mut group = c.benchmark_group("decode_frame_1080p_rgba");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("frame_header_decode", |b| {
        b.iter(|| {
            let mut clone = header_bytes.clone();
            let decoded = FrameHeader::decode(black_box(&mut clone)).unwrap();
            black_box(&decoded);
        });
    });

    group.bench_function("wire_decode_full", |b| {
        b.iter(|| {
            let msg = parse_message(
                MessageType::FrameData,
                black_box(wire_bytes.clone()),
            )
            .unwrap();
            black_box(&msg);
        });
    });

    group.finish();
}

/// Benchmark encode+decode roundtrip on a small 64x64 RGBA frame.
fn bench_roundtrip_small(c: &mut Criterion) {
    let header = FrameHeader {
        seq: 1,
        pts_us: 16_667,
        dts_us: 16_667,
        is_keyframe: true,
        codec: Codec::Rgba,
        width: 64,
        height: 64,
        payload_len: (64 * 64 * 4) as u32,
        duration_us: 16_667,
    };
    let payload = vec![0xABu8; 64 * 64 * 4];

    let mut body = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
    header.encode(&mut body);
    body.extend_from_slice(&payload);
    let wire = encode_wire(MessageType::FrameData, &body).unwrap().freeze();

    c.bench_function("roundtrip_small_64x64", |b| {
        b.iter(|| {
            let msg = parse_message(MessageType::FrameData, black_box(wire.clone())).unwrap();
            if let FrameMessage::FrameData { header, payload } = msg {
                let mut buf = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
                header.encode(&mut buf);
                buf.extend_from_slice(&payload);
                let wire = encode_wire(MessageType::FrameData, &buf).unwrap();
                black_box(&wire);
            }
        });
    });
}

// ---------------------------------------------------------------------------
// New benchmarks: NV12 encode/decode, SessionInit roundtrip, 60fps throughput
// ---------------------------------------------------------------------------

/// NV12 frame size for 1920x1080: Y plane (W*H) + interleaved UV plane (W*H/2).
const NV12_1080P_SIZE: usize = 1920 * 1080 + (1920 * 1080 / 2); // 3,110,400 bytes

/// Benchmark encoding a 1920x1080 NV12 frame.
fn bench_encode_frame_nv12_1080p(c: &mut Criterion) {
    let header = FrameHeader {
        seq: 1,
        pts_us: 16_667,
        dts_us: 16_667,
        is_keyframe: true,
        codec: Codec::Nv12,
        width: 1920,
        height: 1080,
        payload_len: NV12_1080P_SIZE as u32,
        duration_us: 16_667,
    };
    let payload = vec![0u8; NV12_1080P_SIZE];
    let total_bytes = FrameHeader::SERIALIZED_SIZE + payload.len();

    let mut group = c.benchmark_group("encode_frame_nv12_1080p");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("header_encode", |b| {
        b.iter(|| {
            let mut buf = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE);
            header.encode(black_box(&mut buf));
            black_box(&buf);
        });
    });

    group.bench_function("wire_encode_full", |b| {
        b.iter(|| {
            let mut body =
                BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
            header.encode(&mut body);
            body.extend_from_slice(&payload);
            let wire = encode_wire(MessageType::FrameData, black_box(&body)).unwrap();
            black_box(&wire);
        });
    });

    group.finish();
}

/// Benchmark decoding a 1920x1080 NV12 frame.
fn bench_decode_frame_nv12_1080p(c: &mut Criterion) {
    let header = FrameHeader {
        seq: 1,
        pts_us: 16_667,
        dts_us: 16_667,
        is_keyframe: true,
        codec: Codec::Nv12,
        width: 1920,
        height: 1080,
        payload_len: NV12_1080P_SIZE as u32,
        duration_us: 16_667,
    };
    let payload = vec![0u8; NV12_1080P_SIZE];

    // Prepare encoded wire format
    let mut body = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
    header.encode(&mut body);
    body.extend_from_slice(&payload);
    let wire = encode_wire(MessageType::FrameData, &body).unwrap();
    let wire_bytes = wire.freeze();
    let total_bytes = wire_bytes.len();

    // Pre-encode header for header-only decode
    let mut header_buf = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE);
    header.encode(&mut header_buf);
    let header_bytes = header_buf.freeze();

    let mut group = c.benchmark_group("decode_frame_nv12_1080p");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("header_decode", |b| {
        b.iter(|| {
            let mut clone = header_bytes.clone();
            let decoded = FrameHeader::decode(black_box(&mut clone)).unwrap();
            black_box(&decoded);
        });
    });

    group.bench_function("wire_decode_full", |b| {
        b.iter(|| {
            let msg = parse_message(
                MessageType::FrameData,
                black_box(wire_bytes.clone()),
            )
            .unwrap();
            black_box(&msg);
        });
    });

    group.finish();
}

/// Benchmark encode + decode roundtrip on a SessionInit message.
fn bench_roundtrip_session_init(c: &mut Criterion) {
    let init = SessionInit {
        version: PROTOCOL_VERSION,
        preferred_codec: Codec::Nv12,
        width: 1920,
        height: 1080,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: "bench-client-001".into(),
    };

    // Pre-encode to wire format for decode benchmark
    let json_payload = serde_json::to_vec(&init).unwrap();
    let wire = encode_wire(MessageType::SessionInit, &json_payload)
        .unwrap()
        .freeze();

    let mut group = c.benchmark_group("roundtrip_session_init");
    group.throughput(Throughput::Bytes(wire.len() as u64));

    group.bench_function("encode", |b| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(&init)).unwrap();
            let w = encode_wire(MessageType::SessionInit, &json).unwrap();
            black_box(&w);
        });
    });

    group.bench_function("decode", |b| {
        b.iter(|| {
            let msg = parse_message(
                MessageType::SessionInit,
                black_box(wire.clone()),
            )
            .unwrap();
            black_box(&msg);
        });
    });

    group.bench_function("full_roundtrip", |b| {
        b.iter(|| {
            // Encode
            let json = serde_json::to_vec(&init).unwrap();
            let w = encode_wire(MessageType::SessionInit, &json).unwrap();
            // Decode
            let msg = parse_message(MessageType::SessionInit, w.freeze()).unwrap();
            black_box(&msg);
        });
    });

    group.finish();
}

/// Benchmark encoding 60 NV12 frames sequentially (simulating 1s at 60fps).
///
/// Measures the sustained throughput a daemon would achieve when streaming
/// a 1080p NV12 surface at the target frame rate.
fn bench_throughput_1080p_60fps(c: &mut Criterion) {
    let payload = vec![0u8; NV12_1080P_SIZE];

    c.bench_function("throughput_1080p_nv12_60fps", |b| {
        b.iter(|| {
            for seq in 0..60u64 {
                let header = FrameHeader {
                    seq,
                    pts_us: seq * 16_667,
                    dts_us: seq * 16_667,
                    is_keyframe: seq == 0,
                    codec: Codec::Nv12,
                    width: 1920,
                    height: 1080,
                    payload_len: NV12_1080P_SIZE as u32,
                    duration_us: 16_667,
                };
                let mut body =
                    BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
                header.encode(&mut body);
                body.extend_from_slice(&payload);
                let wire = encode_wire(MessageType::FrameData, black_box(&body)).unwrap();
                black_box(&wire);
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Register all benchmarks
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_encode_frame_1080p,
    bench_decode_frame_1080p,
    bench_roundtrip_small,
    bench_encode_frame_nv12_1080p,
    bench_decode_frame_nv12_1080p,
    bench_roundtrip_session_init,
    bench_throughput_1080p_60fps,
);
criterion_main!(benches);
