//! Frame rate and latency statistics tracker.

use std::time::{Duration, Instant};

/// Tracks frame timing statistics for adaptive bitrate/streaming.
#[derive(Debug)]
pub struct FrameStats {
    /// Total frames sent.
    pub frames_sent: u64,
    /// Total bytes sent.
    pub bytes_sent: u64,
    /// Average frame size in bytes.
    pub avg_frame_size: f64,
    /// Current frame rate (frames per second).
    pub current_fps: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Dropped frames.
    pub dropped_frames: u64,
    /// Last frame timestamp.
    last_frame_time: Instant,
    /// FPS calculation window.
    fps_window: Vec<Instant>,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameStats {
    pub fn new() -> Self {
        Self {
            frames_sent: 0,
            bytes_sent: 0,
            avg_frame_size: 0.0,
            current_fps: 0.0,
            avg_latency_ms: 0.0,
            dropped_frames: 0,
            last_frame_time: Instant::now(),
            fps_window: Vec::with_capacity(120),
        }
    }

    /// Record a sent frame.
    pub fn record_frame(&mut self, payload_size: usize) {
        let now = Instant::now();
        self.frames_sent += 1;
        self.bytes_sent += payload_size as u64;
        self.avg_frame_size = self.bytes_sent as f64 / self.frames_sent as f64;

        self.fps_window.push(now);
        let cutoff = now - Duration::from_secs(1);
        self.fps_window.retain(|t| *t > cutoff);
        self.current_fps = self.fps_window.len() as f64;

        self.last_frame_time = now;
    }

    /// Record latency measurement.
    pub fn record_latency(&mut self, latency_ms: f64) {
        if self.avg_latency_ms == 0.0 {
            self.avg_latency_ms = latency_ms;
        } else {
            self.avg_latency_ms = self.avg_latency_ms * 0.9 + latency_ms * 0.1;
        }
    }

    /// Record a dropped frame.
    pub fn record_drop(&mut self) {
        self.dropped_frames += 1;
    }

    /// Reset all statistics.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_tracking() {
        let mut stats = FrameStats::new();
        for _ in 0..30 {
            stats.record_frame(1000);
        }
        assert_eq!(stats.frames_sent, 30);
        assert_eq!(stats.bytes_sent, 30_000);
        assert_eq!(stats.avg_frame_size, 1000.0);
    }

    #[test]
    fn latency_ema() {
        let mut stats = FrameStats::new();
        stats.record_latency(10.0);
        assert_eq!(stats.avg_latency_ms, 10.0);
        stats.record_latency(20.0);
        assert!((stats.avg_latency_ms - 11.0).abs() < 0.1);
    }

    #[test]
    fn drops_and_reset() {
        let mut stats = FrameStats::new();
        stats.record_drop();
        stats.record_drop();
        assert_eq!(stats.dropped_frames, 2);
        stats.reset();
        assert_eq!(stats.dropped_frames, 0);
    }
}
