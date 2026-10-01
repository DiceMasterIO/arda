//! Local invariants of decoded or to-be-encoded area objects: course
//! geometry, lake ownership, channel edges and copied global authority.

use super::*;
use crate::formats::hydrology::FixedRecord;
use crate::hydrology::{BasinId, ReachId};
use crate::objects::{AreaObjects, Terminus};
use std::collections::BTreeMap;

pub(super) fn validate_local(o: &AreaObjects) -> Result<()> {
    for pair in o.rivers.windows(2) {
        require(
            pair[0].id < pair[1].id,
            "river IDs are not unique ascending",
        )?;
    }
    for pair in o.lakes.windows(2) {
        require(pair[0].id < pair[1].id, "lake IDs are not unique ascending")?;
    }
    for pair in o.channel_edges.windows(2) {
        require(
            pair[0].key() < pair[1].key(),
            "channel edges are not unique canonical pairs",
        )?;
    }
    let global_reaches: BTreeMap<_, _> = o.global.reaches.iter().map(|r| (r.id, r)).collect();
    let global_lakes: BTreeMap<_, _> = o.global.lakes.iter().map(|l| (l.basin, l)).collect();
    let local_index: BTreeMap<_, _> = o
        .rivers
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id, i))
        .collect();
    let mut visits = vec![0_usize; CELLS];
    let mut next = vec![None; o.rivers.len()];
    for (i, r) in o.rivers.iter().enumerate() {
        require(
            r.id > 0 && r.order > 0 && r.width_dm > 0 && r.discharge.raw() >= 40,
            "invalid local river fields",
        )?;
        let g = global_reaches
            .get(&r.global_id)
            .ok_or(ObjectsFormatError::Invalid(
                "river lacks copied global reach",
            ))?;
        require(
            r.discharge == g.mean_discharge,
            "local river discharge disagrees with global authority",
        )?;
        require(
            !r.course.is_empty() && r.course.len() <= CELLS,
            "invalid local course length",
        )?;
        for &c in &r.course {
            require(visits[c.index()] != i + 1, "river course repeats a cell")?;
            visits[c.index()] = i + 1;
        }
        for p in r.course.windows(2) {
            require(
                cell_neighbor(p[0], p[1]),
                "river course is not contiguous D8",
            )?;
        }
        if g.id.is_point() {
            require(
                r.course.len() == 1
                    && u32::from(r.course[0].x()) == g.from.x % 512
                    && u32::from(r.course[0].y()) == g.from.y % 512,
                "point course does not own its source cell",
            )?;
            require(
                matches!(
                    (g.receiving, r.ends),
                    (crate::hydrology::ReceivingAccount::Lake(_), Terminus::Basin)
                        | (
                            crate::hydrology::ReceivingAccount::DomainExport,
                            Terminus::OffTile
                        )
                ),
                "point course terminus disagrees with global authority",
            )?;
        }
        if r.ends == Terminus::Basin {
            let crate::hydrology::ReceivingAccount::Lake(basin) = g.receiving else {
                return Err(ObjectsFormatError::Invalid(
                    "basin terminus lacks physical basin destination",
                ));
            };
            require(
                o.global
                    .catchments
                    .iter()
                    .any(|owner| owner.basin == Some(basin) && owner.representative_lake.is_none()),
                "basin terminus lacks copied dry physical basin",
            )?;
        }
        if r.ends == Terminus::Divergence {
            let crate::hydrology::ReceivingAccount::Junction(junction) = g.receiving else {
                return Err(ObjectsFormatError::Invalid(
                    "divergence lacks junction destination",
                ));
            };
            require(
                g.to == junction.cell(),
                "divergence junction disagrees with endpoint",
            )?;
            require(
                r.course.len() == 1
                    && u32::from(r.course[0].x()) == g.from.x % 512
                    && u32::from(r.course[0].y()) == g.from.y % 512,
                "divergence course does not own its source cell",
            )?;
            let base = junction
                .0
                .checked_mul(8)
                .ok_or(ObjectsFormatError::Invalid(
                    "junction reach identity overflow",
                ))?;
            let end = base.checked_add(7).ok_or(ObjectsFormatError::Invalid(
                "junction reach identity overflow",
            ))?;
            let mut outgoing = 0_u8;
            for (_, target) in global_reaches.range(ReachId(base)..=ReachId(end)) {
                require(
                    target.from == junction.cell() && target.annual_volume.0 > 0,
                    "divergence outgoing reach is not a real source at its junction",
                )?;
                outgoing += 1;
            }
            let point_id = ReachId::point(junction.cell()).ok_or(ObjectsFormatError::Invalid(
                "junction point identity overflow",
            ))?;
            if let Some(target) = global_reaches.get(&point_id) {
                require(
                    target.from == junction.cell()
                        && target.receiving == crate::hydrology::ReceivingAccount::DomainExport
                        && target.annual_volume.0 > 0,
                    "divergence point is not a positive domain export at its junction",
                )?;
                outgoing += 1;
            }
            require(
                (2..=9).contains(&outgoing),
                "divergence lacks multiple copied outgoing reaches",
            )?;
        }
        match (r.ends, r.feeds) {
            (Terminus::Junction, Some(id)) => {
                let j = *local_index.get(&id).ok_or(ObjectsFormatError::Invalid(
                    "feeds names a missing local river",
                ))?;
                require(j != i, "river feeds itself")?;
                let tail = r
                    .course
                    .last()
                    .ok_or(ObjectsFormatError::Invalid("empty course"))?;
                let head = o.rivers[j]
                    .course
                    .first()
                    .ok_or(ObjectsFormatError::Invalid("empty downstream course"))?;
                require(
                    tail == head || cell_neighbor(*tail, *head),
                    "local feeds connection is not adjacent",
                )?;
                next[i] = Some(j);
            }
            (Terminus::Junction, None) => {
                return Err(ObjectsFormatError::Invalid(
                    "junction lacks local feeds target",
                ))
            }
            (_, None) => {}
            (_, Some(_)) => {
                return Err(ObjectsFormatError::Invalid(
                    "non-junction carries local feeds target",
                ))
            }
        }
    }
    // Functional-graph colouring is linear and nonrecursive; shared endpoints are allowed.
    let mut state = vec![0_u8; o.rivers.len()];
    for start in 0..state.len() {
        if state[start] != 0 {
            continue;
        }
        let mut chain = Vec::new();
        let mut at = Some(start);
        while let Some(i) = at {
            if state[i] == 2 {
                break;
            }
            require(state[i] != 1, "local river feeds cycle")?;
            state[i] = 1;
            chain.push(i);
            at = next[i];
        }
        for i in chain {
            state[i] = 2;
        }
    }
    let mut lake_owner = vec![false; CELLS];
    let mut lake_counts = BTreeMap::<BasinId, usize>::new();
    for l in &o.lakes {
        require(
            l.id > 0 && !l.cells.is_empty(),
            "invalid local lake identity or membership",
        )?;
        let g = global_lakes
            .get(&l.global_id)
            .ok_or(ObjectsFormatError::Invalid("lake lacks copied global lake"))?;
        require(
            l.surface == g.surface,
            "local lake level disagrees with exact global surface",
        )?;
        let max_depth = i64::from(g.surface.raw()) - i64::from(g.deepest_bed.raw());
        require(
            max_depth >= 0 && i64::from(l.depth_mm) <= max_depth,
            "local depth exceeds global maximum",
        )?;
        if l.outlet.is_some() {
            require(
                g.annual_outflow.0 > 0 && g.outlet.is_some(),
                "global lake without supported annual outflow has local outlet",
            )?;
        }
        for p in l.cells.windows(2) {
            require(
                p[0].index() < p[1].index(),
                "lake members are not unique row-major cells",
            )?;
        }
        for c in &l.cells {
            require(!lake_owner[c.index()], "local lake memberships overlap")?;
            lake_owner[c.index()] = true;
        }
        let n = lake_counts.entry(l.global_id).or_default();
        *n = n
            .checked_add(l.cells.len())
            .ok_or(ObjectsFormatError::Overflow)?;
        require(
            *n <= usize::try_from(g.submerged_cells).map_err(|_| ObjectsFormatError::Overflow)?,
            "local fragments exceed global wet membership",
        )?;
    }
    Ok(())
}
