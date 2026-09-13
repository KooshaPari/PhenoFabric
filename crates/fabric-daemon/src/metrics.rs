//! Prometheus metrics for fabric-daemon.
//!
//! Provides counters, histograms, and gauges for daemon health monitoring.
//! Serves metrics in Prometheus text format on a dedicated HTTP endpoint.

use prometheus::{Encoder, Gauge, GaugeVec, Histogram, HistogramVec, IntCounter, IntCounterVec, Registry, TextEncoder};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

/// All Prometheus metrics for the fabric daemon.
pub struct DaemonMetrics {
    /// The Prometheus registry holding all metrics.
    pub registry: Registry,
    /// Total messages processed, labeled by message_type.
    pub messages_total: IntCounterVec,
    /// Duration of message processing, labeled by message_type.
    pub message_duration_seconds: HistogramVec,
    /// Number of currently active connections.
    pub connections_active: Gauge,
    /// Number of currently active leases.
    pub leases_active: Gauge,
    /// Number of currently active routes.
    pub routes_active: Gauge,
    /// Total topology epochs.
    pub epochs_total: IntCounter,
    /// Total errors, labeled by error_type.
    pub errors_total: IntCounterVec,
    /// Daemon uptime in seconds.
    pub uptime_seconds: Gauge,
}

impl DaemonMetrics {
    /// Create a new set of daemon metrics and register them.
    pub fn new() -> Self {
        let registry = Registry::new();

        let messages_total = IntCounterVec::new(
            prometheus::opts!(
                "fabric_daemon_messages_total",
                "Total messages processed"
            ),
            &["message_type"],
        )
        .expect("failed to create messages_total metric");

        let message_duration_seconds = HistogramVec::new(
            prometheus::histogram_opts!(
                "fabric_daemon_message_duration_seconds",
                "Message processing duration in seconds"
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
            &["message_type"],
        )
        .expect("failed to create message_duration_seconds metric");

        let connections_active = Gauge::new(
            "fabric_daemon_connections_active",
            "Number of active connections",
        )
        .expect("failed to create connections_active metric");

        let leases_active = Gauge::new(
            "fabric_daemon_leases_active",
            "Number of active leases",
        )
        .expect("failed to create leases_active metric");

        let routes_active = Gauge::new(
            "fabric_daemon_routes_active",
            "Number of active routes",
        )
        .expect("failed to create routes_active metric");

        let epochs_total =
            IntCounter::new("fabric_daemon_epochs_total", "Total topology epochs")
                .expect("failed to create epochs_total metric");

        let errors_total = IntCounterVec::new(
            prometheus::opts!("fabric_daemon_errors_total", "Total errors"),
            &["error_type"],
        )
        .expect("failed to create errors_total metric");

        let uptime_seconds = Gauge::new(
            "fabric_daemon_uptime_seconds",
            "Daemon uptime in seconds",
        )
        .expect("failed to create uptime_seconds metric");

        // Register all metrics with the registry.
        registry
            .register(Box::new(messages_total.clone()))
            .expect("failed to register messages_total");
        registry
            .register(Box::new(message_duration_seconds.clone()))
            .expect("failed to register message_duration_seconds");
        registry
            .register(Box::new(connections_active.clone()))
            .expect("failed to register connections_active");
        registry
            .register(Box::new(leases_active.clone()))
            .expect("failed to register leases_active");
        registry
            .register(Box::new(routes_active.clone()))
            .expect("failed to register routes_active");
        registry
            .register(Box::new(epochs_total.clone()))
            .expect("failed to register epochs_total");
        registry
            .register(Box::new(errors_total.clone()))
            .expect("failed to register errors_total");
        registry
            .register(Box::new(uptime_seconds.clone()))
            .expect("failed to register uptime_seconds");

        Self {
            registry,
            messages_total,
            message_duration_seconds,
            connections_active,
            leases_active,
            routes_active,
            epochs_total,
            errors_total,
            uptime_seconds,
        }
    }
}

/// Start the Prometheus metrics HTTP server on the given port.
///
/// Returns a thread handle for the metrics server thread.
pub fn start_metrics_server(registry: Arc<Registry>, port: u16) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let addr = format!("0.0.0.0:{port}");
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => {
                error!("failed to bind metrics server to {addr}: {e}");
                return;
            }
        };
        info!(port, "metrics server started");

        // Ignore the WouldBlock error from set_nonblocking.
        let _ = listener.set_nonblocking(false);

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let reader_stream = match stream.try_clone() {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let mut reader = BufReader::new(reader_stream);

                    // Read the request line.
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
                        continue;
                    }

                    // Consume remaining headers until blank line.
                    let mut header = String::new();
                    loop {
                        header.clear();
                        if reader.read_line(&mut header).unwrap_or(0) == 0
                            || header.trim().is_empty()
                        {
                            break;
                        }
                    }

                    if request_line.starts_with("GET /metrics") {
                        let encoder = TextEncoder::new();
                        let metric_families = registry.gather();
                        let mut buffer = Vec::new();
                        if encoder.encode(&metric_families, &mut buffer).is_err() {
                            let body = "Internal Server Error";
                            let response = format!(
                                "HTTP/1.1 500 Internal Server Error\r\n\
                                 Content-Type: text/plain\r\n\
                                 Content-Length: {}\r\n\
                                 \r\n\
                                 {body}",
                                body.len()
                            );
                            let _ = stream.write_all(response.as_bytes());
                            continue;
                        }

                        let content_type = encoder.format_type();
                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
                             Content-Type: {content_type}\r\n\
                             Content-Length: {}\r\n\
                             \r\n",
                            buffer.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.write_all(&buffer);
                    } else {
                        let body = "Not Found";
                        let response = format!(
                            "HTTP/1.1 404 Not Found\r\n\
                             Content-Type: text/plain\r\n\
                             Content-Length: {}\r\n\
                             \r\n\
                             {body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                }
                Err(e) => {
                    error!("metrics server accept error: {e}");
                }
            }
        }

        info!("metrics server stopped");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::io::BufReader;
    use std::net::TcpStream;

    #[test]
    fn metrics_endpoint_returns_prometheus_format() {
        let metrics = DaemonMetrics::new();
        let registry = Arc::new(metrics.registry.clone());
        let port = 19401; // Use a non-standard port for tests.
        let _handle = start_metrics_server(registry, port);

        // Give the server a moment to start.
        std::thread::sleep(std::time::Duration::from_millis(100));

        let stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        let mut stream_clone = stream.try_clone().unwrap();
        let _ = std::io::Write::write_all(
            &mut stream_clone,
            b"GET /metrics HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );

        let reader = BufReader::new(stream);
        let mut lines: Vec<String> = Vec::new();
        for line in reader.lines() {
            match line {
                Ok(l) => lines.push(l),
                Err(_) => break,
            }
        }

        // Should get a 200 OK response.
        assert!(
            lines.first().map_or(false, |l| l.contains("200 OK")),
            "expected 200 OK, got: {:?}",
            lines.first()
        );

        // Body should contain our metric names.
        let body = lines.join("\n");
        assert!(
            body.contains("fabric_daemon_messages_total"),
            "response should contain messages_total metric"
        );
        assert!(
            body.contains("fabric_daemon_connections_active"),
            "response should contain connections_active metric"
        );
        assert!(
            body.contains("fabric_daemon_uptime_seconds"),
            "response should contain uptime_seconds metric"
        );
    }

    #[test]
    fn counter_increments() {
        let metrics = DaemonMetrics::new();
        metrics
            .messages_total
            .with_label_values(&["heartbeat"])
            .inc();
        metrics
            .messages_total
            .with_label_values(&["heartbeat"])
            .inc();
        assert_eq!(
            metrics.messages_total.with_label_values(&["heartbeat"]).get(),
            2
        );
    }

    #[test]
    fn histogram_records() {
        let metrics = DaemonMetrics::new();
        metrics
            .message_duration_seconds
            .with_label_values(&["health_check"])
            .observe(0.05);
        metrics
            .message_duration_seconds
            .with_label_values(&["health_check"])
            .observe(0.15);

        let hist = metrics
            .message_duration_seconds
            .with_label_values(&["health_check"]);
        assert_eq!(hist.get().sample_count(), 2);
        assert!(hist.get().sample_sum > 0.19);
    }

    #[test]
    fn gauge_tracks_connections() {
        let metrics = DaemonMetrics::new();
        metrics.connections_active.inc();
        metrics.connections_active.inc();
        assert_eq!(metrics.connections_active.get() as u64, 2);
        metrics.connections_active.dec();
        assert_eq!(metrics.connections_active.get() as u64, 1);
    }

    #[test]
    fn error_counter_increments() {
        let metrics = DaemonMetrics::new();
        metrics.errors_total.with_label_values(&["parse"]).inc();
        metrics
            .errors_total
            .with_label_values(&["timeout"])
            .inc();
        assert_eq!(
            metrics.errors_total.with_label_values(&["parse"]).get(),
            1
        );
        assert_eq!(
            metrics.errors_total.with_label_values(&["timeout"]).get(),
            1
        );
    }

    #[test]
    fn metrics_not_found_for_non_metrics_path() {
        let metrics = DaemonMetrics::new();
        let registry = Arc::new(metrics.registry.clone());
        let port = 19402;
        let _handle = start_metrics_server(registry, port);

        std::thread::sleep(std::time::Duration::from_millis(100));

        let stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        let mut stream_clone = stream.try_clone().unwrap();
        let _ = std::io::Write::write_all(
            &mut stream_clone,
            b"GET /other HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );

        let reader = BufReader::new(stream);
        let mut lines: Vec<String> = Vec::new();
        for line in reader.lines() {
            match line {
                Ok(l) => lines.push(l),
                Err(_) => break,
            }
        }

        assert!(
            lines.first().map_or(false, |l| l.contains("404")),
            "expected 404 for non-metrics path"
        );
    }
}
