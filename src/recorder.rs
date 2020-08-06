//! Plain recorder.

use metrics::{Identifier, Key, Recorder};
use metrics_util::{CompositeKey, Handle, MetricKind, Registry};

pub(crate) struct PlainRecorder {
    pub(crate) registry: Registry<CompositeKey, Handle>,
}

impl PlainRecorder {
    pub(crate) fn new() -> Self {
        Self {
            registry: Registry::new(),
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
