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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let rec = PlainRecorder::new();

        let c0 = rec.register_counter(Key::from_name("spam.ham"), None);
        let c1 = rec.register_counter(Key::from_name("spam.eggs"), None);
        let c2 = rec.register_counter(Key::from_name("spam.ham"), None);
        assert_eq!(c0, c2);
        assert_ne!(c0, c1);

        rec.increment_counter(c0, 1);
        rec.registry.with_handle(c0, |handle| {
            assert_eq!(handle.read_counter(), 1);
        });
        rec.increment_counter(c0, 2);
        rec.registry.with_handle(c0, |handle| {
            assert_eq!(handle.read_counter(), 3);
        });
    }

    #[test]
    fn test_register_gauge() {
        let rec = PlainRecorder::new();

        let g0 = rec.register_gauge(Key::from_name("spam.ham"), None);
        let g1 = rec.register_gauge(Key::from_name("spam.eggs"), None);
        let g2 = rec.register_gauge(Key::from_name("spam.ham"), None);
        assert_eq!(g0, g2);
        assert_ne!(g0, g1);

        rec.update_gauge(g0, 7.0);
        rec.registry.with_handle(g0, |handle| {
            assert_eq!(handle.read_gauge(), 7.0);
        });
        rec.update_gauge(g0, 3.0);
        rec.registry.with_handle(g0, |handle| {
            assert_eq!(handle.read_gauge(), 3.0);
        });
    }
}
