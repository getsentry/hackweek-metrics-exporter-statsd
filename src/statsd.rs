use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use metrics_util::MetricKind;

use crate::recorder::PlainRecorder;

#[derive(Debug)]
pub struct StatsdExporter {
    local_socket: UdpSocket,
    peer_addr: SocketAddr,
    interval: Duration,
    recorder: Arc<PlainRecorder>,
}

impl StatsdExporter {
    pub(crate) fn new(
        local_socket: UdpSocket,
        peer_addr: SocketAddr,
        interval: Duration,
        recorder: Arc<PlainRecorder>,
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
