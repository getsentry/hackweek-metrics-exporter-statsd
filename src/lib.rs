//! A [`metrics`][metrics]-compatible exporter that outputs metrics using statsd.

use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::thread::{self, JoinHandle};
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

    pub fn statsd(&mut self, enabled: bool) -> &mut Self {
        self.statsd = enabled;
        self
    }

    pub fn local_addr(&mut self, addr: SocketAddr) -> &mut Self {
        self.local_addr = addr;
        self
    }

    pub fn statsd_addr(&mut self, addr: SocketAddr) -> &mut Self {
        self.peer_addr = addr;
        self
    }

    pub fn interval(&mut self, duration: Duration) -> &mut Self {
        self.interval = duration;
        self
    }

    fn create_exporter(&self, recorder: PlainRecorder) -> Result<StatsdExporter, io::Error> {
        Ok(StatsdExporter::new(
            UdpSocket::bind(self.local_addr)?,
            self.peer_addr,
            self.interval,
            recorder,
        ))
    }

    pub fn install(&self) -> Result<MetricsCollector, InstallError> {
        let recorder = PlainRecorder::new();
        let exporter = self
            .create_exporter(recorder.clone())
            .map_err(InstallError::Build)?;
        metrics::set_boxed_recorder(Box::new(recorder.clone())).map_err(InstallError::Install)?;
        let handle = if self.statsd {
            let handle = thread::spawn(move || match exporter.run() {
                Ok(()) => (),
                Err(e) => {
                    log::error!("Statsd exporter failed: {}", e);
                }
            });
            Some(handle)
        } else {
            None
        };
        Ok(MetricsCollector {
            recorder,
            statsd_handle: handle,
        })
    }
}

impl Default for MetricsBuilder {
    fn default() -> Self {
        MetricsBuilder::new()
    }
}

#[derive(Debug)]
pub struct MetricsCollector {
    recorder: PlainRecorder,
    statsd_handle: Option<JoinHandle<()>>,
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
}
