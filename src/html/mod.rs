//! A simple HTML Frontend for metrics
//!
//! This

use std::sync::Arc;

use metrics_util::MetricKind;
use serde_json::{json, Map, Value};

use crate::recorder::PlainRecorder;

/// The static index page that needs to be served as `index.html`.
pub static INDEX: &str = include_str!("index.html");

/// The static index page that needs to be served as `graph.js`.
pub static JS: &str = include_str!("graph.js");

/// Snapshots the current state of the `registry` as JSON.
///
/// This can be served as a `data.json` file.
pub fn metrics_json(recorder: Arc<PlainRecorder>) -> Value {
    let metrics: Map<String, Value> = recorder
        .registry
        .get_handles()
        .into_iter()
        .map(|(desc, handle)| {
            (
                desc.key().name().to_string(),
                match desc.kind() {
                    MetricKind::Counter => handle.read_counter().into(),
                    MetricKind::Gauge => handle.read_gauge().into(),
                    MetricKind::Histogram => handle.read_histogram().into(),
                },
            )
        })
        .collect();
    json!({ "metrics": metrics })
}
