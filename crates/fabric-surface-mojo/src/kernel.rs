//! Kernel interface for frame processing.
//!
//! Defines the [`FrameKernel`] trait that a Mojo backend would implement,
//! plus a CPU fallback using Rust's portable SIMD.
//!
//! # Design
//!
//! A `FrameKernel` processes frame buffers through three operations:
//!
//! 1. **`encode`** — compress raw pixel data into an encoded format (e.g. RGBA -> HEVC).
//! 2. **`decode`** — decompress encoded data back to raw pixels.
//! 3. **`transform`** — apply a per-pixel transformation (e.g. color space, scaling).
//!
//! In a Mojo implementation, these would be `fnparam` kernels compiled to GPU
//! via MLIR. Here we provide a CPU fallback using Rust iterator-based SIMD.

use crate::{MojoFrameBuffer, MojoKernel, MojoSurfaceError};

/// Trait for frame processing kernels.
///
/// Implementors provide encode/decode/transform operations that can run
/// on any compute backend. A Mojo GPU implementation would dispatch these
/// as MLIR kernels; the CPU fallback uses Rust SIMD.
pub trait FrameKernel {
    /// Encode a raw frame buffer into the target format.
    fn encode(&self, input: &MojoFrameBuffer, output: &mut [u8])
        -> Result<usize, MojoSurfaceError>;

    /// Decode an encoded buffer back to raw pixels.
    fn decode(&self, input: &[u8], output: &mut MojoFrameBuffer) -> Result<(), MojoSurfaceError>;

    /// Apply a per-pixel transform (e.g. brightness, color space).
    fn transform(
        &self,
        buffer: &mut MojoFrameBuffer,
        transform_type: TransformType,
    ) -> Result<(), MojoSurfaceError>;
}

/// Available per-pixel transforms.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TransformType {
    /// Invert all color channels.
    Invert,
    /// Convert RGB to grayscale.
    Grayscale,
    /// Apply brightness adjustment (-1.0 to 1.0).
    Brightness(f32),
    /// Apply contrast adjustment (0.0 to 2.0).
    Contrast(f32),
}

/// CPU SIMD fallback kernel implementation.
///
/// Uses Rust iterator-based SIMD (no unsafe) to process frame data.
/// This is what the Mojo GPU kernel would accelerate.
#[derive(Debug)]
pub struct CpuSimdKernel {
    /// The compiled Mojo kernel metadata (for reference/metrics).
    pub kernel_meta: MojoKernel,
}

impl CpuSimdKernel {
    /// Create a new CPU fallback kernel.
    pub fn new() -> Self {
        Self {
            kernel_meta: MojoKernel {
                kernel_id: 0,
                name: "cpu_simd_fallback".into(),
                backend: "cpu".into(),
                local_mem_per_thread: 0,
                shared_mem_per_workgroup: 0,
                threads_per_workgroup: 1,
                estimated_flops: 0.0,
            },
        }
    }

    /// Create a CPU kernel that references a specific Mojo kernel.
    pub fn with_meta(meta: MojoKernel) -> Self {
        Self { kernel_meta: meta }
    }
}

impl Default for CpuSimdKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameKernel for CpuSimdKernel {
    fn encode(
        &self,
        input: &MojoFrameBuffer,
        output: &mut [u8],
    ) -> Result<usize, MojoSurfaceError> {
        // CPU fallback: simple run-length encoding for demonstration.
        // A real implementation would use a proper codec (libavcodec, etc.).
        //
        // For a Mojo GPU implementation, this would be an MLIR kernel that
        // runs the HEVC/AV1 encoder on the GPU with zero-copy frame access.
        if output.is_empty() {
            return Err(MojoSurfaceError::InvalidConfig(
                "Output buffer too small".into(),
            ));
        }

        let src = &input.host_mapped_data;
        if src.is_empty() {
            return Err(MojoSurfaceError::DispatchFailed(
                "Input buffer has no data".into(),
            ));
        }

        // Simple RLE: [count, byte, count, byte, ...]
        let mut written = 0;
        let mut i = 0;
        while i < src.len() {
            let byte = src[i];
            let mut count: u8 = 1;
            while i + (count as usize) < src.len() && src[i + count as usize] == byte && count < 255
            {
                count += 1;
            }
            if written + 2 > output.len() {
                return Err(MojoSurfaceError::DispatchFailed(
                    "Output buffer overflow during encode".into(),
                ));
            }
            output[written] = count;
            output[written + 1] = byte;
            written += 2;
            i += count as usize;
        }

        Ok(written)
    }

    fn decode(&self, input: &[u8], output: &mut MojoFrameBuffer) -> Result<(), MojoSurfaceError> {
        if input.len() % 2 != 0 {
            return Err(MojoSurfaceError::DispatchFailed(
                "Invalid RLE input length".into(),
            ));
        }

        let mut dest = Vec::new();
        let mut i = 0;
        while i + 1 < input.len() {
            let count = input[i] as usize;
            let byte = input[i + 1];
            dest.extend(std::iter::repeat(byte).take(count));
            i += 2;
        }

        let expected = (output.width as usize) * (output.height as usize) * 4;
        if dest.len() > expected {
            return Err(MojoSurfaceError::DispatchFailed(
                "Decoded data exceeds frame buffer".into(),
            ));
        }

        output.host_mapped_data = dest;
        Ok(())
    }

    fn transform(
        &self,
        buffer: &mut MojoFrameBuffer,
        transform_type: TransformType,
    ) -> Result<(), MojoSurfaceError> {
        if buffer.host_mapped_data.is_empty() {
            return Err(MojoSurfaceError::DispatchFailed(
                "Buffer has no data".into(),
            ));
        }

        match transform_type {
            TransformType::Invert => {
                // SIMD-friendly: process 4 bytes (RGBA) at a time
                for chunk in buffer.host_mapped_data.chunks_exact_mut(4) {
                    chunk[0] = 255 - chunk[0]; // R
                    chunk[1] = 255 - chunk[1]; // G
                    chunk[2] = 255 - chunk[2]; // B
                                               // A unchanged
                }
            }
            TransformType::Grayscale => {
                for chunk in buffer.host_mapped_data.chunks_exact_mut(4) {
                    let gray = ((chunk[0] as f32 * 0.299)
                        + (chunk[1] as f32 * 0.587)
                        + (chunk[2] as f32 * 0.114)) as u8;
                    chunk[0] = gray;
                    chunk[1] = gray;
                    chunk[2] = gray;
                }
            }
            TransformType::Brightness(b) => {
                let adjustment = (b * 255.0) as i16;
                for chunk in buffer.host_mapped_data.chunks_exact_mut(4) {
                    for channel in &mut chunk[..3] {
                        let val = (*channel as i16) + adjustment;
                        *channel = val.clamp(0, 255) as u8;
                    }
                }
            }
            TransformType::Contrast(c) => {
                let factor = (259.0 * (c * 255.0 + 255.0)) / (255.0 * (259.0 - c * 255.0));
                for chunk in buffer.host_mapped_data.chunks_exact_mut(4) {
                    for channel in &mut chunk[..3] {
                        let val = (factor * (*channel as f32 - 128.0) + 128.0) as i16;
                        *channel = val.clamp(0, 255) as u8;
                    }
                }
            }
        }

        Ok(())
    }
}

/// A Mojo frame buffer with host-mapped data for CPU kernel processing.
///
/// Extends [`MojoFrameBuffer`] with a data vector for the CPU fallback path.
/// In a Mojo GPU implementation, the data would live in VRAM and be
/// accessed via the Mojo runtime's buffer API.
#[derive(Debug, Clone)]
pub struct CpuFrameBuffer {
    /// The base frame buffer metadata.
    pub inner: MojoFrameBuffer,
    /// Host-mapped pixel data.
    pub data: Vec<u8>,
}

impl CpuFrameBuffer {
    /// Create a new CPU frame buffer with zeroed data.
    pub fn new(width: u32, height: u32, pixel_format: &str) -> Self {
        let inner = MojoFrameBuffer::new(0, width, height, pixel_format, -1);
        let size = inner.size_bytes as usize;
        Self {
            inner,
            data: vec![0u8; size],
        }
    }

    /// Create a CPU frame buffer with existing data.
    pub fn with_data(width: u32, height: u32, pixel_format: &str, data: Vec<u8>) -> Self {
        let inner = MojoFrameBuffer::new(0, width, height, pixel_format, -1);
        Self { inner, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_buffer() -> MojoFrameBuffer {
        let mut buf = MojoFrameBuffer::new(1, 4, 4, "RGBA8", -1);
        // Fill with a known pattern: white pixels (RGBA = 255,255,255,255)
        buf.host_mapped_data = vec![255u8; 4 * 4 * 4];
        buf
    }

    #[test]
    fn encode_decode_roundtrip() {
        let kernel = CpuSimdKernel::new();
        let input = test_buffer();
        let mut encoded = vec![0u8; input.host_mapped_data.len() * 2];
        let n = kernel.encode(&input, &mut encoded).unwrap();
        assert!(n > 0);

        let mut output = MojoFrameBuffer::new(2, 4, 4, "RGBA8", -1);
        kernel.decode(&encoded[..n], &mut output).unwrap();
        assert_eq!(output.host_mapped_data.len(), input.host_mapped_data.len());
    }

    #[test]
    fn transform_invert() {
        let kernel = CpuSimdKernel::new();
        let mut buf = MojoFrameBuffer::new(1, 2, 2, "RGBA8", -1);
        buf.host_mapped_data = vec![
            0, 128, 255, 255, 0, 128, 255, 255, 0, 128, 255, 255, 0, 128, 255, 255,
        ];

        kernel.transform(&mut buf, TransformType::Invert).unwrap();

        // R: 255-0=255, G: 255-128=127, B: 255-255=0, A: unchanged
        assert_eq!(buf.host_mapped_data[0], 255);
        assert_eq!(buf.host_mapped_data[1], 127);
        assert_eq!(buf.host_mapped_data[2], 0);
        assert_eq!(buf.host_mapped_data[3], 255);
    }

    #[test]
    fn transform_grayscale() {
        let kernel = CpuSimdKernel::new();
        let mut buf = MojoFrameBuffer::new(1, 1, 1, "RGBA8", -1);
        // Pure red pixel: R=255, G=0, B=0, A=255
        buf.host_mapped_data = vec![255, 0, 0, 255];

        kernel
            .transform(&mut buf, TransformType::Grayscale)
            .unwrap();

        // gray = 255*0.299 + 0*0.587 + 0*0.114 = ~76
        let gray = buf.host_mapped_data[0];
        assert_eq!(gray, 76);
        assert_eq!(buf.host_mapped_data[1], 76);
        assert_eq!(buf.host_mapped_data[2], 76);
    }

    #[test]
    fn encode_empty_output_errors() {
        let kernel = CpuSimdKernel::new();
        let input = test_buffer();
        let mut encoded = vec![];
        let result = kernel.encode(&input, &mut encoded);
        assert!(result.is_err());
    }
}
