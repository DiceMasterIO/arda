//! The stored notables of the real seed-42 MICRO world have a plausible
//! medieval mix (logic/13 §npc-ranks, §npc-notables): masters are guild
//! masters and prosperous owners rather than the default, labourers lead
//! among the farm and woodland folk, gentry and nobles are rare, a hamlet
//! has one civic figure, and every rank agrees with the job and office
//! that made it.
//!
//! World resolution: `$ARDA_TEST_WORLD`, then `<workspace>/out/micro42`
//! when settled, else a MICRO world generated and settled once into
//! `target/arda-people-fixture/micro42`. The notables are recomputed with
//! this build (`arda_people::compute`); nothing is written.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_npc::ranks::{civic_cap, civic_office, job_rank, lifestyle_band, office_rank};
use arda_npc::{JobCategory, Tier};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    let out = root.join("out/micro42");
    if ready(&out) {
        return out;
    }
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

fn share(counts: &BTreeMap<String, usize>, keys: &[&str]) -> f64 {
    let total: usize = counts.values().sum();
    let n: usize = keys.iter().filter_map(|k| counts.get(*k)).sum();
    n as f64 / total.max(1) as f64
}

fn key<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn micro_notables_have_a_plausible_medieval_mix() {
    let dir = world_dir();
    let files = arda_people::SocietyFiles::read(&dir.join("society")).unwrap();
    let built = arda_people::compute(&dir).unwrap();
    let mut ranks: BTreeMap<String, usize> = BTreeMap::new();
    let mut categories: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_tier: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for ((s, stored), soc) in files
        .settlements
        .settlements
        .iter()
        .zip(&built.notables.settlements)
        .zip(&built.society.settlements)
    {
        assert_eq!(stored.settlement_id, s.id);
        let tier: Tier = serde_json::from_value(serde_json::to_value(s.tier).unwrap()).unwrap();
        let mut civic = 0;
        for npc in &stored.npcs {
            let office = soc.roles.iter().find(|r| r.title == npc.job.title);
            // Rank = the job's rank in the tier, raised to the office's.
            let job = job_rank(&npc.job.key, tier).unwrap();
            let expected = office
                .and_then(|r| office_rank(&r.kind, tier).unwrap())
                .map_or(job, |o| o.max(job));
            assert_eq!(
                npc.social_rank, expected,
                "{} ({}) in {}",
                npc.job.title, npc.job.key, s.name
            );
            let [lo, hi] = lifestyle_band(npc.social_rank).unwrap().unwrap();
            assert!(
                (lo..=hi).contains(&npc.lifestyle),
                "{} in {}: {:?} lives {:?}",
                npc.job.title,
                s.name,
                npc.social_rank,
                npc.lifestyle
            );
            if npc.job.category == JobCategory::Government
                || office.is_some_and(|r| civic_office(&r.kind).unwrap())
            {
                civic += 1;
            }
            *ranks.entry(key(&npc.social_rank)).or_default() += 1;
            *categories.entry(key(&npc.job.category)).or_default() += 1;
            *by_tier
                .entry(tier.key().to_string())
                .or_default()
                .entry(key(&npc.job.category))
                .or_default() += 1;
        }
        let cap = usize::try_from(civic_cap(tier).unwrap()).unwrap();
        assert!(
            civic <= cap,
            "{} ({tier:?}, {} people): {civic} civic figures among {:?}",
            s.name,
            s.population,
            stored
                .npcs
                .iter()
                .map(|n| (&n.job.title, &n.job.key))
                .collect::<Vec<_>>()
        );
    }
    let total: usize = ranks.values().sum();
    assert!(total > 1_500, "{total} notables");
    // Master is not the default: tradesfolk lead, masters are a minority,
    // labourers appear, gentry and nobles are rare.
    let master = share(&ranks, &["master"]);
    let trades = share(&ranks, &["tradesfolk"]);
    let labour = share(&ranks, &["labourer"]);
    let high = share(&ranks, &["gentry", "noble"]);
    assert!(master <= 0.40, "masters {master:.2} of {ranks:?}");
    assert!(trades >= 0.40, "tradesfolk {trades:.2} of {ranks:?}");
    assert!((0.05..=0.30).contains(&labour), "labourers {labour:.2}");
    assert!(high <= 0.10, "gentry and nobles {high:.2}");
    // Officials are a small part of the notables; religion is modest.
    let government = share(&categories, &["government"]);
    assert!(government <= 0.12, "government {government:.2}");
    assert!(share(&categories, &["religion"]) <= 0.12);
    // Hamlets live off the land and water; towns and cities off trades.
    let hamlet = &by_tier["hamlet"];
    let land = share(hamlet, &["agriculture", "extraction", "maritime", "labour"]);
    assert!(land >= 0.70, "hamlet land folk {land:.2} of {hamlet:?}");
    for tier in ["town", "city"] {
        let mix = &by_tier[tier];
        let trades = share(mix, &["craft", "trade", "service"]);
        assert!(trades >= 0.45, "{tier} trades {trades:.2} of {mix:?}");
    }
}
