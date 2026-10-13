//! The Arda side of the Arda ↔ DiceMaster contract kit (1.0 task 10,
//! `contract-kit/README.md`): every case of `contract-kit/manifest.json`
//! replayed in-process against the canonical world (MICRO seed 42, fine
//! terrain at the default recipe, settled and society-built), served with
//! the CLI's default limits and the committed placeholder library, exactly
//! as `arda-server --world <dir> --library assets/tactical/placeholder`.
//!
//! - Every case matches its status, content type, listed headers and golden
//!   bytes or SHA-256 and size.
//! - Every JSON golden validates against the schema the server publishes
//!   under the case's `schema` name.
//! - The manifest's fixture identity is the served world's, and
//!   `contract-kit/world.txt` is the fixture's file fingerprint.
//!
//! `ARDA_BLESS_CONTRACT=1` rewrites the goldens, hashes, expected statuses
//! and headers, the fixture identity and `world.txt`. It never adds or
//! removes cases: those are edited by hand in the manifest.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_server::{router, AppState, ServerConfig};
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tower::ServiceExt;

const BLESS: &str = "ARDA_BLESS_CONTRACT";

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn kit() -> PathBuf {
    workspace().join("contract-kit")
}

fn blessing() -> bool {
    std::env::var_os(BLESS).is_some()
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    kit_format: u32,
    about: String,
    fixture: Fixture,
    cases: Vec<Case>,
}

/// Which world, build and library the goldens were recorded on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Fixture {
    seed: String,
    size: String,
    terrain: String,
    recipe_version: u16,
    society: bool,
    arda_version: String,
    server_version: String,
    world_json_blake3: String,
    world_fingerprint: String,
    library: String,
    library_version: String,
    contract_version: u32,
    api_version: String,
    sheet_mapping: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestBody {
    content_type: String,
    body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BodyHash {
    sha256: String,
    bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Case {
    id: String,
    route: String,
    method: String,
    uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request: Option<RequestBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    if_none_match_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uri_from_location_of: Option<String>,
    status: u16,
    content_type: Option<String>,
    headers: BTreeMap<String, Option<String>>,
    schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    golden: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    body: Option<BodyHash>,
}

fn read_manifest() -> Manifest {
    let path = kit().join("manifest.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn write_manifest(m: &Manifest) {
    let mut text = serde_json::to_string_pretty(m).unwrap();
    text.push('\n');
    std::fs::write(kit().join("manifest.json"), text).unwrap();
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

// ---- the canonical world --------------------------------------------------

fn has_society(dir: &Path) -> bool {
    arda::World::load(dir).is_ok_and(|w| {
        w.seed() == 42
            && w.manifest()
                .fine_terrain
                .as_ref()
                .is_some_and(|f| f.recipe_version == arda::FINE_TERRAIN_RECIPE_VERSION)
    }) && dir.join("society/notables.json").is_file()
        && dir.join("society/gazetteer.json").is_file()
}

/// The stamped society fixture of `tests/world_api/support.rs`, always
/// built here (never `$ARDA_SERVER_TEST_WORLD` or `out/micro42`, which may
/// come from another build).
fn world_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = workspace().join("target/arda-server-fixture/micro42-society");
        fixture_dir::ensure(&dir, has_society, |tmp| {
            arda::generate_from_fine_source(
                42,
                arda::GenerateConfig::MICRO,
                tmp,
                arda::FineDeliveryLimits::default(),
            )
            .expect("generating the MICRO society fixture");
            arda_settle::generate(tmp, arda_settle::grid::MEMORY_BUDGET).expect("settle");
            arda_people::build(tmp).expect("society build");
        });
        dir
    })
}

/// The CLI's defaults (`ServerConfig::new`) with the placeholder library.
fn state() -> Arc<AppState> {
    static STATE: OnceLock<Arc<AppState>> = OnceLock::new();
    Arc::clone(STATE.get_or_init(|| {
        let mut c = ServerConfig::new(world_dir().to_path_buf());
        c.library = workspace().join("assets/tactical/placeholder");
        Arc::new(AppState::open(&c).unwrap())
    }))
}

/// `<path>  <blake3>` of every world file, sorted, as `tests/golden_world.rs`
/// writes it; the fixture stamp is not part of the world.
fn fingerprint(root: &Path) -> String {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                walk(root, &path, out);
            } else if path.file_name() != Some(fixture_dir::STAMP.as_ref()) {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy();
                let hash = blake3::hash(&std::fs::read(&path).unwrap());
                out.push(format!("{}  {}", rel.replace('\\', "/"), hash.to_hex()));
            }
        }
    }
    let mut entries = Vec::new();
    walk(root, root, &mut entries);
    entries.sort();
    entries.join("\n") + "\n"
}

// ---- requests -------------------------------------------------------------

struct Reply {
    status: u16,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .map(|v| v.to_str().unwrap().to_owned())
    }
}

async fn send(case: &Case, etags: &BTreeMap<String, Option<String>>) -> Reply {
    let mut request = Request::builder()
        .method(case.method.as_str())
        .uri(&case.uri);
    let mut body = Body::empty();
    if let Some(r) = &case.request {
        request = request.header("content-type", &r.content_type);
        body = Body::from(std::fs::read(kit().join(&r.body)).unwrap());
    }
    // Without a recorded ETag the request goes unconditional, and the case
    // fails on its status.
    let etag = case.if_none_match_of.as_ref().and_then(|of| etags.get(of));
    if let Some(Some(etag)) = etag {
        request = request.header("if-none-match", etag);
    }
    if case.uri.starts_with("/v1/tactical/prefetch") {
        // The 202 body lists each neighbour as `queued` only on an idle
        // queue; the case's open-sea cells drain in milliseconds.
        let s = state();
        let idle =
            tokio::task::spawn_blocking(move || s.prefetch.wait_idle(Duration::from_secs(600)));
        assert!(idle.await.unwrap(), "prefetch queue never drained");
    }
    let response = router(state())
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
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

/// Every difference between `case` and `reply`; empty when it matches.
fn compare(case: &Case, reply: &Reply) -> Vec<String> {
    let mut problems = Vec::new();
    if reply.status != case.status {
        problems.push(format!("status {}, want {}", reply.status, case.status));
    }
    let content_type = reply.header("content-type");
    if content_type != case.content_type {
        problems.push(format!(
            "content-type {content_type:?}, want {:?}",
            case.content_type
        ));
    }
    for (name, want) in &case.headers {
        let got = reply.header(name);
        if &got != want {
            problems.push(format!("header {name} {got:?}, want {want:?}"));
        }
    }
    if let Some(golden) = &case.golden {
        match std::fs::read(kit().join(golden)) {
            Err(_) => problems.push(format!("missing golden {golden}")),
            Ok(want) if want != reply.body => problems.push(format!(
                "golden {golden} differs ({} bytes wanted, {} served)",
                want.len(),
                reply.body.len()
            )),
            Ok(_) => {}
        }
    } else if let Some(hash) = &case.body {
        let got = BodyHash {
            sha256: sha256(&reply.body),
            bytes: reply.body.len(),
        };
        if &got != hash {
            problems.push(format!("body {got:?}, want {hash:?}"));
        }
    } else {
        problems.push("neither golden nor body hash".to_owned());
    }
    problems
}

/// Records `reply` as `case`'s expectation.
fn bless(case: &mut Case, reply: &Reply) {
    case.status = reply.status;
    case.content_type = reply.header("content-type");
    let names: Vec<String> = case.headers.keys().cloned().collect();
    for name in names {
        case.headers.insert(name.clone(), reply.header(&name));
    }
    if let Some(golden) = &case.golden {
        let path = kit().join(golden);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &reply.body).unwrap();
    } else {
        case.body = Some(BodyHash {
            sha256: sha256(&reply.body),
            bytes: reply.body.len(),
        });
    }
}

async fn served_json(uri: &str) -> serde_json::Value {
    let request = Request::get(uri).body(Body::empty()).unwrap();
    let response = router(state()).oneshot(request).await.unwrap();
    assert_eq!(response.status(), 200, "{uri}");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Validates `body` against the served schema `name`; the first errors.
async fn schema_errors(name: &str, body: &serde_json::Value) -> Vec<String> {
    let schema = served_json(&format!("/v1/schema/{name}.json")).await;
    let validator = jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|e| panic!("{name}.json does not compile: {e}"));
    validator
        .iter_errors(body)
        .take(5)
        .map(|e| format!("{} at {}", e.schema_path(), e.instance_path()))
        .collect()
}

async fn served_fixture() -> Fixture {
    let world = served_json("/v1/world").await;
    let library = served_json("/v1/tactical/library").await;
    let text = |v: &serde_json::Value| v.as_str().unwrap().to_owned();
    let world_json = std::fs::read(world_dir().join("world.json")).unwrap();
    Fixture {
        seed: text(&world["seed"]),
        size: "micro".into(),
        terrain: "fine".into(),
        recipe_version: u16::try_from(world["fine_terrain"]["recipe_version"].as_u64().unwrap())
            .unwrap(),
        society: world_dir().join("society").is_dir(),
        arda_version: text(&world["arda_version"]),
        server_version: text(&world["server_version"]),
        world_json_blake3: blake3::hash(&world_json).to_hex().to_string(),
        world_fingerprint: "world.txt".into(),
        library: text(&library["library"]),
        library_version: text(&library["library_version"]),
        contract_version: u32::try_from(world["contract_version"].as_u64().unwrap()).unwrap(),
        api_version: text(&world["api_version"]),
        sheet_mapping: None,
    }
}

// ---- tests ----------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn every_case_matches_its_golden() {
    let mut manifest = read_manifest();
    let blessing = blessing();
    // ETags by case id, for the `If-None-Match` revalidation cases.
    let mut etags: BTreeMap<String, Option<String>> = manifest
        .cases
        .iter()
        .map(|c| (c.id.clone(), c.headers.get("etag").cloned().flatten()))
        .collect();
    // `Location` headers by case id, for the job a later case deletes.
    let mut locations: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut failures = Vec::new();
    for case in &mut manifest.cases {
        if let (true, Some(of)) = (blessing, &case.uri_from_location_of) {
            let location = locations.get(of).cloned().flatten();
            case.uri = location.unwrap_or_else(|| panic!("{}: case {of} has no location", case.id));
        }
        let reply = send(case, &etags).await;
        locations.insert(case.id.clone(), reply.header("location"));
        if blessing {
            bless(case, &reply);
            etags.insert(case.id.clone(), reply.header("etag"));
        } else {
            let problems = compare(case, &reply);
            if !problems.is_empty() {
                failures.push(format!("{}: {}", case.id, problems.join("; ")));
            }
        }
        // Hashed JSON bodies are not in `golden/`; validate them here.
        let json = case
            .content_type
            .as_deref()
            .is_some_and(|t| t.starts_with("application/json"));
        if let (Some(name), true, true) = (&case.schema, json, case.golden.is_none()) {
            let body = serde_json::from_slice(&reply.body).unwrap();
            let errors = schema_errors(name, &body).await;
            assert!(errors.is_empty(), "{} against {name}: {errors:#?}", case.id);
        }
    }
    if blessing {
        manifest.fixture = served_fixture().await;
        write_manifest(&manifest);
        std::fs::write(kit().join("world.txt"), fingerprint(world_dir())).unwrap();
        return;
    }
    assert!(
        failures.is_empty(),
        "{} of {} contract-kit cases differ (re-bless with {BLESS}=1 only for an intended change):\n{}",
        failures.len(),
        manifest.cases.len(),
        failures.join("\n")
    );
}

#[tokio::test]
async fn every_json_golden_validates_against_its_schema() {
    if blessing() {
        return;
    }
    let manifest = read_manifest();
    let mut checked = 0;
    for case in &manifest.cases {
        let (Some(name), Some(golden)) = (&case.schema, &case.golden) else {
            continue;
        };
        let path = kit().join(golden);
        let bytes =
            std::fs::read(&path).unwrap_or_else(|_| panic!("{}: missing golden {golden}", case.id));
        let body: serde_json::Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("{}: {golden} is not JSON: {e}", case.id));
        let errors = schema_errors(name, &body).await;
        assert!(errors.is_empty(), "{} against {name}: {errors:#?}", case.id);
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} JSON goldens carry a schema");
}

#[tokio::test]
async fn the_fixture_identity_is_the_served_world() {
    if blessing() {
        return;
    }
    let manifest = read_manifest();
    assert_eq!(manifest.kit_format, 1);
    assert_eq!(manifest.fixture, served_fixture().await);
    let want = std::fs::read_to_string(kit().join("world.txt")).unwrap();
    assert_eq!(
        fingerprint(world_dir()),
        want,
        "contract-kit/world.txt is not the fixture's fingerprint"
    );
}

#[test]
fn cases_are_unique_and_well_formed() {
    let manifest = read_manifest();
    let mut ids = std::collections::BTreeSet::new();
    for case in &manifest.cases {
        assert!(ids.insert(&case.id), "duplicate case {}", case.id);
        assert!(
            case.golden.is_some() != case.body.is_some() || blessing(),
            "{}: exactly one of golden and body",
            case.id
        );
        assert!(case.uri.starts_with("/v1/"), "{}", case.id);
        assert!(
            case.route.starts_with(&format!("{} /v1/", case.method)),
            "{}: route {}",
            case.id,
            case.route
        );
        if let Some(of) = &case.if_none_match_of {
            assert!(ids.contains(of), "{}: {of} must come first", case.id);
        }
        if let Some(of) = &case.uri_from_location_of {
            let source = manifest.cases.iter().find(|c| &c.id == of);
            let location = source.and_then(|c| c.headers.get("location").cloned().flatten());
            assert!(ids.contains(of), "{}: {of} must come first", case.id);
            assert!(
                blessing() || location.as_deref() == Some(case.uri.as_str()),
                "{}: uri is not {of}'s location {location:?}",
                case.id
            );
        }
        if let Some(r) = &case.request {
            assert!(kit().join(&r.body).is_file(), "{}: {}", case.id, r.body);
        }
    }
}
