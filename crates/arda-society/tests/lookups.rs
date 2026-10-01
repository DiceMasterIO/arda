//! Review round 2 #29: the history lookups that replaced per-settlement and
//! per-pair scans answer exactly what those scans answered, record for
//! record and in the same order, so the output stays byte-identical.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_society::history::index::HistoryIndex;
use arda_society::history::History;

fn ids<T: PartialEq>(got: &[&T], want: &[&T]) -> bool {
    got.len() == want.len() && got.iter().zip(want).all(|(a, b)| std::ptr::eq(*a, *b))
}

fn check(h: &History, settlements: &[u64], realms: &[u64]) {
    let ix = HistoryIndex::new(h);
    for &s in settlements {
        let got: Vec<_> = ix.events_of(h, s).collect();
        let want: Vec<_> = h
            .events
            .iter()
            .filter(|e| e.settlements.contains(&s))
            .collect();
        assert!(ids(&got, &want), "events of {s}");
        let got: Vec<_> = ix.shifts_of(h, s).collect();
        let want: Vec<_> = h
            .border_shifts
            .iter()
            .filter(|b| b.settlement == s)
            .collect();
        assert!(ids(&got, &want), "shifts of {s}");
        let first = h.border_shifts.iter().find(|b| b.settlement == s);
        assert_eq!(
            ix.first_shift(h, s).map(std::ptr::from_ref),
            first.map(std::ptr::from_ref)
        );
        let ruin = h.ruins.iter().find(|r| r.near == s);
        assert_eq!(
            ix.ruin_near(h, s).map(std::ptr::from_ref),
            ruin.map(std::ptr::from_ref)
        );
    }
    for &r in realms {
        let got: Vec<_> = ix.realm_events(h, r).collect();
        let want: Vec<_> = h.events.iter().filter(|e| e.realms.contains(&r)).collect();
        assert!(ids(&got, &want), "events of realm {r}");
        let got: Vec<_> = ix.wars_of(h, r).collect();
        let want: Vec<_> = h.wars.iter().filter(|w| w.realms.contains(&r)).collect();
        assert!(ids(&got, &want), "wars of realm {r}");
        for &o in realms.iter().filter(|&&o| o > r) {
            let got: Vec<_> = ix.wars_between(h, o, r).collect();
            let want: Vec<_> = h
                .wars
                .iter()
                .filter(|w| w.realms == [r, o] || w.realms == [o, r])
                .collect();
            assert!(ids(&got, &want), "wars {r}–{o}");
            let taken = h
                .border_shifts
                .iter()
                .filter(|b| {
                    (b.from_realm == r && b.to_realm == o) || (b.from_realm == o && b.to_realm == r)
                })
                .count();
            assert_eq!(ix.shifts_between(r, o), taken, "shifts {r}–{o}");
        }
    }
    for w in &h.wars {
        let first = h.wars.iter().find(|x| x.event == w.event).unwrap();
        assert!(std::ptr::eq(ix.war_of_event(h, w.event).unwrap(), first));
    }
    for r in &h.ruins {
        if let Some(road) = r.road {
            let first = h.ruins.iter().find(|x| x.road == Some(road)).unwrap();
            assert!(std::ptr::eq(ix.ruin_on_road(h, road).unwrap(), first));
        }
    }
}

#[test]
fn history_lookups_equal_the_scans_they_replace() {
    let mut events = 0;
    for seed in common::SEEDS {
        let (world, s) = common::build(seed);
        let settlements: Vec<u64> = world.settlements.iter().map(|x| x.id).collect();
        let realms: Vec<u64> = s.realms.iter().map(|r| r.state.id).collect();
        check(&s.history, &settlements, &realms);
        events += s.history.events.len() + s.history.border_shifts.len();
    }
    assert!(events > 100, "the fixtures have a history to index");
}
