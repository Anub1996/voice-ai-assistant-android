//! Process-wide Prometheus metrics (text exposition format 0.0.4).
//!
//! Hand-rolled to stay dependency-free; label values are bounded (matched
//! route templates, runtime names from config), so cardinality is fixed.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

const BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0,
];

#[derive(Default)]
struct Histogram {
    counts: Vec<u64>,
    sum: f64,
    count: u64,
}

#[derive(Default)]
pub struct Metrics {
    requests: Mutex<BTreeMap<(String, String, u16), u64>>,
    latency: Mutex<BTreeMap<String, Histogram>>,
    runtime_events: Mutex<BTreeMap<(String, &'static str), u64>>,
    pub inflight: AtomicI64,
    pub rejected: AtomicU64,
    pub memories_recalled: AtomicU64,
    pub memories_stored: AtomicU64,
    pub auth_failures: AtomicU64,
}

pub static METRICS: LazyLock<Metrics> = LazyLock::new(Metrics::default);

impl Metrics {
    pub fn observe_request(&self, method: &str, route: &str, status: u16, secs: f64) {
        *self
            .requests
            .lock()
            .unwrap()
            .entry((method.to_string(), route.to_string(), status))
            .or_default() += 1;
        let mut lat = self.latency.lock().unwrap();
        let h = lat.entry(route.to_string()).or_insert_with(|| Histogram {
            counts: vec![0; BUCKETS.len()],
            ..Default::default()
        });
        for (i, b) in BUCKETS.iter().enumerate() {
            if secs <= *b {
                h.counts[i] += 1;
            }
        }
        h.sum += secs;
        h.count += 1;
    }

    /// `event` is one of `start`, `start_failure`, `eviction`, `idle_stop`.
    pub fn runtime_event(&self, runtime: &str, event: &'static str) {
        *self
            .runtime_events
            .lock()
            .unwrap()
            .entry((runtime.to_string(), event))
            .or_default() += 1;
    }

    /// Render all metrics. `extra` carries gauges computed at scrape time.
    pub fn render(&self, extra: &str) -> String {
        let mut o = String::new();
        let _ = writeln!(
            o,
            "# HELP llmo_build_info Build information.\n# TYPE llmo_build_info gauge"
        );
        let _ = writeln!(
            o,
            "llmo_build_info{{version=\"{}\"}} 1",
            env!("CARGO_PKG_VERSION")
        );

        let _ = writeln!(
            o,
            "# HELP llmo_http_requests_total HTTP requests by route and status."
        );
        let _ = writeln!(o, "# TYPE llmo_http_requests_total counter");
        for ((m, r, s), n) in self.requests.lock().unwrap().iter() {
            let _ = writeln!(
                o,
                "llmo_http_requests_total{{method=\"{m}\",route=\"{}\",status=\"{s}\"}} {n}",
                esc(r)
            );
        }

        let _ = writeln!(
            o,
            "# HELP llmo_http_request_duration_seconds Request latency (time to response headers)."
        );
        let _ = writeln!(o, "# TYPE llmo_http_request_duration_seconds histogram");
        for (r, h) in self.latency.lock().unwrap().iter() {
            let r = esc(r);
            for (i, b) in BUCKETS.iter().enumerate() {
                let _ = writeln!(
                    o,
                    "llmo_http_request_duration_seconds_bucket{{route=\"{r}\",le=\"{b}\"}} {}",
                    h.counts[i]
                );
            }
            let _ = writeln!(
                o,
                "llmo_http_request_duration_seconds_bucket{{route=\"{r}\",le=\"+Inf\"}} {}",
                h.count
            );
            let _ = writeln!(
                o,
                "llmo_http_request_duration_seconds_sum{{route=\"{r}\"}} {}",
                h.sum
            );
            let _ = writeln!(
                o,
                "llmo_http_request_duration_seconds_count{{route=\"{r}\"}} {}",
                h.count
            );
        }

        let gauge = |o: &mut String, name: &str, help: &str, kind: &str, v: String| {
            let _ = writeln!(o, "# HELP {name} {help}\n# TYPE {name} {kind}\n{name} {v}");
        };
        gauge(
            &mut o,
            "llmo_http_inflight_requests",
            "Requests being processed.",
            "gauge",
            self.inflight.load(Ordering::Relaxed).to_string(),
        );
        gauge(
            &mut o,
            "llmo_http_rejected_total",
            "Requests shed with 503 because the server was at capacity.",
            "counter",
            self.rejected.load(Ordering::Relaxed).to_string(),
        );
        gauge(
            &mut o,
            "llmo_auth_failures_total",
            "Requests rejected for a missing or invalid API key.",
            "counter",
            self.auth_failures.load(Ordering::Relaxed).to_string(),
        );
        gauge(
            &mut o,
            "llmo_memories_recalled_total",
            "Memories injected into chat requests.",
            "counter",
            self.memories_recalled.load(Ordering::Relaxed).to_string(),
        );
        gauge(
            &mut o,
            "llmo_memories_stored_total",
            "Memories written.",
            "counter",
            self.memories_stored.load(Ordering::Relaxed).to_string(),
        );

        let _ = writeln!(
            o,
            "# HELP llmo_runtime_events_total Runtime lifecycle events."
        );
        let _ = writeln!(o, "# TYPE llmo_runtime_events_total counter");
        for ((rt, ev), n) in self.runtime_events.lock().unwrap().iter() {
            let _ = writeln!(
                o,
                "llmo_runtime_events_total{{runtime=\"{}\",event=\"{ev}\"}} {n}",
                esc(rt)
            );
        }
        o.push_str(extra);
        o
    }
}

pub fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_prometheus_text() {
        let m = Metrics::default();
        m.observe_request("POST", "/v1/chat/completions", 200, 0.3);
        m.observe_request("POST", "/v1/chat/completions", 200, 7.0);
        m.runtime_event("qwen", "eviction");
        let t = m.render("");
        assert!(t.contains("llmo_http_requests_total{method=\"POST\",route=\"/v1/chat/completions\",status=\"200\"} 2"));
        assert!(t.contains(
            "llmo_http_request_duration_seconds_bucket{route=\"/v1/chat/completions\",le=\"0.5\"} 1"
        ));
        assert!(t.contains(
            "llmo_http_request_duration_seconds_bucket{route=\"/v1/chat/completions\",le=\"10\"} 2"
        ));
        assert!(t.contains("llmo_runtime_events_total{runtime=\"qwen\",event=\"eviction\"} 1"));
        assert!(t.contains("llmo_build_info{version="));
    }

    #[test]
    fn escapes_labels() {
        assert_eq!(esc("a\"b\\c"), "a\\\"b\\\\c");
    }
}
