//! Review round 2 #45: builders racing on one shared fixture directory all
//! end with one complete fixture and no leftovers, and an incomplete
//! directory from an interrupted run is replaced.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "support/fixture_dir.rs"]
mod fixture_dir;

use std::path::{Path, PathBuf};

fn ready(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("done")).is_ok_and(|s| s == "complete")
}

fn build(tmp: &Path) {
    std::fs::create_dir(tmp).unwrap();
    for k in 0..20 {
        std::fs::write(tmp.join(format!("part{k}")), [k]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    std::fs::write(tmp.join("done"), "complete").unwrap();
}

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("arda-fixture-dir-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn racing_builders_publish_one_complete_fixture() {
    let root = scratch("race");
    let dir = root.join("micro42");
    let threads: Vec<_> = (0..6)
        .map(|_| {
            let dir = dir.clone();
            std::thread::spawn(move || {
                fixture_dir::ensure(&dir, ready, build);
                assert!(ready(&dir), "every caller sees a complete fixture");
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 21);
    let left: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().is_dir())
        .collect();
    assert_eq!(left.len(), 1, "no temporary or stale directories remain");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_incomplete_fixture_is_replaced() {
    let root = scratch("stale");
    let dir = root.join("micro42");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("part0"), [9]).unwrap();
    fixture_dir::ensure(&dir, ready, build);
    assert!(ready(&dir));
    assert_eq!(std::fs::read(dir.join("part0")).unwrap(), [0]);
    let _ = std::fs::remove_dir_all(&root);
}
