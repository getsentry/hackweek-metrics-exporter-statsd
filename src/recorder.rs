//! Plain recorder.

use std::fmt;
use std::sync::Arc;

use metrics::{Identifier, Key, Recorder};
use metrics_util::{CompositeKey, Handle, MetricKind, Registry};

/// A simple recorder doing nothing fancy but record the plain values.
///
/// Cloning this is cheap since the clones will refer to the same metrics storage.
#[derive(Clone)]
pub(crate) struct PlainRecorder {
    pub(crate) registry: Arc<Registry<CompositeKey, Handle>>,
}

impl fmt::Debug for PlainRecorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlainRecorder").finish()
    }
}

impl PlainRecorder {
    pub(crate) fn new() -> Self {
        Self {
            registry: Arc::new(Registry::new()),
        }
    }
}

impl Recorder for PlainRecorder {
    fn register_counter(&self, key: Key, _description: Option<&'static str>) -> Identifier {
        self.registry
            .get_or_create_identifier(CompositeKey::new(MetricKind::Counter, key), |_key| {
                Handle::counter()
            })
    }

    fn register_gauge(&self, key: Key, _description: Option<&'static str>) -> Identifier {
        self.registry
            .get_or_create_identifier(CompositeKey::new(MetricKind::Gauge, key), |_key| {
                Handle::gauge()
            })
    }

    fn register_histogram(&self, key: Key, _description: Option<&'static str>) -> Identifier {
        self.registry
            .get_or_create_identifier(CompositeKey::new(MetricKind::Histogram, key), |_key| {
                Handle::histogram()
            })
    }

    fn increment_counter(&self, id: Identifier, value: u64) {
        self.registry
            .with_handle(id, move |handle| handle.increment_counter(value));
    }

    fn update_gauge(&self, id: Identifier, value: f64) {
        self.registry
            .with_handle(id, move |handle| handle.update_gauge(value));
    }

    fn record_histogram(&self, id: Identifier, value: u64) {
        self.registry
            .with_handle(id, move |handle| handle.record_histogram(value));
    }
}
