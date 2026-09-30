//! The generated population of one settlement and its queries.

use serde::{Deserialize, Serialize};

use crate::input::{BuildingId, SettlementId};
use crate::npc::{HouseholdId, JobCategory, Npc, NpcId, RelationKind};

/// One family tie inside a household: `to` is `from`'s `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseholdRelation {
    /// The person the relation belongs to.
    pub from: NpcId,
    /// What `to` is to `from`.
    pub kind: RelationKind,
    /// The other person.
    pub to: NpcId,
}

/// People sharing a home.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Household {
    /// Household id.
    pub id: HouseholdId,
    /// Home building.
    pub home: BuildingId,
    /// Family name of the head.
    pub family_name: String,
    /// Members, head first.
    pub members: Vec<NpcId>,
    /// Spouse, parent/child, sibling and lodger ties, both directions.
    pub relations: Vec<HouseholdRelation>,
}

/// Compact index entry for one inhabitant (about 40 bytes), so queries work
/// without expanding anyone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterEntry {
    /// Person.
    pub id: NpcId,
    /// Index into [`Population::job_keys`].
    pub job: u16,
    /// Workplace, if any.
    pub workplace: Option<BuildingId>,
    /// Whether the person is a stored notable.
    pub notable: bool,
}

/// The population of one settlement. Full [`Npc`] records are stored for
/// notables only; any commoner is regenerated on demand with [`crate::npc`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Population {
    /// Settlement.
    pub settlement_id: SettlementId,
    /// Total inhabitants.
    pub population: u32,
    /// Every household.
    pub households: Vec<Household>,
    /// Stored notables, full records.
    pub npcs: Vec<Npc>,
    /// Inhabitants who are not stored.
    pub commoner_count: u32,
    /// Job keys referenced by the roster.
    pub job_keys: Vec<String>,
    /// Category of each job key, same order.
    pub job_categories: Vec<JobCategory>,
    /// One compact entry per inhabitant, in skeleton order (home building
    /// id, then resident index).
    pub roster: Vec<RosterEntry>,
}

impl Population {
    /// Everyone living in a building.
    #[must_use]
    pub fn residents(&self, building: BuildingId) -> Vec<NpcId> {
        self.households
            .iter()
            .filter(|h| h.home == building)
            .flat_map(|h| h.members.iter().copied())
            .collect()
    }

    /// Everyone working at a building.
    #[must_use]
    pub fn workers(&self, building: BuildingId) -> Vec<NpcId> {
        self.roster
            .iter()
            .filter(|r| r.workplace == Some(building))
            .map(|r| r.id)
            .collect()
    }

    /// Everyone with a job key such as `smith` or `farmer`.
    #[must_use]
    pub fn by_job(&self, job: &str) -> Vec<NpcId> {
        let Some(index) = self.job_keys.iter().position(|k| k == job) else {
            return Vec::new();
        };
        self.roster
            .iter()
            .filter(|r| usize::from(r.job) == index)
            .map(|r| r.id)
            .collect()
    }

    /// The job key of a person.
    #[must_use]
    pub fn job_of(&self, id: NpcId) -> Option<&str> {
        let entry = self.roster.iter().find(|r| r.id == id)?;
        self.job_keys
            .get(usize::from(entry.job))
            .map(String::as_str)
    }

    /// Stored notables.
    #[must_use]
    pub fn notables(&self) -> &[Npc] {
        &self.npcs
    }

    /// A stored notable by id.
    #[must_use]
    pub fn notable(&self, id: NpcId) -> Option<&Npc> {
        self.npcs.iter().find(|n| n.id == id)
    }

    /// Head count per job category.
    #[must_use]
    pub fn category_counts(&self) -> Vec<(JobCategory, u32)> {
        let mut out: Vec<(JobCategory, u32)> = Vec::new();
        for r in &self.roster {
            let Some(&category) = self.job_categories.get(usize::from(r.job)) else {
                continue;
            };
            match out.iter_mut().find(|(c, _)| *c == category) {
                Some((_, n)) => *n += 1,
                None => out.push((category, 1)),
            }
        }
        out.sort();
        out
    }

    /// Head count per job key, in first-seen order.
    #[must_use]
    pub fn job_counts(&self) -> Vec<(String, u32)> {
        let mut counts = vec![0u32; self.job_keys.len()];
        for r in &self.roster {
            if let Some(c) = counts.get_mut(usize::from(r.job)) {
                *c += 1;
            }
        }
        self.job_keys
            .iter()
            .cloned()
            .zip(counts)
            .filter(|&(_, c)| c > 0)
            .collect()
    }
}
