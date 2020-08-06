//! A [`metrics`][metrics]-compatible exporter that outputs metrics using statsd.

use std::io;
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use metrics::{Identifier, Key, Recorder};
use metrics_util::{CompositeKey, Handle, MetricKind, Registry};

#[derive(Debug)]
pub enum Error {
    Dummy,
}

pub struct StatsdBuilder {
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    interval: Duration,
}

impl StatsdBuilder {
    pub fn new() -> Self {
        Self {
            local_addr: SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
            peer_addr: SocketAddr::from((Ipv6Addr::LOCALHOST, 8125)),
            interval: Duration::from_secs(5),
        }
    }

    pub fn create_registry() -> Registry<CompositeKey, Handle> {
        Registry::new()
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

    pub fn build(&self) -> Result<StatsdRecorder, io::Error> {
        Ok(StatsdRecorder {
            local_socket: UdpSocket::bind(self.local_addr)?,
            peer_addr: self.peer_addr,
            interval: self.interval,
            registry: Arc::new(Self::create_registry()),
        })
    }

    pub fn build_with_registry(
        &self,
        registry: Arc<Registry<CompositeKey, Handle>>,
    ) -> Result<StatsdRecorder, io::Error> {
        Ok(StatsdRecorder {
            local_socket: UdpSocket::bind(self.local_addr)?,
            peer_addr: self.peer_addr,
            interval: self.interval,
            registry,
        })
    }
}

pub struct StatsdRecorder {
    local_socket: UdpSocket,
    peer_addr: SocketAddr,
    interval: Duration,
    registry: Arc<Registry<CompositeKey, Handle>>,
}

impl Recorder for StatsdRecorder {
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

impl StatsdRecorder {
    fn export(&self) -> Bytes {
        // TODO: re-use the allocated buffer
        // TODO: chunk this into 512 byte buffers
        // TODO: re-set the data sent, only send updates
        let mut buf = BytesMut::with_capacity(512);
        for (desc, handle) in self.registry.get_handles() {
            match desc.kind() {
                MetricKind::Counter | MetricKind::Gauge => (),
                _ => continue,
            }
            let metric_name = desc.key().name();
            buf.put_slice(metric_name.as_bytes());
            buf.put_slice(":".as_bytes());
            match desc.kind() {
                MetricKind::Counter => {
                    buf.put_slice(format!("{}|c", handle.read_counter()).as_bytes());
                }
                MetricKind::Gauge => {
                    buf.put_slice(format!("{}|g", handle.read_gauge()).as_bytes());
                }
                _ => continue,
            }
            buf.put_slice("\n".as_bytes());
        }
        buf.freeze()
    }

    fn send(&self) -> io::Result<()> {
        let mut data = self.export();
        while data.has_remaining() {
            let count = self.local_socket.send_to(data.bytes(), &self.peer_addr)?;
            data.advance(count);
        }
        Ok(())
    }

    pub fn run(self) {
        loop {
            std::thread::sleep(self.interval);
            self.send().unwrap();
        }
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
