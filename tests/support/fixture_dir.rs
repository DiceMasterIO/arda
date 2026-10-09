//! Shared test fixtures under `target/` that several test processes may
//! build at once (review round 2 #45: two `cargo test` processes regenerating
//! one directory in place raced to `OutputNotEmpty` and missing files).
//!
//! A fixture is built in a private sibling directory and renamed into place,
//! so readers only ever see a complete one. Included with `#[path]` by the
//! test targets that need it.
//!
//! Each fixture carries a stamp, [`STAMP`], holding a fingerprint of the
//! workspace's library inputs (sources, data files, manifests, lock file).
//! A fixture whose stamp differs was built by other code and is rebuilt, so
//! a local `target/` never serves a world generated before a code or data
//! change. Test files are left out of the fingerprint: fixture directories
//! are shared across test targets, and a test edit should not regenerate a
//! world. A builder change in a test file that alters a fixture bumps
//! [`FORMAT`] instead.

#![allow(dead_code, clippy::expect_used, clippy::unwrap_used)]

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Bumped when a fixture builder in a test file changes what it writes.
pub const FORMAT: u32 = 1;

/// The stamp file inside a fixture directory.
pub const STAMP: &str = ".fixture-stamp";

/// Makes `dir` a complete fixture: does nothing when `ready(dir)` holds,
/// otherwise runs `build` on a private temporary directory beside it and
/// renames that into place. Builders hold an exclusive lock on
/// `<name>.lock` beside `dir`, so of several racing threads or processes one
/// builds and the others wait and use its result; without the lock a late
/// builder could take the winner's complete fixture for an interrupted one
/// and move it aside. A fixture whose [`STAMP`] is not the current
/// [`fingerprint`] counts as not ready.
pub fn ensure(dir: &Path, ready: impl Fn(&Path) -> bool, build: impl FnOnce(&Path)) {
    let ready = |d: &Path| stamped(d) && ready(d);
    if ready(dir) {
        return;
    }
    let parent = dir.parent().expect("a fixture directory has a parent");
    std::fs::create_dir_all(parent).expect("creating the fixture parent");
    let name = dir
        .file_name()
        .expect("a fixture directory has a name")
        .to_string_lossy()
        .into_owned();
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(parent.join(format!("{name}.lock")))
        .expect("opening the fixture lock");
    lock.lock().expect("locking the fixture");
    if ready(dir) {
        return;
    }
    // Unique per process and per call, so a crashed builder's leftovers
    // never collide with a new one.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let pid = std::process::id();
    let tmp = parent.join(format!("{name}.tmp-{pid}-{n}"));
    let _ = std::fs::remove_dir_all(&tmp);
    build(&tmp);
    std::fs::write(tmp.join(STAMP), fingerprint()).expect("stamping the fixture");
    if dir.exists() {
        // An incomplete directory left by an interrupted older run, or a
        // complete one built by older code: move it aside in one step, then
        // drop it.
        let stale = parent.join(format!("{name}.stale-{pid}-{n}"));
        if std::fs::rename(dir, &stale).is_ok() {
            let _ = std::fs::remove_dir_all(&stale);
        }
    }
    if let Err(e) = std::fs::rename(&tmp, dir) {
        panic!("publishing the fixture {}: {e}", dir.display());
    }
    // The lock is released when `lock` drops; the file stays for the next run.
}

fn stamped(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join(STAMP)).is_ok_and(|s| s == fingerprint())
}

/// Fingerprint of the code and data that build fixtures: [`FORMAT`], the
/// workspace manifest (and so the workspace version), lock file and toolchain, and
/// every file of each `crates/*` member outside its `tests`, `benches` and
/// `examples` directories (compiled-in data such as
/// `arda-npc/data/content/occupations.json` included). Computed once per
/// process.
pub fn fingerprint() -> &'static str {
    static PRINT: OnceLock<String> = OnceLock::new();
    PRINT.get_or_init(|| {
        let root = workspace_root();
        let mut files = Vec::new();
        for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
            files.push(root.join(name));
        }
        let mut members: Vec<PathBuf> = std::fs::read_dir(root.join("crates"))
            .expect("listing the workspace crates")
            .map(|e| e.expect("a crate entry").path())
            .filter(|p| p.is_dir())
            .collect();
        members.sort();
        for member in members {
            collect(&member, &member, &mut files);
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        FORMAT.hash(&mut h);
        for file in &files {
            file.strip_prefix(&root).unwrap_or(file).hash(&mut h);
            std::fs::read(file).ok().hash(&mut h);
        }
        format!(
            "format {FORMAT} files {} hash {:016x}\n",
            files.len(),
            h.finish()
        )
    })
}

/// The directory whose `Cargo.toml` declares `[workspace]`, found upward
/// from the including crate.
fn workspace_root() -> PathBuf {
    let start = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    start
        .ancestors()
        .find(|d| {
            std::fs::read_to_string(d.join("Cargo.toml")).is_ok_and(|s| s.contains("[workspace]"))
        })
        .expect("a workspace root above the crate")
        .to_path_buf()
}

/// Files under `dir`, sorted, skipping the member's test-only directories.
fn collect(member: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("listing a crate directory")
        .map(|e| e.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            let test_only = dir == member
                && path.file_name().is_some_and(|n| {
                    ["tests", "benches", "examples"].contains(&&*n.to_string_lossy())
                });
            if !test_only {
                collect(member, &path, files);
            }
        } else {
            files.push(path);
        }
    }
}
