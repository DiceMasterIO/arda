//! The contract kit's manifest (`contract-kit/manifest.json`), its hashes
//! and the world fingerprint (`contract-kit/world.txt`).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::kit;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub kit_format: u32,
    pub about: String,
    pub fixture: Fixture,
    pub cases: Vec<Case>,
}

/// Which world, build and library the goldens were recorded on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
    pub seed: String,
    pub size: String,
    pub terrain: String,
    pub recipe_version: u16,
    pub society: bool,
    pub arda_version: String,
    pub server_version: String,
    pub world_json_blake3: String,
    pub world_fingerprint: String,
    pub library: String,
    pub library_version: String,
    pub contract_version: u32,
    pub api_version: String,
    pub sheet_mapping: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub content_type: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyHash {
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub route: String,
    pub method: String,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_none_match_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri_from_location_of: Option<String>,
    pub status: u16,
    pub content_type: Option<String>,
    pub headers: BTreeMap<String, Option<String>>,
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub golden: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<BodyHash>,
}

pub fn read_manifest() -> Manifest {
    let path = kit().join("manifest.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub fn write_manifest(m: &Manifest) {
    let mut text = serde_json::to_string_pretty(m).unwrap();
    text.push('\n');
    std::fs::write(kit().join("manifest.json"), text).unwrap();
}

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// `<path>  <blake3>` of every world file, sorted, as `tests/golden_world.rs`
/// writes it; the fixture stamp is not part of the world.
pub fn fingerprint(root: &Path) -> String {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                walk(root, &path, out);
            } else if path.file_name() != Some(crate::fixture_dir::STAMP.as_ref()) {
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
