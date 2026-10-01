//! Lazy access to a settlement's skeleton (logic/13 §npc-queries, goal 57):
//! who lives in which building at which index, with which job and
//! workplace, without expanding anyone. The service filters and pages on
//! these summaries and expands only the people it returns, so commoners
//! are never materialised settlement- or world-wide (goal 56).

use crate::input::BuildingId;
use crate::npc::{Npc, NpcId};
use crate::{Generator, NpcError};
use std::ops::Range;

/// One inhabitant in skeleton form: enough to filter on without expanding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resident {
    /// Public id, `NpcId::from_parts(settlement, home, index)`.
    pub id: NpcId,
    /// Home building.
    pub home: BuildingId,
    /// Index among the home building's residents.
    pub index: u32,
    /// Occupation key in `occupations.json`, e.g. `smith`.
    pub job: &'static str,
    /// Workplace building, if the job has one.
    pub workplace: Option<BuildingId>,
    /// Whether the person is a stored notable.
    pub notable: bool,
}

impl Generator<'_> {
    /// The inhabitant at skeleton `position` (`0..self.len()`), in skeleton
    /// order: home building id, then resident index.
    #[must_use]
    pub fn resident(&self, position: usize) -> Option<Resident> {
        let plan = &self.plan;
        let p = plan.people.get(position)?;
        Some(Resident {
            id: plan.id_of(position),
            home: plan.buildings.get(p.building)?.id,
            index: p.index,
            job: p.job,
            workplace: p
                .workplace
                .and_then(|w| plan.buildings.get(w))
                .map(|b| b.id),
            notable: p.notable,
        })
    }

    /// Skeleton positions of `building`'s residents, or `None` for a
    /// building the settlement does not have. Residents are contiguous.
    #[must_use]
    pub fn residents_of(&self, building: BuildingId) -> Option<Range<usize>> {
        let plan = &self.plan;
        let b = plan
            .buildings
            .binary_search_by_key(&building, |b| b.id)
            .ok()?;
        Some(*plan.offsets.get(b)?..*plan.offsets.get(b + 1)?)
    }

    /// Whether the settlement has `building`.
    #[must_use]
    pub fn has_building(&self, building: BuildingId) -> bool {
        self.residents_of(building).is_some()
    }

    /// Expands the inhabitant at skeleton `position`.
    ///
    /// # Errors
    /// [`NpcError::UnknownNpc`] past the end; bundled-data errors.
    pub fn npc_at(&self, position: usize) -> Result<Npc, NpcError> {
        if position >= self.plan.people.len() {
            return Err(NpcError::UnknownNpc(NpcId::default()));
        }
        self.plan.npc(position)
    }
}

#[cfg(test)]
mod tests {
    use crate::{sample, Generator};

    #[test]
    fn residents_follow_skeleton_order_and_expand_to_the_same_person() {
        let (seed, settlement, buildings) = sample::market_town();
        let g = Generator::new(seed, &settlement, &buildings).unwrap();
        let all: Vec<_> = (0..g.len()).map(|p| g.resident(p).unwrap()).collect();
        assert!(g.resident(g.len()).is_none());
        assert!(all
            .windows(2)
            .all(|w| (w[0].home, w[0].index) < (w[1].home, w[1].index)));
        let ids: Vec<_> = g.ids().collect();
        assert_eq!(all.iter().map(|r| r.id).collect::<Vec<_>>(), ids);
        for r in all.iter().step_by(97) {
            let range = g.residents_of(r.home).unwrap();
            let at = range.start + usize::try_from(r.index).unwrap();
            assert_eq!(g.resident(at), Some(*r));
            let npc = g.npc_at(at).unwrap();
            assert_eq!(npc, g.commoner(r.home, r.index).unwrap());
            assert_eq!(npc.job.key, r.job);
            assert_eq!(npc.workplace_building, r.workplace);
            assert_eq!(npc.home_building, r.home);
        }
        assert!(g.npc_at(g.len()).is_err());
        assert!(!g.has_building(crate::BuildingId(u64::MAX)));
    }
}
