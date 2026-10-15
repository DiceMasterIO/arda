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

async fn post(state: &Arc<AppState>, uri: &str, body: &serde_json::Value) -> Reply {
    let request = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap();
    send(Arc::clone(state), request).await
}

async fn idle(state: &Arc<AppState>, what: fn(&AppState) -> bool) -> bool {
    let s = Arc::clone(state);
    tokio::task::spawn_blocking(move || what(&s)).await.unwrap()
}

/// 3.6-7: a panicking bake fails its job with `internal` and the worker
/// keeps baking (the review saw 503 `busy` until a restart).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panicking_bake_fails_its_job_and_the_worker_survives() {
    let mut c = config();
    c.prefetch_workers = 0;
    c.bake_workers = 1;
    let s = Arc::new(AppState::open(&c).unwrap());
    let (gx, gy) = CELL;
    let cell = fetch(Arc::clone(&s), format!("/v1/tactical/cell/{gx}/{gy}")).await;
    let layout = cell.json()["layout"].clone();
    let body = |elevation: i64| {
        let mut l = layout.clone();
        l["squares"][0]["elevation_ft"] = serde_json::json!(elevation);
        serde_json::json!({"gx": gx, "gy": gy, "ppsq": PPSQ, "layout": l})
    };
    s.bake.inject_panic(true);
    let r = post(&s, "/v1/tactical/bake", &body(1001)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    let id = r.json()["job_id"].as_str().unwrap().to_owned();
    let w = Arc::clone(&s);
    let i = id.clone();
    assert!(
        tokio::task::spawn_blocking(move || {
            w.bake.wait_state(
                &i,
                arda_server::tactical::bake::BakeState::Failed,
                Duration::from_secs(60),
            )
        })
        .await
        .unwrap(),
        "the panicked job never failed"
    );
    let status = fetch(Arc::clone(&s), format!("/v1/tactical/bake/{id}")).await;
    assert_eq!(status.json()["error"]["code"], "internal");
    s.bake.inject_panic(false);
    let r = post(&s, "/v1/tactical/bake", &body(1002)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    assert!(idle(&s, |s| s.bake.wait_idle(WAIT)).await);
    let id = r.json()["job_id"].as_str().unwrap().to_owned();
    let status = fetch(Arc::clone(&s), format!("/v1/tactical/bake/{id}")).await;
    assert_eq!(status.json()["state"], "done");
}

/// 3.6-7: a panicking prefetch job is dropped and its worker keeps
/// warming (the queue went idle never again before).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panicking_prefetch_job_leaves_the_worker_alive() {
    let mut c = config();
    c.prefetch_workers = 1;
    c.bake_workers = 0;
    let s = Arc::new(AppState::open(&c).unwrap());
    let (gx, gy) = CELL;
    s.prefetch.inject_panic(true);
    let r = post(
        &s,
        "/v1/tactical/prefetch",
        &serde_json::json!({"gx": gx, "gy": gy, "ppsq": 16}),
    )
    .await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    assert!(
        idle(&s, |s| s.prefetch.wait_idle(Duration::from_secs(60))).await,
        "the queue never drained"
    );
    s.prefetch.inject_panic(false);
    let r = post(
        &s,
        "/v1/tactical/prefetch",
        &serde_json::json!({"gx": gx, "gy": gy, "ppsq": 16}),
    )
    .await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    let cells = r.json()["cells"].as_array().unwrap().clone();
    assert!(cells.iter().all(|c| c["status"] == "queued"), "{cells:?}");
    assert!(idle(&s, |s| s.prefetch.wait_idle(WAIT)).await);
    // The worker warmed the neighbours' bodies this time.
    let r = fetch(
        Arc::clone(&s),
        format!("/v1/tactical/cell/{}/{gy}?ppsq=16", gx - 1),
    )
    .await;
    assert_eq!(r.headers["x-arda-cache"], "hit");
}

/// 3.6-5: a render is held once. The pyramid built from it shares it and
/// the render cache lets it go; a PNG of the same render reads it from the
/// pyramid instead of rendering it again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_render_is_cached_once_by_its_pyramid() {
    let s = fresh();
    let tile = fetch(Arc::clone(&s), tile_uri(CELL, (0, 0))).await;
    assert_eq!(tile.status, StatusCode::OK);
    let [renders, pyramids, _, _] = s.tactical.cache_counts();
    assert_eq!((renders, pyramids), (0, 1), "the render is cached twice");
    let (gx, gy) = CELL;
    let png = fetch(
        Arc::clone(&s),
        format!("/v1/tactical/cell/{gx}/{gy}.png?ppsq={PPSQ}"),
    )
    .await;
    assert_eq!(png.status, StatusCode::OK);
    assert_eq!(s.tactical.build_counts(), [1, 1], "the PNG rendered again");
}

/// 3.6-11: an `ETag` names the request, not the bytes, so revalidating a
/// cold server's image answers `304` without rendering or composing it;
/// JSON bodies, the scene and dungeon images carry the same headers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revalidation_never_renders_and_headers_agree() {
    let (gx, gy) = CELL;
    let uris = [
        tile_uri(CELL, (0, 0)),
        format!("/v1/tactical/cell/{gx}/{gy}.png?ppsq={PPSQ}"),
        format!("/v1/tactical/cell/{gx}/{gy}"),
        format!("/v1/tactical/cell/{gx}/{gy}/scene?time=night"),
        "/v1/tactical/dungeon?seed=7".to_owned(),
        "/v1/tactical/dungeon.png?seed=7&ppsq=64".to_owned(),
        "/v1/tactical/layout/riverside.png?ppsq=64".to_owned(),
    ];
    let warm = fresh();
    let mut tags = Vec::new();
    for uri in &uris {
        let r = fetch(Arc::clone(&warm), uri.clone()).await;
        assert_eq!(r.status, StatusCode::OK, "{uri}");
        assert_eq!(r.headers["cache-control"], "no-cache", "{uri}");
        tags.push(r.headers["etag"].to_str().unwrap().to_owned());
    }
    let cold = fresh();
    for (uri, tag) in uris.iter().zip(&tags) {
        let request = Request::get(uri.as_str())
            .header("if-none-match", tag.as_str())
            .body(Body::empty())
            .unwrap();
        let r = send(Arc::clone(&cold), request).await;
        assert_eq!(r.status, StatusCode::NOT_MODIFIED, "{uri}");
        assert_eq!(r.headers["etag"], tag.as_str(), "{uri}");
        assert_eq!(r.headers["cache-control"], "no-cache", "{uri}");
    }
    assert_eq!(
        cold.tactical.build_counts(),
        [0, 0],
        "revalidation rendered"
    );
    assert_eq!(cold.tactical.cache_counts(), [0, 0, 0, 0]);
}
