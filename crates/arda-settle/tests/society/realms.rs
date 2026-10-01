//! Realms and names on the synthetic world: closed borders on rivers and
//! ridges, whole realms, unique names.
use super::*;

#[test]
fn realms_partition_the_land_with_closed_borders() {
    let f = &*FIXTURE;
    let g = &f.grid;
    let realms = &f.society.realms;
    let n = u16::try_from(realms.realms.len()).unwrap();
    assert!(n >= 2, "two or more towns must give two or more realms");
    for i in 0..g.len() {
        let r = realms.map[i];
        if g.is_land(i) {
            assert!((1..=n).contains(&r), "land cell {i} has realm {r}");
        } else {
            assert_eq!(r, 0);
        }
    }
    for s in &f.society.settlements {
        assert_eq!(u64::from(realms.map[s.index(g.width)]), s.realm_id.get());
    }
    // Borders are closed: inside the land every corner where a realm's
    // border polylines end is shared by an even number of ends, so the
    // lines join up; only corners on water or the map edge may end a line.
    let touches_edge = |x: i64, y: i64| {
        [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)]
            .iter()
            .any(|&(cx, cy)| g.at(cx, cy).is_none_or(|j| !g.is_land(j)))
    };
    for r in &realms.realms {
        assert!(!r.name.is_empty());
        let mut ends: std::collections::BTreeMap<[i64; 2], u32> = std::collections::BTreeMap::new();
        for line in &r.borders {
            let (a, b) = (line[0], line[line.len() - 1]);
            if a != b {
                *ends.entry(a).or_default() += 1;
                *ends.entry(b).or_default() += 1;
            }
        }
        for (p, n) in ends {
            assert!(
                n % 2 == 0 || touches_edge(p[0] / 100, p[1] / 100),
                "realm {} border dangles at {p:?}",
                r.id
            );
        }
    }
    let cities: usize = f
        .society
        .settlements
        .iter()
        .filter(|s| s.tier == Tier::City)
        .count();
    assert!(cities <= 2 * realms.realms.len());
}

#[test]
fn borders_snap_to_rivers_and_ridges_and_realms_stay_whole() {
    let f = &*FIXTURE;
    let g = &f.grid;
    let st = &f.society.stats;
    eprintln!(
        "border on line {} (raw {}), of all {}",
        st.border_on_river_or_ridge_pm,
        st.border_on_river_or_ridge_raw_pm,
        st.border_natural_of_all_pm
    );
    assert!(
        st.border_on_river_or_ridge_pm >= 900,
        "only {} per mille of snappable border is on a line",
        st.border_on_river_or_ridge_pm
    );
    assert!(st.border_on_river_or_ridge_pm >= st.border_on_river_or_ridge_raw_pm);
    // Every 8-connected fragment of a realm either holds its seat or
    // touches no other realm (it is alone on its island).
    let map = &f.society.realms.map;
    let seat_cell = |r: u16| {
        let seat = f.society.realms.realms[usize::from(r) - 1].seat;
        f.society
            .settlements
            .iter()
            .find(|s| s.id.get() == seat)
            .unwrap()
            .index(g.width)
    };
    let mut seen = vec![false; g.len()];
    for s in 0..g.len() {
        if map[s] == 0 || seen[s] {
            continue;
        }
        let mut stack = vec![s];
        seen[s] = true;
        let (mut has_seat, mut touches) = (false, false);
        let seat = seat_cell(map[s]);
        while let Some(a) = stack.pop() {
            has_seat |= a == seat;
            for (b, _, _) in g.neighbours8(a) {
                if map[b] == map[a] && !seen[b] {
                    seen[b] = true;
                    stack.push(b);
                } else if map[b] != 0 && map[b] != map[a] {
                    touches = true;
                }
            }
        }
        assert!(has_seat || !touches, "realm {} has an exclave", map[s]);
    }
}

#[test]
fn names_are_unique_within_their_scope() {
    let f = &*FIXTURE;
    let unique = |v: Vec<&str>| {
        let n = v.len();
        let set: BTreeSet<&str> = v.into_iter().collect();
        assert_eq!(set.len(), n);
    };
    unique(
        f.society
            .settlements
            .iter()
            .map(|s| s.name.as_str())
            .collect(),
    );
    unique(
        f.society
            .realms
            .realms
            .iter()
            .map(|r| r.name.as_str())
            .collect(),
    );
    unique(f.society.rivers.iter().map(|r| r.name.as_str()).collect());
    unique(
        f.society
            .mountains
            .iter()
            .map(|m| m.name.as_str())
            .collect(),
    );
    assert!(f
        .society
        .settlements
        .iter()
        .all(|s| !s.name.is_empty() && !s.history.is_empty()));
    assert!(!f.society.rivers.is_empty());
}
