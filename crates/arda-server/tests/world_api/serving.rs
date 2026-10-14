//! Serving concurrency (review 3.6-1, 3.6-2, 3.6-7): single-flight builds,
//! abandoned requests leaving the queue, and workers surviving panics,
//! against the real MICRO seed-42 world. No sleeps: requests line up behind
//! the held request lane, and the tests wait on the parked-caller count.

use crate::support::{config, send, Reply};
use arda_server::AppState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The river cell of `cells.rs` and its eastern neighbour (both land).
const CELL: (u32, u32) = (529, 812);
const OTHER: (u32, u32) = (530, 812);
/// Quick renders: 4096 px a side, max zoom 3.
const PPSQ: u32 = 64;
const WAIT: Duration = Duration::from_secs(120);

fn fresh() -> Arc<AppState> {
    let mut c = config();
    c.prefetch_workers = 0;
    c.bake_workers = 0;
    Arc::new(AppState::open(&c).unwrap())
}

async fn fetch(state: Arc<AppState>, uri: String) -> Reply {
    send(state, Request::get(uri).body(Body::empty()).unwrap()).await
}

fn tile_uri((gx, gy): (u32, u32), (x, y): (u32, u32)) -> String {
    format!("/v1/tactical/cell/{gx}/{gy}/tiles/3/{x}/{y}.webp?ppsq={PPSQ}")
}

/// Holds the request lane shut until dropped, so a failing test never
/// leaves a blocking task parked (the runtime waits for those at exit).
struct Hold(Arc<AppState>);

impl Hold {
    fn new(s: &Arc<AppState>) -> Self {
        s.tactical.hold_request_lane(true);
        Self(Arc::clone(s))
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        self.0.tactical.hold_request_lane(false);
    }
}

/// Waits until exactly `n` callers are parked, up to [`WAIT`].
async fn parked(s: &AppState, n: usize) -> bool {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if s.tactical.parked() == n {
            return true;
        }
        tokio::task::yield_now().await;
    }
    false
}

/// 3.6-1: sixteen concurrent tile requests on a cold cell render it once
/// and build its pyramid once (the review measured 16 builds).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_cold_tiles_build_one_pyramid() {
    let s = fresh();
    let hold = Hold::new(&s);
    let tasks: Vec<_> = (0..16)
        .map(|i| tokio::spawn(fetch(Arc::clone(&s), tile_uri(CELL, (i % 4, i / 4)))))
        .collect();
    assert!(parked(&s, 16).await, "{} parked", s.tactical.parked());
    let start = Instant::now();
    drop(hold);
    let mut bodies = Vec::new();
    for t in tasks {
        let r = t.await.unwrap();
        assert_eq!(r.status, StatusCode::OK);
        bodies.push(r.body);
    }
    eprintln!(
        "16 cold tiles: last after {:.0} ms, builds {:?}",
        start.elapsed().as_secs_f64() * 1e3,
        s.tactical.build_counts()
    );
    assert_eq!(s.tactical.build_counts(), [1, 1], "[renders, pyramids]");
    // Same request, same bytes, warm.
    let again = fetch(Arc::clone(&s), tile_uri(CELL, (1, 2))).await;
    assert_eq!(again.body, bodies[1 + 2 * 4]);
}

/// 3.6-2: a request whose client went away while it queued leaves the
/// queue; the live request behind it renders first and alone.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_abandoned_request_does_not_delay_a_live_one() {
    let s = fresh();
    let hold = Hold::new(&s);
    let abandoned = tokio::spawn(fetch(Arc::clone(&s), tile_uri(OTHER, (0, 0))));
    assert!(parked(&s, 1).await);
    abandoned.abort();
    assert!(abandoned.await.is_err_and(|e| e.is_cancelled()));
    assert!(parked(&s, 0).await, "the abandoned request still queues");
    let live = tokio::spawn(fetch(Arc::clone(&s), tile_uri(CELL, (0, 0))));
    assert!(parked(&s, 1).await);
    let start = Instant::now();
    drop(hold);
    let r = live.await.unwrap();
    assert_eq!(r.status, StatusCode::OK);
    eprintln!(
        "live tile behind an abandoned one: {:.0} ms",
        start.elapsed().as_secs_f64() * 1e3
    );
    assert_eq!(
        s.tactical.build_counts()[0],
        1,
        "only the live cell rendered"
    );
    // The abandoned cell renders normally later, to the same bytes.
    let a = fetch(Arc::clone(&s), tile_uri(OTHER, (0, 0))).await;
    let b = fetch(fresh(), tile_uri(OTHER, (0, 0))).await;
    assert_eq!((a.status, &a.body), (StatusCode::OK, &b.body));
}
