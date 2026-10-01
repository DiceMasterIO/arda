//! Divergent branches and point-local courses: round trips and the
//! dry-basin semantics they keep.

use super::*;

#[test]
fn divergence_roundtrips_real_branches_without_false_single_feed_or_recursive_context() {
    let mut o = sample();
    o.lakes.clear();
    o.global.lakes.clear();
    o.global.catchments[0].representative_lake = None;
    o.rivers[0].ends = Terminus::Divergence;
    o.rivers[0].course.truncate(1);
    let at = o.global.reaches[0].to;
    o.global.reaches[0].receiving =
        ReceivingAccount::Junction(crate::hydrology::JunctionId::at(at));
    let template = o.global.reaches[0].clone();
    for to in [GlobalCell { x: 3, y: 1 }, GlobalCell { x: 3, y: 2 }] {
        let mut branch = template.clone();
        branch.id = ReachId::from_step(at, to).unwrap();
        branch.from = at;
        branch.to = to;
        branch.annual_volume.0 /= 2;
        branch.mean_discharge = DischargeMilli::new(template.mean_discharge.raw() / 2);
        // The next junction is external to this bounded context. Its branches
        // need not be recursively copied merely to describe this divergence.
        branch.receiving = ReceivingAccount::Junction(crate::hydrology::JunctionId::at(to));
        o.global.reaches.push(branch);
    }
    o.global.reaches.sort_by_key(|r| r.id);
    assert_eq!(Terminus::from_u8(5), Some(Terminus::Divergence));
    let bytes = encode_objects(&o).unwrap();
    assert_eq!(decode(&bytes).unwrap(), o);
    let mut bad = o.clone();
    bad.rivers[0].feeds = Some(70000);
    assert!(encode_objects(&bad).is_err());
    let mut bad = o.clone();
    bad.global.reaches.pop();
    assert!(encode_objects(&bad).is_err());
    let mut bad = o.clone();
    bad.global.reaches[0].receiving = ReceivingAccount::Sea;
    assert!(encode_objects(&bad).is_err());
    let mut bad = o.clone();
    bad.rivers[0].course[0] = cc(2, 1);
    assert!(encode_objects(&bad).is_err());
    let mut bad = o.clone();
    bad.global.reaches[1].from.x += 1;
    assert!(encode_objects(&bad).is_err());
    let mut bad = o.clone();
    bad.global.reaches[1].annual_volume = Litres(0);
    bad.global.reaches[1].mean_discharge = DischargeMilli::new(0);
    assert!(encode_objects(&bad).is_err());
    let mut bad = o.clone();
    bad.global.reaches[2].id = bad.global.reaches[1].id;
    assert!(encode_objects(&bad).is_err());
    // Eight actual D8 branches plus one distinct physical exterior point fit.
    let mut full = o.clone();
    full.global.reaches.truncate(1);
    for (dx, dy) in [
        (-1_i32, -1_i32),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        let to = GlobalCell {
            x: u32::try_from(i64::from(at.x) + i64::from(dx)).unwrap(),
            y: u32::try_from(i64::from(at.y) + i64::from(dy)).unwrap(),
        };
        let mut branch = o.global.reaches[1].clone();
        branch.id = ReachId::from_step(at, to).unwrap();
        branch.to = to;
        branch.receiving = ReceivingAccount::Sea;
        full.global.reaches.push(branch);
    }
    full.global.reaches.sort_by_key(|r| r.id);
    assert!(encode_objects(&full).is_ok());
    let mut point = full.global.reaches[1].clone();
    point.id = ReachId::point(at).unwrap();
    point.to = at;
    point.receiving = ReceivingAccount::DomainExport;
    point.annual_volume = Litres(1);
    point.mean_discharge = DischargeMilli::new(0);
    full.global.reaches.push(point.clone());
    full.global.reaches.sort_by_key(|r| r.id);
    let bytes = encode_objects(&full).unwrap();
    assert_eq!(decode(&bytes).unwrap(), full);
    let mut bad = full.clone();
    bad.global.reaches.last_mut().unwrap().receiving = ReceivingAccount::Lake(BasinId(55));
    assert!(encode_objects(&bad).is_err());
    let mut bad = full.clone();
    bad.global.reaches.push(point);
    assert!(encode_objects(&bad).is_err());
    // A below-threshold domain exit may be the second branch by itself.
    let mut pair = full.clone();
    pair.global
        .reaches
        .retain(|g| g.id == template.id || g.id == full.global.reaches[1].id || g.id.is_point());
    assert!(encode_objects(&pair).is_ok());
}
#[test]
fn point_local_course_owns_exact_terminal_and_keeps_dry_basin_semantics() {
    let mut o = sample();
    o.lakes.clear();
    o.global.lakes.clear();
    o.global.catchments[0].representative_lake = None;
    let g = &mut o.global.reaches[0];
    g.id = ReachId::point(g.from).unwrap();
    g.to = g.from;
    g.receiving = ReceivingAccount::Lake(BasinId(99));
    o.rivers[0].global_id = g.id;
    o.rivers[0].course.truncate(1);
    o.rivers[0].ends = Terminus::Basin;
    assert_eq!(decode(&encode_objects(&o).unwrap()).unwrap(), o);
    let mut bad = o.clone();
    bad.rivers[0].course[0] = cc(1, 0);
    assert!(encode_objects(&bad).is_err());
    let mut bad = o.clone();
    bad.rivers[0].ends = Terminus::Lake;
    assert!(encode_objects(&bad).is_err());
    o.global.reaches[0].receiving = ReceivingAccount::DomainExport;
    o.rivers[0].ends = Terminus::OffTile;
    assert_eq!(decode(&encode_objects(&o).unwrap()).unwrap(), o);
    o.rivers[0].ends = Terminus::Sea;
    assert!(encode_objects(&o).is_err());
}
