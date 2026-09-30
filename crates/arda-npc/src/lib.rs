//! Deterministic NPC populations for Arda settlements: every inhabitant has a
//! name, ancestry, age, home, job tied to a building, personality,
//! relationships and a full SRD 5.1 sheet.
//!
//! The crate is a pure library driven by [`SettlementProfile`] and
//! [`BuildingSpec`] descriptions. Notables are stored in the returned
//! [`Population`]; everyone else is regenerated on demand by [`npc`] and is
//! byte-for-byte the same person every time. See the crate README for the
//! regeneration scheme and `docs/capstone/logic/06-society-generation.md`
//! (steps 5–6) for the design.
//!
//! SRD 5.1 material is used under CC-BY-4.0; see `data/srd/SOURCE.md`.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub(crate) mod data;
pub mod error;
pub mod input;
mod names;
pub mod npc;
mod personality;
mod plan;
pub mod population;
pub mod rng;
pub mod rules;
pub mod sample;
pub mod sheet;
pub mod text;

pub use error::NpcError;
pub use input::{
    Biome, BuildingFunction, BuildingId, BuildingSpec, Craft, NotableSlot, RealmId,
    SettlementFunction, SettlementId, SettlementProfile, Tier,
};
pub use npc::{
    HouseholdId, Ideal, Job, JobCategory, Lifestyle, Name, Npc, NpcId, Personality, RelationKind,
    Relationship, Sex, SocialRank,
};
pub use population::{Household, HouseholdRelation, Population, RosterEntry};
pub use sheet::Sheet;

use plan::Plan;

/// A settlement's population skeleton, built once and expanded person by
/// person. Use it to materialise many commoners without rebuilding.
pub struct Generator<'a> {
    plan: Plan<'a>,
}

impl<'a> Generator<'a> {
    /// Builds the skeleton for a settlement.
    ///
    /// # Errors
    /// See [`NpcError`]: foreign or duplicate buildings, too little housing,
    /// or invalid bundled data.
    pub fn new(
        world_seed: u64,
        settlement: &'a SettlementProfile,
        buildings: &'a [BuildingSpec],
    ) -> Result<Self, NpcError> {
        Self::with_notables(world_seed, settlement, buildings, &[])
    }

    /// Builds the skeleton and binds the society layer's notable slots
    /// (logic/14 §soc-offices): each slot's holder is a stored notable
    /// with the slot's title, name and sex.
    ///
    /// # Errors
    /// As [`Generator::new`].
    pub fn with_notables(
        world_seed: u64,
        settlement: &'a SettlementProfile,
        buildings: &'a [BuildingSpec],
        notables: &'a [NotableSlot],
    ) -> Result<Self, NpcError> {
        Ok(Self {
            plan: Plan::build(world_seed, settlement, buildings, notables)?,
        })
    }

    /// Number of inhabitants.
    #[must_use]
    pub fn len(&self) -> usize {
        self.plan.people.len()
    }

    /// Whether the settlement is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.plan.people.is_empty()
    }

    /// Every inhabitant's id, in skeleton order (home building id, then index).
    pub fn ids(&self) -> impl Iterator<Item = NpcId> + '_ {
        (0..self.plan.people.len()).map(|p| self.plan.id_of(p))
    }

    /// Ids of the notables.
    pub fn notable_ids(&self) -> impl Iterator<Item = NpcId> + '_ {
        (0..self.plan.people.len())
            .filter(|&p| self.plan.people[p].notable)
            .map(|p| self.plan.id_of(p))
    }

    /// Expands one inhabitant.
    ///
    /// # Errors
    /// [`NpcError::UnknownNpc`] when the id is not an inhabitant.
    pub fn npc(&self, id: NpcId) -> Result<Npc, NpcError> {
        self.plan.npc(self.plan.index_of(id)?)
    }

    /// The home building and resident index of an inhabitant's id.
    #[must_use]
    pub fn locate(&self, id: NpcId) -> Option<(BuildingId, u32)> {
        let person = self.plan.index_of(id).ok()?;
        let p = &self.plan.people[person];
        Some((self.plan.buildings[p.building].id, p.index))
    }

    /// Expands the `index`-th resident of `building`.
    ///
    /// # Errors
    /// [`NpcError::UnknownNpc`] when there is no such resident.
    pub fn commoner(&self, building: BuildingId, index: u32) -> Result<Npc, NpcError> {
        let id = NpcId::from_parts(self.plan.settlement.id, building, index);
        let person = self
            .plan
            .resident(building, index)
            .ok_or(NpcError::UnknownNpc(id))?;
        self.plan.npc(person)
    }

    /// The population: households, the roster and the stored notables.
    ///
    /// # Errors
    /// [`NpcError::Data`] if bundled data is inconsistent.
    pub fn population(&self) -> Result<Population, NpcError> {
        let plan = &self.plan;
        let mut job_keys: Vec<String> = Vec::new();
        let mut job_categories = Vec::new();
        let mut roster = Vec::with_capacity(plan.people.len());
        for (p, person) in plan.people.iter().enumerate() {
            let job = match job_keys.iter().position(|k| k == person.job) {
                Some(i) => i,
                None => {
                    job_keys.push(person.job.to_string());
                    let category = &plan.data.job(person.job)?.category;
                    job_categories.push(
                        JobCategory::from_key(category)
                            .ok_or_else(|| NpcError::Data(format!("category {category}")))?,
                    );
                    job_keys.len() - 1
                }
            };
            roster.push(RosterEntry {
                id: plan.id_of(p),
                job: u16::try_from(job).map_err(|_| NpcError::Data("too many job kinds".into()))?,
                workplace: person.workplace.map(|b| plan.buildings[b].id),
                notable: person.notable,
            });
        }
        let mut households = Vec::with_capacity(plan.houses.len());
        for house in &plan.houses {
            let family_name = match house.members.first() {
                Some(&head) => plan.name_of(head)?.family,
                None => String::new(),
            };
            households.push(Household {
                id: HouseholdId {
                    building: plan.buildings[house.building].id,
                    index: house.index,
                },
                home: plan.buildings[house.building].id,
                family_name,
                members: house.members.iter().map(|&m| plan.id_of(m)).collect(),
                relations: house
                    .relations
                    .iter()
                    .map(|&(a, kind, b)| HouseholdRelation {
                        from: plan.id_of(a),
                        kind,
                        to: plan.id_of(b),
                    })
                    .collect(),
            });
        }
        let npcs = (0..plan.people.len())
            .filter(|&p| plan.people[p].notable)
            .map(|p| plan.npc(p))
            .collect::<Result<Vec<_>, _>>()?;
        let total = u32::try_from(plan.people.len()).unwrap_or(u32::MAX);
        let stored = u32::try_from(npcs.len()).unwrap_or(u32::MAX);
        Ok(Population {
            settlement_id: plan.settlement.id,
            population: total,
            households,
            npcs,
            commoner_count: total - stored,
            job_keys,
            job_categories,
            roster,
        })
    }
}

/// Generates the population of a settlement.
///
/// # Errors
/// See [`Generator::new`].
pub fn generate_population(
    world_seed: u64,
    settlement: &SettlementProfile,
    buildings: &[BuildingSpec],
) -> Result<Population, NpcError> {
    Generator::new(world_seed, settlement, buildings)?.population()
}

/// Regenerates any inhabitant, notable or commoner, identically.
///
/// # Errors
/// See [`Generator::new`] and [`Generator::npc`].
pub fn npc(
    world_seed: u64,
    settlement: &SettlementProfile,
    buildings: &[BuildingSpec],
    id: NpcId,
) -> Result<Npc, NpcError> {
    Generator::new(world_seed, settlement, buildings)?.npc(id)
}

/// Regenerates the `index`-th resident of `building`.
///
/// # Errors
/// See [`npc`].
pub fn commoner(
    world_seed: u64,
    settlement: &SettlementProfile,
    buildings: &[BuildingSpec],
    building: BuildingId,
    index: u32,
) -> Result<Npc, NpcError> {
    Generator::new(world_seed, settlement, buildings)?.commoner(building, index)
}
