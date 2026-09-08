//! The cross-platform determinism gate (`architecture-interview.md` §Q4,
//! `06-testing.md` golden-world snapshots).
//!
//! Regenerating the fixture: run with `ARDA_BLESS=1` to rewrite
//! `tests/golden/micro-42.txt`, then inspect the diff before committing.
//! `code-prefs.md` §Q9 forbids changing golden hashes unless explicitly asked.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda::{generate, GenerateConfig};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const GOLDEN: &str = "tests/golden/micro-42.txt";

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("arda-golden-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every world file, sorted, as `<relative path>  <blake3 hex>`.
fn fingerprint(root: &Path) -> String {
    let mut entries = Vec::new();
    collect(root, root, &mut entries);
    entries.sort();
    entries.join("\n")
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    // Sort so traversal order cannot vary by filesystem.
    let mut paths: Vec<PathBuf> = read.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect(root, &path, out);
        } else if let Ok(bytes) = std::fs::read(&path) {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                // Windows writes backslashes; normalise so one fixture serves
                // all three runners.
                .replace('\\', "/");
            out.push(format!("{rel}  {}", blake3::hash(&bytes).to_hex()));
        }
    }
}

fn generated_fingerprint() -> &'static str {
    static FIRST_RUN: LazyLock<String> = LazyLock::new(|| {
        let dir = TempDir::new("micro-42");
        generate(42, GenerateConfig::MICRO, dir.path()).expect("generation failed");
        fingerprint(dir.path())
    });
    &FIRST_RUN
}

#[test]
fn micro_world_matches_the_golden_fingerprint() {
    let actual = generated_fingerprint();

    if std::env::var("ARDA_BLESS").is_ok() {
        std::fs::create_dir_all("tests/golden").expect("cannot create fixture dir");
        std::fs::write(GOLDEN, actual).expect("cannot write fixture");
        return;
    }

    let expected = std::fs::read_to_string(GOLDEN)
        .unwrap_or_else(|_| panic!("{GOLDEN} is missing; run with ARDA_BLESS=1 to create it"));

    assert_eq!(
        actual.trim(),
        expected.trim(),
        "world fingerprint changed — a sim rule or the byte layout moved"
    );
}

#[test]
fn regenerating_produces_the_same_fingerprint() {
    // Reuse the first actual generation, but always generate the comparison
    // independently; comparing two copies would not test reproducibility.
    let first = generated_fingerprint();
    let b = TempDir::new("repeat-b");
    generate(42, GenerateConfig::MICRO, b.path()).expect("generation failed");
    assert_eq!(first, fingerprint(b.path()));
}
