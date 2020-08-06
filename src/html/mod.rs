//! A simple HTML Frontend for metrics

use metrics_util::MetricKind;
use serde_json::{json, Map, Value};

use crate::recorder::PlainRecorder;

#[derive(Debug, Clone)]
pub struct HtmlExporter {
    recorder: PlainRecorder,
}

impl HtmlExporter {
    /// The static index page that needs to be served as `index.html`.
    pub const INDEX: &'static str = include_str!("index.html");

    /// The static index page that needs to be served as `graph.js`.
    pub const JS: &'static str = include_str!("graph.js");

    pub(crate) fn new(recorder: PlainRecorder) -> Self {
        Self { recorder }
    }

    /// Snapshots the current state of the `registry` as JSON.
    ///
    /// This can be served as a `data.json` file.
    pub fn json_snapshot(&self) -> Value {
        let metrics: Map<String, Value> = self
            .recorder
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
}
