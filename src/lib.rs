//! A [`metrics`][metrics]-compatible exporter that outputs metrics using statsd.

use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::Duration;

use metrics::{self, SetRecorderError};

mod html;
mod recorder;
mod statsd;

use crate::recorder::PlainRecorder;
use crate::statsd::StatsdExporter;

pub use html::HtmlExporter;

#[derive(Debug, Clone)]
pub struct MetricsBuilder {
    statsd: bool,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    interval: Duration,
}

#[derive(Debug)]
pub enum InstallError {
    Build(io::Error),
    Install(SetRecorderError),
}

impl MetricsBuilder {
    pub fn new() -> Self {
        Self {
            statsd: true,
            local_addr: SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
            peer_addr: SocketAddr::from((Ipv6Addr::LOCALHOST, 8125)),
            interval: Duration::from_secs(5),
        }
    }

    pub fn statsd<'a>(&'a mut self, enabled: bool) -> &'a mut Self {
        self.statsd = enabled;
        self
    }

    pub fn local_addr<'a>(&'a mut self, addr: SocketAddr) -> &'a mut Self {
        self.local_addr = addr;
        self
    }

    pub fn statsd_addr<'a>(&'a mut self, addr: SocketAddr) -> &'a mut Self {
        self.peer_addr = addr;
        self
    }

    pub fn interval<'a>(&'a mut self, duration: Duration) -> &'a mut Self {
        self.interval = duration;
        self
    }

    pub fn build(&self) -> Result<MetricsCollector, io::Error> {
        let recorder = PlainRecorder::new();
        let statsd_exporter = if self.statsd {
            Some(StatsdExporter::new(
                UdpSocket::bind(self.local_addr)?,
                self.peer_addr,
                self.interval,
                recorder.clone(),
            ))
        } else {
            None
        };
        Ok(MetricsCollector {
            recorder,
            statsd_exporter,
        })
    }

    pub fn install(&self) -> Result<MetricsCollector, InstallError> {
        let collector = self.build().map_err(|e| InstallError::Build(e))?;
        metrics::set_boxed_recorder(Box::new(collector.recorder()))
            .map_err(|e| InstallError::Install(e))?;
        Ok(collector)
    }
}

#[derive(Debug)]
pub struct MetricsCollector {
    recorder: PlainRecorder,
    statsd_exporter: Option<StatsdExporter>,
}

impl MetricsCollector {
    /// Return an HtmlExporter which can be used to show the collected metrics.
    pub fn html(&self) -> HtmlExporter {
        HtmlExporter::new(self.recorder.clone())
    }

    /// Return the underlying recorder instance.
    ///
    /// This can be used to directly invoke `Recorder::register_counter()` etc functions.
    pub fn recorder(&self) -> impl metrics::Recorder {
        self.recorder.clone()
    }

    #[cfg(test)]
    pub(crate) fn with_handle<F, V>(&self, identifier: metrics::Identifier, f: F) -> Option<V>
    where
        F: FnOnce(&metrics_util::Handle) -> V,
    {
        self.recorder.registry.with_handle(identifier, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use metrics::{Key, Recorder};

    #[test]
    fn test_record_counter() {
        let collector = MetricsBuilder::new().statsd(false).build().unwrap();
        let recorder = collector.recorder();
        let c0 = recorder.register_counter(Key::from_name("spam"), None);
        recorder.increment_counter(c0, 1);
        collector.with_handle(c0, |handle| {
            assert_eq!(handle.read_counter(), 1);
        });
    }
}
