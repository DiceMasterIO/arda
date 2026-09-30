//! The accept loop's defences against slow or hoarding clients.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_server::serve::{serve, ServeLimits};
use axum::routing::get;
use axum::Router;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn start(limits: ServeLimits) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = Router::new().route("/ok", get(|| async { "ok" }));
    tokio::spawn(serve(listener, app, limits, std::future::pending()));
    addr
}

const LIMITS: ServeLimits = ServeLimits {
    header_read_timeout: Duration::from_millis(300),
    max_connections: 4,
};

#[tokio::test]
async fn a_client_that_never_finishes_its_headers_is_disconnected() {
    let addr = start(LIMITS).await;
    let mut slow = TcpStream::connect(addr).await.unwrap();
    slow.write_all(b"GET /ok HTTP/1.1\r\nHost: x\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    let read = tokio::time::timeout(Duration::from_secs(5), slow.read_to_end(&mut buf)).await;
    assert!(
        read.is_ok(),
        "a slow-loris connection was still open after 5 s"
    );
}

#[tokio::test]
async fn complete_requests_are_still_served() {
    let addr = start(LIMITS).await;
    let mut client = TcpStream::connect(addr).await.unwrap();
    client
        .write_all(b"GET /ok HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), client.read_to_end(&mut buf))
        .await
        .unwrap()
        .unwrap();
    let text = String::from_utf8_lossy(&buf);
    assert!(text.starts_with("HTTP/1.1 200"), "{text}");
    assert!(text.ends_with("ok"), "{text}");
}

#[tokio::test]
async fn connections_beyond_the_cap_wait_for_a_free_slot() {
    let addr = start(ServeLimits {
        header_read_timeout: Duration::from_secs(30),
        max_connections: 1,
    })
    .await;
    let hog = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mut client = TcpStream::connect(addr).await.unwrap();
    client
        .write_all(b"GET /ok HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    let early =
        tokio::time::timeout(Duration::from_millis(500), client.read_to_end(&mut buf)).await;
    assert!(early.is_err(), "served past the connection cap");
    drop(hog);
    tokio::time::timeout(Duration::from_secs(5), client.read_to_end(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert!(String::from_utf8_lossy(&buf).starts_with("HTTP/1.1 200"));
}
