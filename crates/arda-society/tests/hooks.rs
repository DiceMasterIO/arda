//! Campaign hooks, history hooks and NPC slots: counts, filled text, SRD
//! 5.1 stat names and stable ids.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_society::hooks::{MAX_HOOKS, MIN_HOOKS};
use arda_society::simulate_society;
use std::collections::BTreeSet;

const SRD_BLOCKS: [&str; 21] = [
    "acolyte",
    "bandit",
    "bandit captain",
    "berserker",
    "commoner",
    "cult fanatic",
    "cultist",
    "druid",
    "gladiator",
    "guard",
    "knight",
    "mage",
    "noble",
    "priest",
    "scout",
    "spy",
    "thug",
    "tribal warrior",
    "veteran",
    "archmage",
    "assassin",
];
const SRD_CLASSES: [&str; 12] = [
    "barbarian",
    "bard",
    "cleric",
    "druid",
    "fighter",
    "monk",
    "paladin",
    "ranger",
    "rogue",
    "sorcerer",
    "warlock",
    "wizard",
];

fn filled(text: &str) -> bool {
    !text.is_empty() && !text.contains('{') && !text.contains('}')
}

#[test]
fn every_settlement_and_realm_has_three_to_five_hooks() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        let mut ids = BTreeSet::new();
        let all = s
            .settlements
            .iter()
            .map(|x| &x.hooks)
            .chain(s.realms.iter().map(|r| &r.hooks));
        for hooks in all {
            assert!(
                (MIN_HOOKS..=MAX_HOOKS).contains(&hooks.len()),
                "{} hooks",
                hooks.len()
            );
            for h in hooks {
                assert!(ids.insert(h.id.clone()), "duplicate hook id {}", h.id);
                assert!(
                    filled(&h.title) && filled(&h.text),
                    "unfilled: {} / {}",
                    h.title,
                    h.text
                );
                assert!(!h.refs.is_empty());
            }
        }
    }
}

#[test]
fn every_settlement_has_two_to_four_history_hooks() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        for st in &s.settlements {
            assert!(
                (2..=4).contains(&st.history_hooks.len()),
                "{}: {}",
                st.name,
                st.history_hooks.len()
            );
            for h in &st.history_hooks {
                assert!(filled(&h.text), "{}", h.text);
            }
        }
    }
}

#[test]
fn all_generated_text_is_filled() {
    let (_, s) = common::build(42);
    let mut names = BTreeSet::new();
    for f in s.settlements.iter().flat_map(|x| x.factions.iter()) {
        assert!(
            names.insert(f.name.clone()),
            "duplicate faction name {}",
            f.name
        );
    }
    for e in &s.history.events {
        assert!(
            filled(&e.title) && filled(&e.text),
            "{} / {}",
            e.title,
            e.text
        );
    }
    for st in &s.settlements {
        for f in &st.factions {
            assert!(
                filled(&f.name) && f.goals.iter().all(|g| filled(g)),
                "{}",
                f.name
            );
        }
        for r in &st.faction_relations {
            assert!(filled(&r.reason), "{}", r.reason);
        }
        for r in &st.roles {
            assert!(filled(&r.title), "{}", r.title);
        }
    }
    for r in &s.realms {
        assert!(r
            .state
            .vassals
            .iter()
            .all(|v| v.grievances.iter().all(|g| filled(g))));
        for rel in &s.relations {
            assert!(rel.reasons.iter().all(|g| filled(g)));
        }
    }
}

#[test]
fn roles_use_srd_names_and_real_buildings() {
    for seed in common::SEEDS {
        let (_, s) = common::build(seed);
        for st in &s.settlements {
            let mut ids = BTreeSet::new();
            assert!(
                st.roles
                    .iter()
                    .any(|r| ["ruler", "lord", "reeve", "elder"].contains(&r.kind.as_str())),
                "{} has no head",
                st.name
            );
            for r in &st.roles {
                assert!(ids.insert(r.id.clone()), "duplicate role {}", r.id);
                assert!(r.id.starts_with(&format!("r{}.", st.id)));
                assert!(
                    r.stat_block.is_some() || r.class.is_some(),
                    "{} has no stats",
                    r.id
                );
                if let Some(b) = &r.stat_block {
                    assert!(SRD_BLOCKS.contains(&b.as_str()), "non-SRD block {b}");
                }
                if let Some(c) = &r.class {
                    assert!(SRD_CLASSES.contains(&c.class.as_str()) && (1..=20).contains(&c.level));
                }
                let b = r.building.expect("every role has a building");
                assert!(st.buildings.iter().any(|x| x.id == b));
            }
            let has = |k: &str| st.buildings.iter().any(|b| b.function == k);
            let role = |k: &str| st.roles.iter().any(|r| r.kind == k);
            assert_eq!(has("temple"), role("high_priest"), "{}", st.name);
            assert_eq!(
                has("barracks") || has("guardhouse") || has("keep"),
                role("captain"),
                "{}",
                st.name
            );
            for f in &st.factions {
                assert!(
                    role(f.leader_role.rsplit('.').next().unwrap()),
                    "{} leader slot",
                    f.id
                );
            }
        }
    }
}

#[test]
fn role_ids_depend_only_on_the_world() {
    let (world, a) = common::build(42);
    let b = simulate_society(4242, &world).unwrap();
    let ids = |s: &arda_society::Society| -> BTreeSet<String> {
        s.settlements
            .iter()
            .flat_map(|x| x.roles.iter().map(|r| r.id.clone()))
            .collect()
    };
    assert_eq!(ids(&a), ids(&b));
}
