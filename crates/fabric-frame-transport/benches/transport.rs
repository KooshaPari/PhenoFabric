//! Criterion benchmarks for fabric-frame-transport encode/decode paths.

use bytes::BytesMut;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use fabric_frame_transport::{Codec, FrameHeader, MessageType, SessionInit, PROTOCOL_VERSION};
use fabric_frame_transport::transport::{encode_wire, parse_message};

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

    let mut group = c.benchmark_group("encode_frame_1080p");
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

/// Benchmark decoding a 1080p frame header from wire format.
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

    let mut group = c.benchmark_group("decode_frame_1080p");
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

/// Benchmark encode+decode roundtrip on a small 64x64 frame.
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
    let total_bytes = encode_wire(MessageType::FrameData, &body).unwrap().len();

    let mut group = c.benchmark_group("roundtrip_small_64x64");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("encode_decode", |b| {
        b.iter(|| {
            // Encode
            let mut body = BytesMut::with_capacity(
                FrameHeader::SERIALIZED_SIZE + payload.len(),
            );
            header.encode(&mut body);
            body.extend_from_slice(&payload);
            let wire = encode_wire(MessageType::FrameData, &body).unwrap();

            // Decode
            let msg = parse_message(MessageType::FrameData, wire.freeze()).unwrap();
            black_box(&msg);
        });
    });

    group.finish();
}

/// Benchmark encode+decode roundtrip on a large 3840x2160 frame.
fn bench_roundtrip_large(c: &mut Criterion) {
    let header = FrameHeader {
        seq: 1,
        pts_us: 16_667,
        dts_us: 16_667,
        is_keyframe: true,
        codec: Codec::Rgba,
        width: 3840,
        height: 2160,
        payload_len: (3840 * 2160 * 4) as u32,
        duration_us: 16_667,
    };
    let payload = vec![0xCDu8; 3840 * 2160 * 4];

    let mut body = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE + payload.len());
    header.encode(&mut body);
    body.extend_from_slice(&payload);
    let total_bytes = encode_wire(MessageType::FrameData, &body).unwrap().len();

    let mut group = c.benchmark_group("roundtrip_large_3840x2160");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("encode_decode", |b| {
        b.iter(|| {
            let mut body = BytesMut::with_capacity(
                FrameHeader::SERIALIZED_SIZE + payload.len(),
            );
            header.encode(&mut body);
            body.extend_from_slice(&payload);
            let wire = encode_wire(MessageType::FrameData, &body).unwrap();

            let msg = parse_message(MessageType::FrameData, wire.freeze()).unwrap();
            black_box(&msg);
        });
    });

    group.finish();
}

/// Benchmark throughput for encoding/decoding SessionInit JSON messages.
fn bench_session_init_throughput(c: &mut Criterion) {
    let init = SessionInit {
        version: PROTOCOL_VERSION,
        preferred_codec: Codec::Hevc,
        width: 1920,
        height: 1080,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: "bench-client".to_string(),
    };
    let payload = serde_json::to_vec(&init).unwrap();
    let wire = encode_wire(MessageType::SessionInit, &payload).unwrap();
    let total_bytes = wire.len();

    let mut group = c.benchmark_group("session_init");
    group.throughput(Throughput::Bytes(total_bytes as u64));

    group.bench_function("json_encode_decode", |b| {
        b.iter(|| {
            let p = serde_json::to_vec(black_box(&init)).unwrap();
            let w = encode_wire(MessageType::SessionInit, &p).unwrap();
            let msg = parse_message(MessageType::SessionInit, w.freeze()).unwrap();
            black_box(&msg);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encode_frame_1080p,
    bench_decode_frame_1080p,
    bench_roundtrip_small,
    bench_roundtrip_large,
    bench_session_init_throughput,
);
criterion_main!(benches);
