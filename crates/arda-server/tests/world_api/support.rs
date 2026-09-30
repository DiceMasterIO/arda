//! Shared fixture: a real seed-42 MICRO fine world, served in-process.
//!
//! Resolution order: `$ARDA_SERVER_TEST_WORLD`, then `<workspace>/out/micro42`
//! (the documented `arda generate --seed 42 --micro --terrain fine` output),
//! else the world is generated once into `target/arda-server-fixture/`.

use arda_server::{router, AppState, ServerConfig};
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tower::ServiceExt;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn loads(dir: &Path) -> bool {
    arda::World::load(dir).is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == 42)
}

/// The fixture world directory.
pub fn world_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = std::env::var_os("ARDA_SERVER_TEST_WORLD") {
            return PathBuf::from(dir);
        }
        let out = workspace().join("out/micro42");
        if loads(&out) {
            return out;
        }
        let dir = workspace().join("target/arda-server-fixture/micro42");
        if !loads(&dir) {
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
            arda::generate_from_fine_source(
                42,
                arda::GenerateConfig::MICRO,
                &dir,
                arda::FineDeliveryLimits::default(),
            )
            .expect("generating the MICRO fixture world");
        }
        dir
    })
}

/// Test limits: a 512 px pyramid (max zoom 1) keeps renders quick.
pub fn config() -> ServerConfig {
    let mut config = ServerConfig::new(world_dir().to_path_buf());
    config.overview.tile_base_px = 512;
    config.overview.max_quality_px = 1024;
    config.library = library_dir();
    config
}

/// The committed placeholder tactical library.
pub fn library_dir() -> PathBuf {
    workspace().join("assets/tactical/placeholder")
}

/// One shared state, so caches warm across tests.
pub fn state() -> Arc<AppState> {
    static STATE: OnceLock<Arc<AppState>> = OnceLock::new();
    Arc::clone(STATE.get_or_init(|| Arc::new(AppState::open(&config()).unwrap())))
}

/// A response, fully read.
pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&self.body)))
    }

    pub fn error_code(&self) -> String {
        self.json()["error"]["code"].as_str().unwrap().to_owned()
    }
}

/// Sends `GET uri` through the real router with optional `Origin`.
pub async fn get_with(state: Arc<AppState>, uri: &str, origin: Option<&str>) -> Reply {
    let mut request = Request::get(uri);
    if let Some(origin) = origin {
        request = request.header("origin", origin);
    }
    send(state, request.body(Body::empty()).unwrap()).await
}

/// `POST uri` with a JSON body against the shared state.
pub async fn post(uri: &str, body: Vec<u8>) -> Reply {
    let request = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    send(state(), request).await
}

/// Sends any request through the real router.
pub async fn send(state: Arc<AppState>, request: Request<Body>) -> Reply {
    let response = router(state).oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    Reply {
        status,
        headers,
        body,
    }
}

/// `GET uri` against the shared state.
pub async fn get(uri: &str) -> Reply {
    get_with(state(), uri, None).await
}

/// Sends `POST uri` with a JSON body through the real router.
pub async fn post_json(uri: &str, body: Vec<u8>) -> Reply {
    let request = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let response = router(state()).oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    Reply {
        status,
        headers,
        body,
    }
}
