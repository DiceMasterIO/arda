//! Shared test fixtures under `target/` that several test processes may
//! build at once (review round 2 #45: two `cargo test` processes regenerating
//! one directory in place raced to `OutputNotEmpty` and missing files).
//!
//! A fixture is built in a private sibling directory and renamed into place,
//! so readers only ever see a complete one. Included with `#[path]` by the
//! test targets that need it.

#![allow(dead_code, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

/// Makes `dir` a complete fixture: does nothing when `ready(dir)` holds,
/// otherwise runs `build` on a private temporary directory beside it and
/// renames that into place. Builders hold an exclusive lock on
/// `<name>.lock` beside `dir`, so of several racing threads or processes one
/// builds and the others wait and use its result; without the lock a late
/// builder could take the winner's complete fixture for an interrupted one
/// and move it aside.
pub fn ensure(dir: &Path, ready: impl Fn(&Path) -> bool, build: impl FnOnce(&Path)) {
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
    if dir.exists() {
        // An incomplete directory left by an interrupted older run: move it
        // aside in one step, then drop it.
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
