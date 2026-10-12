//! A std-only health probe for container healthchecks (`arda-server --probe`).
//!
//! The runtime image has no curl, so the server binary can ask itself: it
//! opens a loopback connection, sends `GET /v1/health` and reports whether
//! the answer was `200`. No new dependency, no async runtime.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Why a probe failed.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// The connection or the exchange failed.
    #[error("probing {addr}: {source}")]
    Io {
        /// Address probed.
        addr: SocketAddr,
        /// Underlying error.
        source: std::io::Error,
    },
    /// The server answered something other than `200`.
    #[error("probing {addr}: unhealthy status line {line:?}")]
    Status {
        /// Address probed.
        addr: SocketAddr,
        /// First response line.
        line: String,
    },
}

/// Sends `GET /v1/health` to `addr` and succeeds on a `200` status line.
///
/// # Errors
/// [`ProbeError::Io`] when the server is unreachable or silent past
/// `timeout`; [`ProbeError::Status`] on any non-200 answer.
pub fn probe(addr: SocketAddr, timeout: Duration) -> Result<(), ProbeError> {
    let io = |source| ProbeError::Io { addr, source };
    let mut stream = TcpStream::connect_timeout(&addr, timeout).map_err(io)?;
    stream.set_read_timeout(Some(timeout)).map_err(io)?;
    stream.set_write_timeout(Some(timeout)).map_err(io)?;
    stream
        .write_all(b"GET /v1/health HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .map_err(io)?;
    let mut head = [0u8; 64];
    let mut len = 0;
    while len < head.len() {
        let n = stream.read(&mut head[len..]).map_err(io)?;
        if n == 0 {
            break;
        }
        len += n;
        if head[..len].contains(&b'\n') {
            break;
        }
    }
    let text = String::from_utf8_lossy(&head[..len]);
    let line = text.lines().next().unwrap_or("").trim_end().to_owned();
    let mut parts = line.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some(v), Some("200")) if v.starts_with("HTTP/") => Ok(()),
        _ => Err(ProbeError::Status { addr, line }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    const T: Duration = Duration::from_secs(2);

    fn serve_once(reply: &'static [u8]) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 256];
                let n = s.read(&mut buf).unwrap_or(0);
                assert!(buf[..n].starts_with(b"GET /v1/health HTTP/1.0"));
                let _ = s.write_all(reply);
            }
        });
        addr
    }

    #[test]
    fn a_200_is_healthy() {
        let addr = serve_once(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok");
        assert!(probe(addr, T).is_ok());
    }

    #[test]
    fn a_non_200_is_unhealthy() {
        let addr = serve_once(b"HTTP/1.1 503 Service Unavailable\r\n\r\n");
        assert!(matches!(probe(addr, T), Err(ProbeError::Status { .. })));
    }

    #[test]
    fn a_refused_connection_is_an_io_error() {
        let addr = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap()
        };
        assert!(matches!(probe(addr, T), Err(ProbeError::Io { .. })));
    }

    #[test]
    fn a_silent_close_is_unhealthy() {
        let addr = serve_once(b"");
        assert!(probe(addr, T).is_err());
    }
}
