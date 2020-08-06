//! A [`metrics`][metrics]-compatible exporter that outputs metrics using statsd.

use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

mod html;
mod recorder;
mod statsd;

use crate::recorder::PlainRecorder;
use crate::statsd::StatsdExporter;
use serde_json::Value;

pub struct MetricsBuilder {
    statsd: bool,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    interval: Duration,
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
        let recorder = Arc::new(PlainRecorder::new());
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
}

pub struct MetricsCollector {
    recorder: Arc<PlainRecorder>,
    statsd_exporter: Option<StatsdExporter>,
}

impl MetricsCollector {
    pub fn json_snapshot(&self) -> Value {
        html::metrics_json(&*self.recorder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::mem::MaybeUninit;
    use std::net::Ipv4Addr;

    use metrics::Key;

    #[test]
    fn test_builder_plain() {
        let builder = StatsdBuilder::new();
        let rec = builder.build().unwrap();
        assert_eq!(
            rec.local_socket.local_addr().unwrap().ip(),
            builder.local_addr.ip()
        );
        assert_eq!(rec.peer_addr, builder.peer_addr);
        assert_eq!(rec.interval, Duration::from_secs(5));
    }

    #[test]
    fn test_builder_local_addr() {
        let mut builder = StatsdBuilder::new();
        assert!(builder.local_addr.is_ipv6());
        builder.local_addr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)));
        assert!(builder.local_addr.is_ipv4());

        let rec = builder.build().unwrap();
        assert!(rec.local_socket.local_addr().unwrap().is_ipv4());
    }

    #[test]
    fn test_couter() {
        let builder = StatsdBuilder::new();
        let rec = builder.build().unwrap();

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
        let builder = StatsdBuilder::new();
        let rec = builder.build().unwrap();

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

    #[test]
    fn test_export_counter() {
        let builder = StatsdBuilder::new();
        let rec = builder.build().unwrap();

        let c0 = rec.register_counter(Key::from_name("spam"), None);
        let out = rec.export();
        assert_eq!(out.bytes(), b"spam:0|c\n");

        rec.increment_counter(c0, 1);
        let out = rec.export();
        assert_eq!(out.bytes(), b"spam:1|c\n");
    }

    #[test]
    fn test_export_gauge() {
        let builder = StatsdBuilder::new();
        let rec = builder.build().unwrap();

        let g0 = rec.register_gauge(Key::from_name("spam"), None);
        let out = rec.export();
        assert_eq!(out.bytes(), b"spam:0|g\n");

        rec.update_gauge(g0, 42.0);
        let out = rec.export();
        assert_eq!(out.bytes(), b"spam:42|g\n");

        rec.update_gauge(g0, 3.3);
        let out = rec.export();
        assert_eq!(out.bytes(), b"spam:3.3|g\n");
    }

    #[test]
    fn test_send() {
        let recv_socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0)).unwrap();

        let mut builder = StatsdBuilder::new();
        builder.statsd_addr(recv_socket.local_addr().unwrap());
        let rec = builder.build().unwrap();

        let c0 = rec.register_counter(Key::from_name("spam"), None);
        let c1 = rec.register_counter(Key::from_name("ham"), None);
        let g0 = rec.register_gauge(Key::from_name("eggs"), None);
        rec.increment_counter(c0, 3);
        rec.increment_counter(c1, 7);
        rec.update_gauge(g0, 11.0);
        rec.send().unwrap();

        let mut buf = BytesMut::with_capacity(512);
        unsafe {
            // What are we even doing?  (See AsyncRead::poll_read_buf())
            let uninit_slice = buf.bytes_mut();
            prepare_uninitialized_buffer(uninit_slice);
            let slice: &mut [u8] = &mut *(uninit_slice as *mut [MaybeUninit<u8>] as *mut [u8]);

            let count = recv_socket.recv(slice).unwrap();
            buf.advance_mut(count);
        }
        let data = std::str::from_utf8(buf.bytes()).unwrap();
        let mut lines: Vec<String> = data.lines().map(String::from).collect();
        lines.sort();
        let expected = ["eggs:11|g", "ham:7|c", "spam:3|c"];
        assert_eq!(lines, expected);
    }

    fn prepare_uninitialized_buffer(buf: &mut [MaybeUninit<u8>]) {
        for x in buf {
            *x = MaybeUninit::new(0);
        }
    }
}
