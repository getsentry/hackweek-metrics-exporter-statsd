use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use metrics_util::MetricKind;

use crate::recorder::PlainRecorder;

#[derive(Debug)]
pub struct StatsdExporter {
    local_socket: UdpSocket,
    peer_addr: SocketAddr,
    interval: Duration,
    recorder: PlainRecorder,
}

impl StatsdExporter {
    pub(crate) fn new(
        local_socket: UdpSocket,
        peer_addr: SocketAddr,
        interval: Duration,
        recorder: PlainRecorder,
    ) -> Self {
        Self {
            local_socket,
            peer_addr,
            interval,
            recorder,
        }
    }

    fn export(&self) -> Bytes {
        // TODO: re-use the allocated buffer
        // TODO: chunk this into 512 byte buffers
        // TODO: re-set the data sent, only send updates
        let mut buf = BytesMut::with_capacity(512);
        for (desc, handle) in self.recorder.registry.get_handles() {
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
    use std::net::Ipv6Addr;

    use metrics::{Key, Recorder};

    /// Return exporter and the socket to which it sends data.
    fn statsd_exporter() -> (StatsdExporter, UdpSocket) {
        let recorder = PlainRecorder::new();
        let recv_socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0)).unwrap();
        let send_socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0)).unwrap();
        let exporter = StatsdExporter::new(
            send_socket,
            recv_socket.local_addr().unwrap(),
            Duration::from_secs(1),
            recorder,
        );
        (exporter, recv_socket)
    }

    #[test]
    fn test_export_counter() {
        let (exporter, recv_socket) = statsd_exporter();

        let c0 = exporter
            .recorder
            .register_counter(Key::from_name("spam"), None);
        let out = exporter.export();
        assert_eq!(out.bytes(), b"spam:0|c\n");

        exporter.recorder.increment_counter(c0, 1);
        let out = exporter.export();
        assert_eq!(out.bytes(), b"spam:1|c\n");
    }

    #[test]
    fn test_export_gauge() {
        let (exporter, recv_socket) = statsd_exporter();

        let g0 = exporter
            .recorder
            .register_gauge(Key::from_name("spam"), None);
        let out = exporter.export();
        assert_eq!(out.bytes(), b"spam:0|g\n");

        exporter.recorder.update_gauge(g0, 42.0);
        let out = exporter.export();
        assert_eq!(out.bytes(), b"spam:42|g\n");

        exporter.recorder.update_gauge(g0, 3.3);
        let out = exporter.export();
        assert_eq!(out.bytes(), b"spam:3.3|g\n");
    }

    #[test]
    fn test_send() {
        let (exporter, recv_socket) = statsd_exporter();

        let c0 = exporter
            .recorder
            .register_counter(Key::from_name("spam"), None);
        let c1 = exporter
            .recorder
            .register_counter(Key::from_name("ham"), None);
        let g0 = exporter
            .recorder
            .register_gauge(Key::from_name("eggs"), None);
        exporter.recorder.increment_counter(c0, 3);
        exporter.recorder.increment_counter(c1, 7);
        exporter.recorder.update_gauge(g0, 11.0);
        exporter.send().unwrap();

        let mut buf = BytesMut::with_capacity(512);
        unsafe {
            // Pretend this memory is initialised, we're only writing to it.
            let slice: &mut [u8] = &mut *(buf.bytes_mut() as *mut [MaybeUninit<u8>] as *mut [u8]);
            let count = recv_socket.recv(slice).unwrap();
            buf.advance_mut(count);
        }
        let data = std::str::from_utf8(buf.bytes()).unwrap();
        let mut lines: Vec<String> = data.lines().map(String::from).collect();
        lines.sort();
        let expected = ["eggs:11|g", "ham:7|c", "spam:3|c"];
        assert_eq!(lines, expected);
    }
}
