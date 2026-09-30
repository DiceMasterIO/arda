//! The HTTP/1 accept loop, with the connection limits `axum::serve` lacks.
//!
//! `axum::serve` sets no hyper timer, so hyper's header read timeout never
//! fires and a client that trickles its headers holds a connection forever
//! (slow-loris). This loop arms the timeout and caps concurrent connections;
//! at the cap it stops accepting until a connection closes.

use axum::Router;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

/// Connection limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServeLimits {
    /// Longest wait for a complete request head, including between
    /// keep-alive requests.
    pub header_read_timeout: Duration,
    /// Most connections served at once.
    pub max_connections: usize,
}

impl Default for ServeLimits {
    fn default() -> Self {
        Self {
            header_read_timeout: Duration::from_secs(10),
            max_connections: 64,
        }
    }
}

/// Serves `app` on `listener` until `shutdown` resolves, then stops accepting.
///
/// # Errors
/// Listener accept failures.
pub async fn serve(
    listener: TcpListener,
    app: Router,
    limits: ServeLimits,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let permits = Arc::new(Semaphore::new(limits.max_connections.max(1)));
    let mut shutdown = std::pin::pin!(shutdown);
    loop {
        let permit = tokio::select! {
            () = &mut shutdown => return Ok(()),
            permit = Arc::clone(&permits).acquire_owned() => permit,
        };
        let Ok(permit) = permit else {
            return Ok(());
        };
        let (stream, _) = tokio::select! {
            () = &mut shutdown => return Ok(()),
            accepted = listener.accept() => accepted?,
        };
        let service = TowerToHyperService::new(app.clone());
        let mut builder = http1::Builder::new();
        builder
            .timer(TokioTimer::new())
            .header_read_timeout(limits.header_read_timeout);
        tokio::spawn(async move {
            // A failed or timed-out connection only ends that connection.
            let _ = builder
                .serve_connection(TokioIo::new(stream), service)
                .await;
            drop(permit);
        });
    }
}
