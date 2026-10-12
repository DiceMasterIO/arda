//! One NPC from a few optional inputs (logic/13 §npc-generate-one): every
//! job and ancestry is reachable, filters hold, the same request is the
//! same person and refusals name the valid values.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::sheet::SheetKind;
use arda_npc::{
    generate_one, sample, NpcError, OneRequest, SettlementFunction as S, Sex, SocialRank, Tier,
};
use std::collections::BTreeSet;

const WORLD: u64 = 42;

fn one(req: &OneRequest) -> arda_npc::Npc {
    generate_one(WORLD, req).unwrap_or_else(|e| panic!("{req:?}: {e}"))
}

fn keys(file: &str, field: &str) -> Vec<String> {
    common::content(file)[field]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

fn sane_sheet(npc: &arda_npc::Npc) {
    let s = &npc.sheet;
    for i in 0..6 {
        assert_eq!(
            s.modifiers[i],
            common::modifier(s.abilities[i]),
            "{}",
            npc.job.key
        );
    }
    assert_eq!(s.saves.len(), 6);
    assert!(s.hit_points > 0 && s.armor_class >= 10, "{}", npc.job.key);
    if let SheetKind::Class { level, .. } = &s.kind {
        assert!((1..=20).contains(level));
    }
}

#[test]
fn every_job_is_reachable() {
    let jobs = keys("occupations.json", "jobs");
    assert_eq!(jobs.len(), 96);
    for job in jobs {
        let npc = one(&OneRequest {
            job: Some(job.clone()),
            ..OneRequest::default()
        });
        assert_eq!(npc.job.key, job);
        sane_sheet(&npc);
    }
}

#[test]
fn every_ancestry_is_reachable() {
    let races = common::srd("races.json");
    let keys: BTreeSet<&str> = races
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap())
        .collect();
    assert!(keys.len() >= 9);
    for key in keys {
        let npc = one(&OneRequest {
            ancestry: Some(key.to_owned()),
            job: Some("smith".into()),
            ..OneRequest::default()
        });
        assert_eq!(npc.ancestry, key);
        assert_eq!(npc.job.key, "smith");
    }
}

#[test]
fn an_office_makes_a_notable_with_its_title() {
    let npc = one(&OneRequest {
        office: Some("reeve".into()),
        ..OneRequest::default()
    });
    assert!(npc.notable);
    assert_eq!(npc.job.title, "Reeve");
    for office in keys("occupations.json", "offices") {
        for sex in [Sex::Female, Sex::Male] {
            let npc = one(&OneRequest {
                office: Some(office.clone()),
                sex: Some(sex),
                ..OneRequest::default()
            });
            assert!(npc.notable, "{office}");
            assert_eq!(npc.sex, sex, "{office}");
            assert!(!npc.job.title.is_empty());
        }
    }
}

#[test]
fn rank_filters_hold() {
    for tier in [Tier::Hamlet, Tier::Village, Tier::Town, Tier::City] {
        for rank in [
            SocialRank::Dependent,
            SocialRank::Labourer,
            SocialRank::Tradesfolk,
            SocialRank::Master,
            SocialRank::Gentry,
            SocialRank::Noble,
        ] {
            let req = OneRequest {
                rank: Some(rank),
                tier: Some(tier),
                ..OneRequest::default()
            };
            // A lord ranks gentry below a town (logic/13 §npc-ranks), so
            // no job makes a noble in a hamlet or village.
            let small = matches!(tier, Tier::Hamlet | Tier::Village);
            match generate_one(WORLD, &req) {
                Ok(npc) => assert_eq!(npc.social_rank, rank, "{tier:?}"),
                Err(NpcError::Unreachable(m)) if small && rank == SocialRank::Noble => {
                    assert!(m.contains("valid:"), "{m}");
                }
                Err(e) => panic!("{tier:?} {rank:?}: {e}"),
            }
        }
    }
    let npc = one(&OneRequest {
        job: Some("smith".into()),
        rank: Some(SocialRank::Tradesfolk),
        ..OneRequest::default()
    });
    assert_eq!(npc.social_rank, SocialRank::Tradesfolk);
}

#[test]
fn sex_notable_and_culture_filters_hold() {
    for seed in 0..6 {
        for sex in [Sex::Female, Sex::Male] {
            let npc = one(&OneRequest {
                seed,
                sex: Some(sex),
                job: Some("innkeeper".into()),
                ..OneRequest::default()
            });
            assert_eq!(npc.sex, sex);
        }
        for notable in [true, false] {
            let npc = one(&OneRequest {
                seed,
                notable: Some(notable),
                ..OneRequest::default()
            });
            assert_eq!(npc.notable, notable);
        }
    }
    let npc = one(&OneRequest {
        culture: Some("highland".into()),
        ..OneRequest::default()
    });
    assert_ne!(npc.job.key, "child", "a bare request is a working adult");
}

#[test]
fn same_request_same_bytes_and_salts_differ() {
    let req = OneRequest {
        seed: 7,
        job: Some("smith".into()),
        ancestry: Some("dwarf".into()),
        ..OneRequest::default()
    };
    let a = serde_json::to_string(&one(&req)).unwrap();
    let b = serde_json::to_string(&one(&req)).unwrap();
    assert_eq!(a, b);
    let names: BTreeSet<String> = (0..8)
        .map(|seed| {
            let npc = one(&OneRequest {
                seed,
                ..req.clone()
            });
            format!("{} {}", npc.name.full(), npc.id)
        })
        .collect();
    assert!(names.len() >= 7, "{names:?}");
    assert_ne!(
        serde_json::to_string(&generate_one(WORLD + 1, &req).unwrap()).unwrap(),
        a,
        "the world seed is part of the key"
    );
}

#[test]
fn results_point_nowhere() {
    let npc = one(&OneRequest {
        job: Some("journeyman_smith".into()),
        ..OneRequest::default()
    });
    assert!(npc.relationships.is_empty());
    assert_eq!(npc.employer, None);
    assert_eq!(npc.workplace_building, None);
}

#[test]
fn a_borrowed_settlement_lends_its_context() {
    let mut profile = sample::profile(
        9,
        "Saltmarch",
        Tier::Town,
        2_400,
        &[S::Port],
        130,
        "coastal",
    );
    profile.ancestry_mix = Some([("halfling".to_owned(), 1)].into_iter().collect());
    let npc = one(&OneRequest {
        settlement: Some(profile.clone()),
        job: Some("mayor".into()),
        ..OneRequest::default()
    });
    assert_eq!(npc.ancestry, "halfling");
    assert_eq!(npc.job.key, "mayor");
    assert_eq!(npc.social_rank, SocialRank::Gentry);
    // An explicit tier overrides the borrowed one, and is checked.
    let err = generate_one(
        WORLD,
        &OneRequest {
            settlement: Some(profile),
            tier: Some(Tier::Village),
            job: Some("mayor".into()),
            ..OneRequest::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, NpcError::Unreachable(_)), "{err}");
}

#[test]
fn refusals_name_the_valid_values() {
    let cases = [
        OneRequest {
            job: Some("astronaut".into()),
            ..OneRequest::default()
        },
        OneRequest {
            ancestry: Some("ogre".into()),
            ..OneRequest::default()
        },
        OneRequest {
            office: Some("emperor".into()),
            ..OneRequest::default()
        },
        OneRequest {
            culture: Some("lunar".into()),
            ..OneRequest::default()
        },
    ];
    for req in cases {
        let err = generate_one(WORLD, &req).unwrap_err();
        assert!(matches!(err, NpcError::UnknownKey { .. }), "{err}");
        assert!(err.to_string().contains("valid:"), "{err}");
    }
    // A smith is tradesfolk, never noble.
    let err = generate_one(
        WORLD,
        &OneRequest {
            job: Some("smith".into()),
            rank: Some(SocialRank::Noble),
            ..OneRequest::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, NpcError::Unreachable(_)), "{err}");
    // A mayor governs a town or city, not a hamlet.
    let err = generate_one(
        WORLD,
        &OneRequest {
            job: Some("mayor".into()),
            tier: Some(Tier::Hamlet),
            ..OneRequest::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, NpcError::Unreachable(_)), "{err}");
    // A farmhand is never a notable: nobody matches.
    let err = generate_one(
        WORLD,
        &OneRequest {
            job: Some("farmhand".into()),
            notable: Some(true),
            ..OneRequest::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, NpcError::Unsatisfiable(_)), "{err}");
}
