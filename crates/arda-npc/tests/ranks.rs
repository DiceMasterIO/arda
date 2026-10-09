//! Social rank follows the job and the office held (logic/13 §npc-ranks),
//! wealth stays in the rank's lifestyle band, and a settlement's civic
//! figures stay within its tier's cap (§npc-notables).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::input::{BuildingSpec, SettlementProfile};
use arda_npc::ranks::{civic_cap, job_rank, lifestyle_band, office_rank};
use arda_npc::{Generator, JobCategory, NotableSlot, Npc, SocialRank, Tier};
use common::{city, hamlet, port_town, town, village, SEED};

fn slot(settlement: u64, kind: &str, title: &str) -> NotableSlot {
    NotableSlot {
        id: format!("r{settlement}.{kind}"),
        kind: kind.into(),
        title: title.into(),
        building: None,
        given_name: None,
        family_name: None,
        female: None,
    }
}

/// Every inhabitant's rank, lifestyle and "Master" title agree with the
/// tables; `offices` maps a held title to its office kind.
fn check(npc: &Npc, tier: Tier, offices: &[NotableSlot]) {
    let held = offices.iter().find(|s| s.title == npc.job.title);
    let job = job_rank(&npc.job.key, tier).unwrap();
    let office = held.and_then(|s| office_rank(&s.kind, tier).unwrap());
    let expected = office.map_or(job, |o| o.max(job));
    assert_eq!(
        npc.social_rank, expected,
        "{} ({}) in a {tier:?}",
        npc.job.title, npc.job.key
    );
    if npc.job.category != JobCategory::Dependent {
        let [lo, hi] = lifestyle_band(npc.social_rank).unwrap().unwrap();
        assert!(
            (lo..=hi).contains(&npc.lifestyle),
            "{} ranked {:?} lives {:?}",
            npc.job.title,
            npc.social_rank,
            npc.lifestyle
        );
    }
    if held.is_none() && npc.job.title.starts_with("Master ") {
        assert!(
            npc.social_rank >= SocialRank::Master,
            "{} is only {:?}",
            npc.job.title,
            npc.social_rank
        );
    }
}

#[test]
fn every_inhabitant_ranks_by_job_tier_and_band() {
    for (profile, buildings) in [hamlet(), village(), town(), port_town(), city(6_000)] {
        for npc in common::everyone(&profile, &buildings) {
            check(&npc, profile.tier, &[]);
        }
    }
}

#[test]
fn rank_is_tiered_for_owners_and_flat_for_hands() {
    use SocialRank::{Labourer, Master, Tradesfolk};
    for (job, small, large) in [
        ("smith", Tradesfolk, Master),
        ("innkeeper", Tradesfolk, Master),
        ("craft_master", Tradesfolk, Master),
        ("farmer", Labourer, Labourer),
        ("woodcutter", Labourer, Labourer),
        ("tavern_keeper", Tradesfolk, Tradesfolk),
        ("headman", Tradesfolk, Tradesfolk),
        ("reeve", Tradesfolk, Tradesfolk),
    ] {
        assert_eq!(job_rank(job, Tier::Village).unwrap(), small, "{job}");
        assert_eq!(job_rank(job, Tier::City).unwrap(), large, "{job}");
    }
    // A village manor is gentry; a town's lord is noble.
    assert_eq!(job_rank("lord", Tier::Village).unwrap(), SocialRank::Gentry);
    assert_eq!(job_rank("lord", Tier::Town).unwrap(), SocialRank::Noble);
    // An elder ranks with a reeve, whatever they farm.
    assert_eq!(
        office_rank("elder", Tier::Hamlet).unwrap(),
        Some(job_rank("reeve", Tier::Village).unwrap())
    );
}

/// Civic figures among the stored notables: government jobs and holders
/// of civic offices.
fn civic(notables: &[Npc], slots: &[NotableSlot]) -> usize {
    notables
        .iter()
        .filter(|n| {
            n.job.category == JobCategory::Government
                || slots.iter().any(|s| {
                    s.title == n.job.title && arda_npc::ranks::civic_office(&s.kind).unwrap()
                })
        })
        .count()
}

fn notables(
    profile: &SettlementProfile,
    buildings: &[BuildingSpec],
    slots: &[NotableSlot],
) -> Vec<Npc> {
    Generator::with_notables(SEED, profile, buildings, slots)
        .unwrap()
        .population()
        .unwrap()
        .npcs
}

#[test]
fn a_hamlet_has_one_civic_figure() {
    let (profile, buildings) = hamlet();
    assert_eq!(civic_cap(Tier::Hamlet).unwrap(), 1);
    // Without the society layer the tier promotes at most one headman.
    let plain = notables(&profile, &buildings, &[]);
    assert!(civic(&plain, &[]) <= 1, "{plain:#?}");
    // With an elder's office the elder is the only civic figure: no
    // headman or village elder is promoted beside them.
    let slots = [slot(profile.id.0, "elder", "Elder of Thornby")];
    let held = notables(&profile, &buildings, &slots);
    assert_eq!(civic(&held, &slots), 1);
    assert!(held
        .iter()
        .all(|n| n.job.key != "headman" && n.job.key != "elder"));
    let elder = held
        .iter()
        .find(|n| n.job.title == "Elder of Thornby")
        .expect("the elder is stored");
    assert!(elder.social_rank >= SocialRank::Tradesfolk);
    let (lo, hi) = (2, 4);
    assert!((lo..=hi).contains(&held.len()), "{} notables", held.len());
    for n in &held {
        check(n, Tier::Hamlet, &slots);
    }
}

#[test]
fn an_office_stands_in_for_the_official_it_covers() {
    let (profile, buildings) = village();
    let plain = notables(&profile, &buildings, &[]);
    assert_eq!(plain.iter().filter(|n| n.job.key == "reeve").count(), 1);
    let slots = [
        slot(profile.id.0, "reeve", "Reeve of Oakmere"),
        slot(
            profile.id.0,
            "commons_voice",
            "Voice of the Oakmere Commons",
        ),
    ];
    let held = notables(&profile, &buildings, &slots);
    // The office's holder is the reeve; no second reeve is appointed.
    assert!(held.iter().all(|n| n.job.key != "reeve"));
    assert!(held.iter().any(|n| n.job.title == "Reeve of Oakmere"));
    assert!(civic(&held, &slots) <= usize::try_from(civic_cap(Tier::Village).unwrap()).unwrap());
    for n in &held {
        check(n, Tier::Village, &slots);
    }
}
