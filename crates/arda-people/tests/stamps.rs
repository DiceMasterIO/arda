//! Stored society files are tied to the settlement files and the world
//! they were built from, and the notables are read lazily (review 3.3-4,
//! 3.3-5, 3.3-10).
//!
//! World resolution: `$ARDA_TEST_WORLD`, then a MICRO world generated and
//! settled once into `target/arda-people-fixture/micro42`. Nothing is
//! written into the world.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_people::people::check_stamps;
use arda_people::{Notables, NotablesFile, SocietyFiles};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

fn ready(dir: &Path) -> bool {
    dir.join("society/settlements.json").is_file()
        && arda::World::load(dir)
            .is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == 42)
}

fn world_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ARDA_TEST_WORLD") {
        return PathBuf::from(dir);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("target/arda-people-fixture/micro42");
    fixture_dir::ensure(&dir, ready, |tmp| {
        arda::generate_from_fine_source(
            42,
            arda::GenerateConfig::MICRO,
            tmp,
            arda::FineDeliveryLimits::default(),
        )
        .expect("generating the MICRO fixture world");
        arda_settle::generate(tmp, arda_settle::grid::MEMORY_BUDGET).expect("settle");
    });
    dir
}

static BUILT: LazyLock<(PathBuf, arda_people::Built)> = LazyLock::new(|| {
    let dir = world_dir();
    let built = arda_people::compute(&dir).unwrap();
    (dir, built)
});

struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp(tag: &str) -> Temp {
    let d = std::env::temp_dir().join(format!("arda-people-stamps-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    Temp(d)
}

fn written(file: &NotablesFile, at: &Path) -> Notables {
    let path = at.join("notables.json");
    std::fs::write(&path, arda_people::notables::encode(&path, file).unwrap()).unwrap();
    Notables::open(&path).unwrap()
}

#[test]
fn the_build_records_the_settle_stamp_and_plan_fingerprints() {
    let (dir, built) = &*BUILT;
    let files = SocietyFiles::read(&dir.join("society")).unwrap();
    let stamp = files
        .settlements
        .stamp
        .as_ref()
        .expect("settle stamps its output");
    assert_ne!(stamp.world_json, arda_settle::stamp::DETACHED);
    assert_eq!(built.society.settle_stamp.as_ref(), Some(&stamp.id));
    assert_eq!(built.notables.settle_stamp.as_ref(), Some(&stamp.id));
    for s in &built.notables.settlements {
        assert_eq!(
            s.plan_fingerprint.is_some(),
            s.buildings == "plan",
            "{}",
            s.settlement_id
        );
    }
}

#[test]
fn notables_are_read_one_settlement_at_a_time() {
    let (_, built) = &*BUILT;
    let t = temp("lazy");
    let n = written(&built.notables, &t.0);
    assert_eq!(n.len(), built.notables.settlements.len());
    assert_eq!(n.loaded(), 0, "opening parses the header only");
    // The file is still one JSON document of the same shape.
    let whole: NotablesFile =
        serde_json::from_slice(&std::fs::read(t.0.join("notables.json")).unwrap()).unwrap();
    assert_eq!(whole.settlements, built.notables.settlements);
    let s = built
        .notables
        .settlements
        .iter()
        .find(|s| !s.npcs.is_empty())
        .unwrap();
    let npc = &s.npcs[0];
    let (found, sid) = n.notable(npc.id).unwrap().unwrap();
    assert_eq!((found, sid), (npc, s.settlement_id));
    assert_eq!(n.loaded(), 1);
    assert_eq!(n.of(s.settlement_id.get()).unwrap().unwrap(), s);
    assert_eq!(n.loaded(), 1);
    // A format-1 file (pretty, no index) is still read.
    let mut old = built.notables.clone();
    old.format_version = 1;
    let path = t.0.join("old.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&old).unwrap()).unwrap();
    let o = Notables::open(&path).unwrap();
    assert_eq!(o.notable(npc.id).unwrap().unwrap().0, npc);
}

#[test]
fn society_files_from_another_settle_run_are_refused() {
    let (dir, built) = &*BUILT;
    let society_dir = dir.join("society");
    let files = SocietyFiles::read(&society_dir).unwrap();
    let t = temp("stale");
    let fresh = written(&built.notables, &t.0);
    check_stamps(&society_dir, &files, Some(&built.society), Some(&fresh)).unwrap();
    // A society built from other settlement files.
    let mut soc = built.society.clone();
    soc.settle_stamp = Some("0".repeat(32));
    let e = check_stamps(&society_dir, &files, Some(&soc), Some(&fresh)).unwrap_err();
    assert!(e.to_string().contains("society.json"), "{e}");
    assert!(e.to_string().contains("arda society build"), "{e}");
    // Notables from an unstamped (older) build.
    let mut old = built.notables.clone();
    old.settle_stamp = None;
    let stale = written(&old, &t.0);
    let e = check_stamps(&society_dir, &files, Some(&built.society), Some(&stale)).unwrap_err();
    assert!(e.to_string().contains("notables.json"), "{e}");
    // Settlement files from another world.
    let mut moved = files.clone();
    if let Some(s) = moved.settlements.stamp.as_mut() {
        s.world_json = "f".repeat(32);
    }
    let e = check_stamps(&society_dir, &moved, None, None).unwrap_err();
    assert!(e.to_string().contains("arda settle"), "{e}");
}

#[test]
fn a_plan_drawn_again_is_checked_against_the_stored_notables() {
    let (dir, built) = &*BUILT;
    let t = temp("plan");
    std::fs::copy(dir.join("world.json"), t.0.join("world.json")).unwrap();
    let soc = t.0.join("society");
    std::fs::create_dir_all(&soc).unwrap();
    for name in [
        "settlements.json",
        "roads.json",
        "realms.json",
        "gazetteer.json",
    ] {
        if dir.join("society").join(name).is_file() {
            std::fs::copy(dir.join("society").join(name), soc.join(name)).unwrap();
        }
    }
    arda_society::output::write_json(&built.society, &soc.join("society.json")).unwrap();
    // Two planned hamlets: the first one's stored fingerprint is tampered
    // with, as if its notables came from another plan.
    let files = SocietyFiles::read(&soc).unwrap();
    let hamlet = |s: &&arda_people::StoredNotable| {
        s.buildings == "plan"
            && !s.npcs.is_empty()
            && files
                .settlement(s.settlement_id.get())
                .is_ok_and(|(x, _)| x.tier == arda_settle::model::Tier::Hamlet)
    };
    let mut planned = built.notables.settlements.iter().filter(hamlet);
    let (a, b) = (
        planned.next().unwrap().settlement_id.get(),
        planned.next().unwrap().settlement_id.get(),
    );
    let mut notables = built.notables.clone();
    for s in &mut notables.settlements {
        if s.settlement_id.get() == a {
            s.plan_fingerprint = Some("0".repeat(32));
        }
    }
    written(&notables, &soc);
    let src = arda_people::shared::SharedSource::open(dir).unwrap();
    let w = arda_people::World::with_source(src, &soc).unwrap();
    w.plan(b).unwrap();
    assert!(
        !w.try_notables_of(b).unwrap().is_empty(),
        "a matching plan serves its notables"
    );
    w.plan(a).unwrap();
    let e = w.try_notables_of(a).unwrap_err();
    assert!(matches!(e, arda_people::PeopleError::Stale(_)), "{e}");
    assert!(w.notables_of(a).is_empty());
}

#[test]
fn commoner_lookups_reuse_one_skeleton_per_settlement() {
    let (dir, built) = &*BUILT;
    let t = temp("gen");
    std::fs::copy(dir.join("world.json"), t.0.join("world.json")).unwrap();
    let soc = t.0.join("society");
    std::fs::create_dir_all(&soc).unwrap();
    for name in [
        "settlements.json",
        "roads.json",
        "realms.json",
        "gazetteer.json",
    ] {
        if dir.join("society").join(name).is_file() {
            std::fs::copy(dir.join("society").join(name), soc.join(name)).unwrap();
        }
    }
    arda_society::output::write_json(&built.society, &soc.join("society.json")).unwrap();
    written(&built.notables, &soc);
    let src = arda_people::shared::SharedSource::open(dir).unwrap();
    let w = arda_people::World::with_source(src, &soc).unwrap();
    let id = built
        .notables
        .settlements
        .iter()
        .find(|s| s.buildings == "plan")
        .unwrap()
        .settlement_id
        .get();
    let a = w.generator(id).unwrap();
    let b = w.generator(id).unwrap();
    assert!(
        std::sync::Arc::ptr_eq(&a, &b),
        "the second lookup reuses the skeleton"
    );
    // The cached generator expands the same people as a fresh one.
    let profile = w.profile(id).unwrap();
    let (buildings, _) = w.buildings(id).unwrap();
    let slots = w.slots(id).unwrap();
    let fresh = arda_npc::Generator::with_notables(w.seed(), &profile, &buildings, &slots).unwrap();
    let ids: Vec<_> = fresh.ids().take(25).collect();
    for npc in ids {
        assert_eq!(a.npc(npc).unwrap(), fresh.npc(npc).unwrap());
    }
}
