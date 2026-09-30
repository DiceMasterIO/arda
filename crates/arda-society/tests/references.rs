//! Every reference resolves: hooks, events, history hooks, roles, factions,
//! realms and relations all point at entities that exist.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_society::refs::EntityRef;
use arda_society::Society;
use std::collections::BTreeSet;

fn resolves(s: &Society, r: &EntityRef) -> bool {
    let sid = |id: u64| s.settlements.iter().any(|x| x.id == id);
    match r {
        EntityRef::Settlement { id } => sid(*id),
        EntityRef::Realm { id } => s.realms.iter().any(|x| x.state.id == *id),
        EntityRef::Road { .. } => true, // checked against the world below
        EntityRef::Building { settlement, id } => s
            .settlements
            .iter()
            .any(|x| x.id == *settlement && x.buildings.iter().any(|b| b.id == *id)),
        EntityRef::Faction { id } => s
            .settlements
            .iter()
            .any(|x| x.factions.iter().any(|f| &f.id == id)),
        EntityRef::Role { id } => s
            .settlements
            .iter()
            .any(|x| x.roles.iter().any(|f| &f.id == id)),
        EntityRef::Ruin { id } => s.history.ruins.iter().any(|x| x.id == *id),
        EntityRef::Event { id } => s.history.events.iter().any(|x| x.id == *id),
        EntityRef::Dynasty { id } => s.history.dynasties.iter().any(|x| &x.id == id),
        EntityRef::Good { key } => s.economy.goods.iter().any(|g| &g.good == key),
    }
}

fn all_refs(s: &Society) -> Vec<EntityRef> {
    let mut v = Vec::new();
    for st in &s.settlements {
        for h in &st.hooks {
            v.extend(h.refs.iter().cloned());
        }
        for h in &st.history_hooks {
            v.extend(h.refs.iter().cloned());
        }
        v.push(EntityRef::Event {
            id: st.founding_event,
        });
        for f in &st.factions {
            v.push(EntityRef::role(&f.leader_role));
            if let Some(b) = f.seat_building {
                v.push(EntityRef::Building {
                    settlement: st.id,
                    id: b,
                });
            }
        }
        for r in &st.roles {
            if let Some(b) = r.building {
                v.push(EntityRef::Building {
                    settlement: st.id,
                    id: b,
                });
            }
            if let Some(f) = &r.faction {
                v.push(EntityRef::faction(f));
            }
        }
    }
    for r in &s.realms {
        v.extend(r.hooks.iter().flat_map(|h| h.refs.iter().cloned()));
        v.push(EntityRef::role(&r.state.ruler.role));
        v.push(EntityRef::Dynasty {
            id: r.state.ruler.dynasty.clone(),
        });
        v.push(EntityRef::settlement(r.state.seat));
        for m in &r.state.members {
            v.push(EntityRef::settlement(*m));
        }
        for x in &r.state.vassals {
            v.push(EntityRef::role(&x.role));
            v.push(EntityRef::settlement(x.settlement));
        }
    }
    for e in &s.history.events {
        v.extend(e.refs.iter().cloned());
        v.extend(e.settlements.iter().map(|&i| EntityRef::settlement(i)));
        v.extend(e.realms.iter().map(|&i| EntityRef::realm(i)));
        v.extend(e.cause.map(|id| EntityRef::Event { id }));
    }
    for w in &s.history.wars {
        v.push(EntityRef::Event { id: w.event });
    }
    for r in &s.history.ruins {
        v.push(EntityRef::Event { id: r.event });
        v.push(EntityRef::settlement(r.near));
        v.extend(r.cause_event.map(|id| EntityRef::Event { id }));
    }
    for x in &s.history.reigns {
        v.push(EntityRef::Dynasty {
            id: x.dynasty_id.clone(),
        });
    }
    for r in &s.relations {
        v.extend(r.wars.iter().map(|&id| EntityRef::Event { id }));
        v.extend(r.border_roads.iter().map(|&id| EntityRef::road(id)));
    }
    v
}

#[test]
fn every_reference_resolves() {
    for seed in common::SEEDS {
        let (world, s) = common::build(seed);
        let roads: BTreeSet<u64> = world.roads.iter().map(|r| r.id).collect();
        let refs = all_refs(&s);
        assert!(refs.len() > 500);
        for r in &refs {
            assert!(resolves(&s, r), "dangling {r:?} (seed {seed})");
            if let EntityRef::Road { id } = r {
                assert!(roads.contains(id), "unknown road {id}");
            }
        }
    }
}

#[test]
fn ruler_and_vassal_slots_carry_their_history_names() {
    let (_, s) = common::build(42);
    for r in &s.realms {
        let seat = s.settlements.iter().find(|x| x.id == r.state.seat).unwrap();
        let slot = seat
            .roles
            .iter()
            .find(|x| x.id == r.state.ruler.role)
            .unwrap();
        assert_eq!(slot.kind, "ruler");
        assert_eq!(
            slot.family_name.as_deref(),
            Some(r.state.ruler.house.as_str())
        );
        for v in &r.state.vassals {
            let st = s.settlements.iter().find(|x| x.id == v.settlement).unwrap();
            let slot = st.roles.iter().find(|x| x.id == v.role).unwrap();
            assert_eq!(slot.given_name.as_deref(), Some(v.given.as_str()));
        }
    }
}
