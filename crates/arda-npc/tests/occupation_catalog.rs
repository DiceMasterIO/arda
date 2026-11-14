//! The occupation catalogue (logic/13 §npc-occupations): every job and
//! office key `generate_one` takes, with the tiers and ranks it accepts.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::{
    generate_one, occupations, NpcError, Occupation, OccupationKind, OneRequest, SocialRank, Tier,
};
use std::collections::BTreeSet;

const WORLD: u64 = 42;
const TIERS: [Tier; 4] = [Tier::Hamlet, Tier::Village, Tier::Town, Tier::City];

fn keys(field: &str) -> Vec<String> {
    common::content("occupations.json")[field]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

fn of(kind: OccupationKind) -> Vec<Occupation> {
    occupations()
        .unwrap()
        .into_iter()
        .filter(|o| o.kind == kind)
        .collect()
}

#[test]
fn every_job_then_every_office_once_in_key_order() {
    let all = occupations().unwrap();
    let jobs: Vec<String> = of(OccupationKind::Job).into_iter().map(|o| o.key).collect();
    let offices: Vec<String> = of(OccupationKind::Office)
        .into_iter()
        .map(|o| o.key)
        .collect();
    assert_eq!(jobs, keys("jobs"));
    assert_eq!(offices, keys("offices"));
    assert_eq!(jobs.len(), 96);
    assert_eq!(offices.len(), 21);
    assert_eq!(all.len(), jobs.len() + offices.len());
    assert!(all
        .iter()
        .take(jobs.len())
        .all(|o| o.kind == OccupationKind::Job));
}

#[test]
fn labels_come_from_the_table() {
    let table = common::content("occupations.json");
    for job in of(OccupationKind::Job) {
        let row = &table["jobs"][&job.key];
        assert_eq!(job.label, row["title"].as_str().unwrap(), "{}", job.key);
        assert_eq!(
            job.label_female.as_deref(),
            row["title_female"].as_str(),
            "{}",
            job.key
        );
        assert!(job.category.is_some(), "{}", job.key);
    }
    for office in of(OccupationKind::Office) {
        assert!(!office.label.is_empty(), "{}", office.key);
        assert_eq!(office.label_female, None);
        assert_eq!(office.category, None);
    }
}

/// A job's listed tiers and ranks are exactly what `generate_one` accepts.
#[test]
fn every_listed_job_tier_and_rank_generates_and_no_other_tier_does() {
    for job in of(OccupationKind::Job) {
        assert!(!job.tiers.is_empty(), "{} has no tier", job.key);
        let listed: BTreeSet<Tier> = job.tiers.iter().map(|t| t.tier).collect();
        for t in &job.tiers {
            let req = OneRequest {
                job: Some(job.key.clone()),
                tier: Some(t.tier),
                rank: Some(t.rank),
                ..OneRequest::default()
            };
            let npc = generate_one(WORLD, &req).unwrap_or_else(|e| panic!("{req:?}: {e}"));
            assert_eq!(npc.job.key, job.key);
            assert_eq!(npc.social_rank, t.rank, "{} in {:?}", job.key, t.tier);
        }
        for tier in TIERS.into_iter().filter(|t| !listed.contains(t)) {
            let req = OneRequest {
                job: Some(job.key.clone()),
                tier: Some(tier),
                ..OneRequest::default()
            };
            assert!(
                matches!(generate_one(WORLD, &req), Err(NpcError::Unreachable(_))),
                "{} is not listed in {tier:?} but generates there",
                job.key
            );
        }
    }
}

/// An office is held in every tier, by someone of at least its rank.
#[test]
fn every_office_generates_in_every_tier_at_or_above_its_rank() {
    for office in of(OccupationKind::Office) {
        let tiers: Vec<Tier> = office.tiers.iter().map(|t| t.tier).collect();
        assert_eq!(tiers, TIERS, "{}", office.key);
        for t in &office.tiers {
            let req = OneRequest {
                office: Some(office.key.clone()),
                tier: Some(t.tier),
                ..OneRequest::default()
            };
            let npc = generate_one(WORLD, &req).unwrap_or_else(|e| panic!("{req:?}: {e}"));
            assert!(npc.notable, "{} holder is not a notable", office.key);
            assert!(
                npc.social_rank >= t.rank,
                "{} in {:?}: {:?} below {:?}",
                office.key,
                t.tier,
                npc.social_rank,
                t.rank
            );
        }
    }
}

#[test]
fn ranks_follow_the_table() {
    let smith = of(OccupationKind::Job)
        .into_iter()
        .find(|o| o.key == "smith")
        .unwrap();
    assert!(smith.tiers.iter().all(|t| t.rank >= SocialRank::Tradesfolk));
    let lord = of(OccupationKind::Office)
        .into_iter()
        .find(|o| o.key == "lord")
        .unwrap();
    let rank = |tier| lord.tiers.iter().find(|t| t.tier == tier).unwrap().rank;
    assert_eq!(rank(Tier::Hamlet), SocialRank::Gentry);
    assert_eq!(rank(Tier::City), SocialRank::Noble);
}
